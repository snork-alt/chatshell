mod config;
mod pty;
mod terminal;
mod hooks;
mod window;
mod llm;

use anyhow::{Context, Result};
use clap::{Arg, Command};
use crossterm::event::Event;
use futures::stream::StreamExt;
use nix::sys::signal::Signal;
use std::io::{Read, Write};
use std::os::unix::io::{AsRawFd, FromRawFd};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::select;
use tokio::sync::Mutex;

use config::Config;
use hooks::{HookManager, create_default_hooks};
use llm::{LlmService, LlmConfig};
use pty::PtySession;
use terminal::{Terminal, KeyInput};

#[derive(Debug)]
struct ChatShell {
    config: Config,
    terminal: Terminal,
    pty: PtySession,
    hook_manager: HookManager,
    llm_service: Option<Arc<Mutex<LlmService>>>,
    running: Arc<AtomicBool>,
}

impl ChatShell {
    pub async fn new(config_path: Option<String>) -> Result<Self> {
        // Load or create configuration
        let config_path = if let Some(path) = config_path {
            path
        } else {
            Config::ensure_config_exists()?
        };

        let config = Config::load_from_file(&config_path)
            .with_context(|| format!("Failed to load config from {}", config_path))?;

        // Initialize terminal
        let mut terminal = Terminal::new()
            .with_context(|| "Failed to initialize terminal")?;

        // Enable raw mode to capture all keystrokes
        terminal.enter_raw_mode()
            .with_context(|| "Failed to enter raw mode")?;

        // Spawn shell process
        let pty = PtySession::spawn(&config.shell)
            .with_context(|| "Failed to spawn shell process")?;

        // Set up signal handling
        let running = Arc::new(AtomicBool::new(true));
        Self::setup_signal_handlers(running.clone())?;

        // Initialize hook manager
        let mut hook_manager = HookManager::from_configs(config.hooks.clone());

        // Initialize LLM service if API key is available
        let llm_service = if !config.llm.api_key.is_empty() {
            match LlmService::new(config.llm.clone()) {
                Ok(service) => {
                    let service = Arc::new(Mutex::new(service));
                    hook_manager.set_llm_service(service.clone());
                    Some(service)
                }
                Err(e) => {
                    eprintln!("Warning: Failed to initialize LLM service: {}", e);
                    eprintln!("LLM features will be disabled. Please check your configuration.");
                    None
                }
            }
        } else {
            eprintln!("Warning: OpenAI API key not found. LLM features will be disabled.");
            eprintln!("Set OPENAI_API_KEY environment variable or configure it in the config file.");
            None
        };

        // Resize PTY to match terminal size
        let (cols, rows) = terminal.size()?;
        pty.resize_pty(rows, cols)?;

        Ok(ChatShell {
            config,
            terminal,
            pty,
            hook_manager,
            llm_service,
            running,
        })
    }

    fn setup_signal_handlers(running: Arc<AtomicBool>) -> Result<()> {
        let running_clone = running.clone();
        
        // Handle SIGINT (Ctrl+C) and SIGTERM gracefully
        let mut signals = signal_hook_tokio::Signals::new(&[
            signal_hook::consts::SIGINT,
            signal_hook::consts::SIGTERM,
            signal_hook::consts::SIGWINCH, // Window resize
        ])?;

        tokio::spawn(async move {
            while let Some(signal) = signals.next().await {
                match signal {
                    signal_hook::consts::SIGINT | signal_hook::consts::SIGTERM => {
                        running_clone.store(false, Ordering::Relaxed);
                        break;
                    }
                    signal_hook::consts::SIGWINCH => {
                        // Handle window resize - this would need to be communicated
                        // to the main loop to resize the PTY
                    }
                    _ => {}
                }
            }
        });

        Ok(())
    }

    pub async fn run(&mut self) -> Result<()> {
        println!("ChatShell started. Press Ctrl+; for help.");
        if self.llm_service.is_some() {
            println!("LLM Assistant enabled. Press Ctrl+Shift+L to open prompt.");
        }
        
        // Create channels for communication between tasks
        let (input_tx, mut input_rx) = tokio::sync::mpsc::unbounded_channel::<Vec<u8>>();
        let (output_tx, mut output_rx) = tokio::sync::mpsc::unbounded_channel::<Vec<u8>>();

        // Task to read from shell and send to terminal
        let pty_fd = self.pty.master.as_raw_fd();
        let output_tx_clone = output_tx.clone();
        let running_clone = self.running.clone();
        
        tokio::spawn(async move {
            let mut buffer = [0u8; 4096];
            loop {
                if !running_clone.load(Ordering::Relaxed) {
                    break;
                }

                // Use blocking read with non-blocking fd
                let mut file = unsafe { std::fs::File::from_raw_fd(pty_fd) };
                match file.read(&mut buffer) {
                    Ok(n) if n > 0 => {
                        if output_tx_clone.send(buffer[..n].to_vec()).is_err() {
                            break;
                        }
                    }
                    Ok(_) => {
                        // EOF - shell process ended
                        running_clone.store(false, Ordering::Relaxed);
                        break;
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        // No data available, continue
                    }
                    Err(_) => {
                        // Read error
                        running_clone.store(false, Ordering::Relaxed);
                        break;
                    }
                }
                std::mem::forget(file); // Don't close the fd
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        });

        // Task to write to shell from input queue
        let pty_fd_write = self.pty.master.as_raw_fd();
        let running_clone = self.running.clone();
        
        tokio::spawn(async move {
            while let Some(data) = input_rx.recv().await {
                if !running_clone.load(Ordering::Relaxed) {
                    break;
                }
                
                let mut file = unsafe { std::fs::File::from_raw_fd(pty_fd_write) };
                if let Err(_) = file.write_all(&data) {
                    running_clone.store(false, Ordering::Relaxed);
                    break;
                }
                std::mem::forget(file); // Don't close the fd
            }
        });

        // Main event loop
        while self.running.load(Ordering::Relaxed) {
            select! {
                // Handle terminal input
                _ = self.handle_terminal_input(&input_tx) => {
                    // Error handling is done inside the function
                }
                
                // Handle shell output
                output_data = output_rx.recv() => {
                    if let Some(data) = output_data {
                        if let Err(e) = self.terminal.write(&data) {
                            eprintln!("Failed to write to terminal: {}", e);
                            break;
                        }
                    }
                }
                
                // Small delay to prevent busy waiting
                _ = tokio::time::sleep(Duration::from_millis(1)) => {}
            }
        }

        self.cleanup().await?;
        Ok(())
    }

    async fn handle_terminal_input(&mut self, input_tx: &tokio::sync::mpsc::UnboundedSender<Vec<u8>>) -> Result<()> {
        // Check for terminal events with a short timeout
        if self.terminal.poll_event(Duration::from_millis(10))? {
            match self.terminal.read_event()? {
                Event::Key(key_event) => {
                    let key_input = KeyInput::from_event(key_event);
                    
                    // Check if any hook should handle this key
                    match self.hook_manager.process_key(&key_input).await {
                        Ok(true) => {
                            // Hook consumed the key, don't forward to shell
                            return Ok(());
                        }
                        Ok(false) => {
                            // No hook consumed the key, forward to shell
                        }
                        Err(e) => {
                            eprintln!("Hook processing error: {}", e);
                            // Continue and forward to shell
                        }
                    }

                    // Forward key to shell
                    if !key_input.raw_bytes.is_empty() {
                        input_tx.send(key_input.raw_bytes)?;
                    }
                }
                Event::Resize(cols, rows) => {
                    // Resize PTY to match new terminal size
                    if let Err(e) = self.pty.resize_pty(rows, cols) {
                        eprintln!("Failed to resize PTY: {}", e);
                    }
                }
                _ => {
                    // Ignore other events (mouse, etc.)
                }
            }
        }
        Ok(())
    }

    async fn cleanup(&mut self) -> Result<()> {
        // Signal the shell to terminate gracefully
        if self.pty.is_child_alive() {
            let _ = self.pty.send_signal(Signal::SIGTERM);
            
            // Give it a moment to terminate
            tokio::time::sleep(Duration::from_millis(100)).await;
            
            // Force kill if still alive
            if self.pty.is_child_alive() {
                let _ = self.pty.send_signal(Signal::SIGKILL);
            }
        }

        // Restore terminal state
        self.terminal.leave_raw_mode()?;
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let matches = Command::new("chatshell")
        .version("0.1.0")
        .about("A transparent shell wrapper with hooks and LLM integration")
        .arg(
            Arg::new("config")
                .short('c')
                .long("config")
                .value_name("FILE")
                .help("Configuration file path")
        )
        .arg(
            Arg::new("shell")
                .short('s')
                .long("shell")
                .value_name("SHELL")
                .help("Shell command to run (overrides config)")
        )
        .arg(
            Arg::new("create-config")
                .long("create-config")
                .help("Create a default configuration file and exit")
                .action(clap::ArgAction::SetTrue)
        )
        .get_matches();

    // Handle create-config option
    if matches.get_flag("create-config") {
        let config_path = Config::ensure_config_exists()?;
        
        // Also create a config with default hooks
        let mut config = Config::default();
        config.hooks = create_default_hooks();
        config.save_to_file(&config_path)?;
        
        println!("Created configuration file at: {}", config_path);
        println!("Edit this file to customize your shell, hooks, and LLM settings.");
        println!("Set OPENAI_API_KEY environment variable to enable LLM features.");
        return Ok(());
    }

    // Create and run ChatShell
    let config_path = matches.get_one::<String>("config").map(|s| s.clone());
    let mut chatshell = ChatShell::new(config_path).await?;
    
    chatshell.run().await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_config_creation() {
        let result = Config::ensure_config_exists();
        assert!(result.is_ok());
    }
} 