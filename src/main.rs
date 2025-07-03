mod config;
mod pty;
mod terminal;
mod hooks;

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

use config::Config;
use hooks::{HookManager, create_default_hooks};
use pty::PtySession;
use terminal::{Terminal, KeyInput};

#[derive(Debug)]
struct ChatShell {
    config: Config,
    terminal: Terminal,
    pty: PtySession,
    hook_manager: HookManager,
    running: Arc<AtomicBool>,
}

impl ChatShell {
    pub async fn new(config_path: Option<String>) -> Result<Self> {
        eprintln!("[DEBUG] Loading config...");
        // Load or create configuration
        let config_path = if let Some(path) = config_path {
            path
        } else {
            Config::ensure_config_exists()?
        };

        let config = Config::load_from_file(&config_path)
            .with_context(|| format!("Failed to load config from {}", config_path))?;

        eprintln!("[DEBUG] Config loaded, shell: {}", config.shell.command);

        // Initialize terminal
        eprintln!("[DEBUG] Initializing terminal...");
        let mut terminal = Terminal::new()
            .with_context(|| "Failed to initialize terminal")?;

        // Enable raw mode to capture all keystrokes
        eprintln!("[DEBUG] Entering raw mode...");
        terminal.enter_raw_mode()
            .with_context(|| "Failed to enter raw mode")?;

        // Spawn shell process
        eprintln!("[DEBUG] Spawning shell process...");
        let pty = PtySession::spawn(&config.shell)
            .with_context(|| "Failed to spawn shell process")?;

        eprintln!("[DEBUG] Shell process spawned successfully");

        // Set up signal handling
        eprintln!("[DEBUG] Setting up signal handlers...");
        let running = Arc::new(AtomicBool::new(true));
        Self::setup_signal_handlers(running.clone())?;

        // Initialize hook manager
        eprintln!("[DEBUG] Initializing hook manager...");
        let hook_manager = HookManager::from_configs(config.hooks.clone());

        // Resize PTY to match terminal size
        eprintln!("[DEBUG] Resizing PTY...");
        let (cols, rows) = terminal.size()?;
        pty.resize_pty(rows, cols)?;

        eprintln!("[DEBUG] ChatShell initialization complete");

        Ok(ChatShell {
            config,
            terminal,
            pty,
            hook_manager,
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
        // Show brief welcome message
        let shell_name = self.config.shell.command.split('/').last().unwrap_or("shell");
        eprintln!("\x1b[90m[ChatShell wrapping {}]\x1b[0m", shell_name);
        
        // Wait a moment for shell to initialize and display initial prompt
        tokio::time::sleep(Duration::from_millis(200)).await;
        
        // Simple event loop - just forward data between terminal and shell
        let mut shell_buffer = [0u8; 4096];
        
        loop {
            // Check if child process is still alive
            if !self.pty.is_child_alive() {
                break;
            }
            
            // Handle terminal input (non-blocking)
            if self.terminal.poll_event(Duration::from_millis(1))? {
                match self.terminal.read_event()? {
                    Event::Key(key_event) => {
                        let key_input = KeyInput::from_event(key_event);
                        
                        // Check if any hook should handle this key
                        if let Ok(true) = self.hook_manager.process_key(&key_input) {
                            // Hook consumed the key, don't forward to shell
                            continue;
                        }

                        // Forward key to shell
                        if !key_input.raw_bytes.is_empty() {
                            use std::os::unix::io::AsRawFd;
                            use nix::unistd::write;
                            let _ = write(self.pty.master.as_raw_fd(), &key_input.raw_bytes);
                        }
                    }
                    Event::Resize(cols, rows) => {
                        let _ = self.pty.resize_pty(rows, cols);
                    }
                    _ => {}
                }
            }
            
            // Handle shell output (non-blocking)
            use std::os::unix::io::AsRawFd;
            use nix::unistd::read;
            match read(self.pty.master.as_raw_fd(), &mut shell_buffer) {
                Ok(n) if n > 0 => {
                    let _ = self.terminal.write(&shell_buffer[..n]);
                }
                Ok(_) => break, // EOF
                Err(nix::errno::Errno::EAGAIN) => {
                    // No data available, continue
                }
                Err(_) => break, // Error
            }
            
            // Small sleep to prevent busy waiting
            tokio::time::sleep(Duration::from_millis(1)).await;
        }

        self.cleanup().await?;
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
        .about("A transparent shell wrapper with hooks and plugins")
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
        .arg(
            Arg::new("test-init")
                .long("test-init")
                .help("Test initialization only and exit")
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
        println!("Edit this file to customize your shell and hooks.");
        return Ok(());
    }

    // Create and run ChatShell
    let config_path = matches.get_one::<String>("config").cloned();
    let mut shell = ChatShell::new(config_path).await?;

    // Override shell if specified in command line
    if let Some(shell_cmd) = matches.get_one::<String>("shell") {
        shell.config.shell.command = shell_cmd.clone();
        shell.config.shell.args = vec!["-i".to_string()]; // Interactive mode
    }

    // Handle test-init option
    if matches.get_flag("test-init") {
        eprintln!("[DEBUG] Test initialization completed successfully");
        shell.cleanup().await?;
        return Ok(());
    }

    // Run the shell wrapper
    match shell.run().await {
        Ok(()) => Ok(()),
        Err(e) => {
            eprintln!("ChatShell error: {}", e);
            std::process::exit(1);
        }
    }
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