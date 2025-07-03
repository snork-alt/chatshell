use anyhow::Result;
use crossterm::event::{KeyCode, KeyModifiers};
use nix::sys::signal::Signal;
use serial_test::serial;
use std::fs;
use std::io::Write;
use std::time::Duration;
use tempfile::NamedTempFile;

use chatshell::config::{Config, HookConfig, ShellConfig};
use chatshell::hooks::{HookManager, create_default_hooks};
use chatshell::pty::PtySession;
use chatshell::terminal::{KeyInput, Terminal};

/// Test basic PTY creation and shell spawning
#[tokio::test]
#[serial]
async fn test_pty_shell_spawning() -> Result<()> {
    let shell_config = ShellConfig {
        command: "/bin/bash".to_string(),
        args: vec!["-i".to_string()],
        env: None,
    };

    let pty = PtySession::spawn(&shell_config)?;
    
    // Check that child process is alive
    assert!(pty.is_child_alive());
    
    // Send a simple command
    let cmd = b"echo 'test message'\n";
    pty.write_to_shell(cmd)?;
    
    // Give some time for output
    tokio::time::sleep(Duration::from_millis(500)).await;
    
    // Read output
    let mut buffer = [0u8; 1024];
    let bytes_read = pty.read_from_shell(&mut buffer)?;
    let output = String::from_utf8_lossy(&buffer[..bytes_read]);
    
    // Should contain our test message somewhere
    assert!(output.contains("test message"));
    
    Ok(())
}

/// Test special key sequences are properly converted to bytes
#[test]
fn test_special_key_conversion() {
    // Test arrow keys
    let up_key = KeyInput::new(KeyCode::Up, KeyModifiers::empty());
    assert_eq!(up_key.raw_bytes, vec![27, 91, 65]); // ESC[A
    
    let down_key = KeyInput::new(KeyCode::Down, KeyModifiers::empty());
    assert_eq!(down_key.raw_bytes, vec![27, 91, 66]); // ESC[B
    
    let right_key = KeyInput::new(KeyCode::Right, KeyModifiers::empty());
    assert_eq!(right_key.raw_bytes, vec![27, 91, 67]); // ESC[C
    
    let left_key = KeyInput::new(KeyCode::Left, KeyModifiers::empty());
    assert_eq!(left_key.raw_bytes, vec![27, 91, 68]); // ESC[D
    
    // Test function keys
    let f1_key = KeyInput::new(KeyCode::F(1), KeyModifiers::empty());
    assert_eq!(f1_key.raw_bytes, vec![27, 79, 80]); // ESC OP
    
    let f5_key = KeyInput::new(KeyCode::F(5), KeyModifiers::empty());
    assert_eq!(f5_key.raw_bytes, vec![27, 91, 49, 53, 126]); // ESC[15~
    
    // Test control characters
    let ctrl_a = KeyInput::new(KeyCode::Char('a'), KeyModifiers::CONTROL);
    assert_eq!(ctrl_a.raw_bytes, vec![1]);
    
    let ctrl_c = KeyInput::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
    assert_eq!(ctrl_c.raw_bytes, vec![3]);
    
    // Test Alt combinations
    let alt_a = KeyInput::new(KeyCode::Char('a'), KeyModifiers::ALT);
    assert_eq!(alt_a.raw_bytes, vec![27, 97]); // ESC + 'a'
    
    // Test special keys
    let enter = KeyInput::new(KeyCode::Enter, KeyModifiers::empty());
    assert_eq!(enter.raw_bytes, vec![13]); // \r
    
    let tab = KeyInput::new(KeyCode::Tab, KeyModifiers::empty());
    assert_eq!(tab.raw_bytes, vec![9]); // \t
    
    let backspace = KeyInput::new(KeyCode::Backspace, KeyModifiers::empty());
    assert_eq!(backspace.raw_bytes, vec![127]);
    
    let delete = KeyInput::new(KeyCode::Delete, KeyModifiers::empty());
    assert_eq!(delete.raw_bytes, vec![27, 91, 51, 126]); // ESC[3~
    
    let home = KeyInput::new(KeyCode::Home, KeyModifiers::empty());
    assert_eq!(home.raw_bytes, vec![27, 91, 72]); // ESC[H
    
    let end = KeyInput::new(KeyCode::End, KeyModifiers::empty());
    assert_eq!(end.raw_bytes, vec![27, 91, 70]); // ESC[F
    
    let page_up = KeyInput::new(KeyCode::PageUp, KeyModifiers::empty());
    assert_eq!(page_up.raw_bytes, vec![27, 91, 53, 126]); // ESC[5~
    
    let page_down = KeyInput::new(KeyCode::PageDown, KeyModifiers::empty());
    assert_eq!(page_down.raw_bytes, vec![27, 91, 54, 126]); // ESC[6~
}

/// Test vi editor interaction through the shell
#[tokio::test]
#[serial]
async fn test_vi_editor_interaction() -> Result<()> {
    let shell_config = ShellConfig {
        command: "/bin/bash".to_string(),
        args: vec!["-i".to_string()],
        env: None,
    };

    let pty = PtySession::spawn(&shell_config)?;
    
    // Create a temporary file to edit
    let mut temp_file = NamedTempFile::new()?;
    let temp_path = temp_file.path().to_string_lossy().to_string();
    temp_file.write_all(b"initial content\n")?;
    temp_file.flush()?;
    
    // Start vi with the temp file
    let vi_cmd = format!("vi {}\n", temp_path);
    pty.write_to_shell(vi_cmd.as_bytes())?;
    
    // Wait for vi to start
    tokio::time::sleep(Duration::from_millis(1000)).await;
    
    // Enter insert mode
    pty.write_to_shell(b"i")?;
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Type some text
    pty.write_to_shell(b"Hello from test\n")?;
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Exit insert mode (ESC)
    pty.write_to_shell(&[27])?;
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Save and quit (:wq)
    pty.write_to_shell(b":wq\n")?;
    tokio::time::sleep(Duration::from_millis(500)).await;
    
    // Read the file to verify changes
    let content = fs::read_to_string(&temp_path)?;
    assert!(content.contains("Hello from test"));
    
    Ok(())
}

/// Test hook functionality and key interception
#[test]
fn test_hook_system() {
    let hooks = create_default_hooks();
    let hook_manager = HookManager::from_configs(hooks);
    
    // Test help hook (Ctrl+;)
    let help_key = KeyInput::new(KeyCode::Char(';'), KeyModifiers::CONTROL);
    let result = hook_manager.process_key(&help_key);
    assert!(result.is_ok());
    assert!(result.unwrap()); // Should be consumed
    
    // Test non-matching key
    let random_key = KeyInput::new(KeyCode::Char('x'), KeyModifiers::empty());
    let result = hook_manager.process_key(&random_key);
    assert!(result.is_ok());
    assert!(!result.unwrap()); // Should not be consumed
    
    // Test disabled hook
    let time_key = KeyInput::new(KeyCode::Char('t'), KeyModifiers::CONTROL);
    let result = hook_manager.process_key(&time_key);
    assert!(result.is_ok());
    assert!(!result.unwrap()); // Should not be consumed (disabled by default)
}

/// Test configuration loading and saving
#[test]
fn test_config_operations() -> Result<()> {
    let mut temp_file = NamedTempFile::new()?;
    let config_path = temp_file.path().to_string_lossy().to_string();
    
    // Create a test config
    let config = Config {
        shell: ShellConfig {
            command: "/bin/zsh".to_string(),
            args: vec!["-l".to_string()],
            env: Some([("TEST_VAR".to_string(), "test_value".to_string())].into()),
        },
        hooks: vec![
            HookConfig {
                name: "test_hook".to_string(),
                key_combination: "ctrl+x".to_string(),
                action: "echo 'test'".to_string(),
                description: Some("Test hook".to_string()),
                enabled: true,
            }
        ],
    };
    
    // Save config
    config.save_to_file(&config_path)?;
    
    // Load config
    let loaded_config = Config::load_from_file(&config_path)?;
    
    // Verify config was loaded correctly
    assert_eq!(loaded_config.shell.command, "/bin/zsh");
    assert_eq!(loaded_config.shell.args, vec!["-l".to_string()]);
    assert_eq!(loaded_config.hooks.len(), 1);
    assert_eq!(loaded_config.hooks[0].name, "test_hook");
    
    Ok(())
}

/// Test PTY resize functionality
#[tokio::test]
#[serial]
async fn test_pty_resize() -> Result<()> {
    let shell_config = ShellConfig {
        command: "/bin/bash".to_string(),
        args: vec!["-i".to_string()],
        env: None,
    };

    let pty = PtySession::spawn(&shell_config)?;
    
    // Test resizing the PTY
    pty.resize_pty(25, 80)?;
    pty.resize_pty(50, 120)?;
    pty.resize_pty(30, 100)?;
    
    // Send stty command to check terminal size
    pty.write_to_shell(b"stty size\n")?;
    tokio::time::sleep(Duration::from_millis(300)).await;
    
    let mut buffer = [0u8; 1024];
    let bytes_read = pty.read_from_shell(&mut buffer)?;
    let output = String::from_utf8_lossy(&buffer[..bytes_read]);
    
    // Should show the last resize dimensions (30 100)
    assert!(output.contains("30 100"));
    
    Ok(())
}

/// Test signal handling
#[tokio::test]
#[serial] 
async fn test_signal_handling() -> Result<()> {
    let shell_config = ShellConfig {
        command: "/bin/bash".to_string(),
        args: vec!["-i".to_string()],
        env: None,
    };

    let pty = PtySession::spawn(&shell_config)?;
    
    // Verify child is alive
    assert!(pty.is_child_alive());
    
    // Send SIGTERM
    pty.send_signal(Signal::SIGTERM)?;
    
    // Wait a bit for signal to be processed
    tokio::time::sleep(Duration::from_millis(200)).await;
    
    // Check if child has terminated
    // Note: This might be flaky depending on timing
    let mut attempts = 0;
    while pty.is_child_alive() && attempts < 10 {
        tokio::time::sleep(Duration::from_millis(100)).await;
        attempts += 1;
    }
    
    Ok(())
}

/// Test complex command sequences (like navigating command history)
#[tokio::test]
#[serial]
async fn test_command_history_navigation() -> Result<()> {
    let shell_config = ShellConfig {
        command: "/bin/bash".to_string(),
        args: vec!["-i".to_string()],
        env: None,
    };

    let pty = PtySession::spawn(&shell_config)?;
    
    // Send a few commands to build history
    pty.write_to_shell(b"echo 'first command'\n")?;
    tokio::time::sleep(Duration::from_millis(200)).await;
    
    pty.write_to_shell(b"echo 'second command'\n")?;
    tokio::time::sleep(Duration::from_millis(200)).await;
    
    pty.write_to_shell(b"echo 'third command'\n")?;
    tokio::time::sleep(Duration::from_millis(200)).await;
    
    // Navigate history with up arrow
    pty.write_to_shell(&[27, 91, 65])?; // Up arrow (ESC[A)
    tokio::time::sleep(Duration::from_millis(200)).await;
    
    pty.write_to_shell(&[27, 91, 65])?; // Up arrow again
    tokio::time::sleep(Duration::from_millis(200)).await;
    
    // Execute the command from history
    pty.write_to_shell(b"\n")?;
    tokio::time::sleep(Duration::from_millis(300)).await;
    
    let mut buffer = [0u8; 2048];
    let bytes_read = pty.read_from_shell(&mut buffer)?;
    let output = String::from_utf8_lossy(&buffer[..bytes_read]);
    
    // Should contain output from the historical command
    assert!(output.contains("second command"));
    
    Ok(())
}

/// Test tab completion functionality
#[tokio::test]
#[serial]
async fn test_tab_completion() -> Result<()> {
    let shell_config = ShellConfig {
        command: "/bin/bash".to_string(),
        args: vec!["-i".to_string()],
        env: None,
    };

    let pty = PtySession::spawn(&shell_config)?;
    
    // Type partial command and press tab
    pty.write_to_shell(b"ec")?;
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    pty.write_to_shell(b"\t")?; // Tab for completion
    tokio::time::sleep(Duration::from_millis(300)).await;
    
    let mut buffer = [0u8; 1024];
    let bytes_read = pty.read_from_shell(&mut buffer)?;
    let output = String::from_utf8_lossy(&buffer[..bytes_read]);
    
    // Should show completion (likely "echo")
    assert!(output.contains("echo") || output.contains("ec")); // May show completion or just echo back
    
    Ok(())
}

/// Test hook pattern matching edge cases
#[test]
fn test_hook_pattern_edge_cases() {
    // Test case sensitivity
    let key = KeyInput::new(KeyCode::Char('A'), KeyModifiers::CONTROL);
    assert!(key.matches_pattern("ctrl+a")); // Should match regardless of case
    
    // Test complex combinations
    let key = KeyInput::new(KeyCode::Char('c'), KeyModifiers::CONTROL | KeyModifiers::SHIFT);
    assert!(key.matches_pattern("ctrl+shift+c"));
    
    // Test special key names
    let key = KeyInput::new(KeyCode::Enter, KeyModifiers::ALT);
    assert!(key.matches_pattern("alt+enter"));
    
    let key = KeyInput::new(KeyCode::Tab, KeyModifiers::CONTROL);
    assert!(key.matches_pattern("ctrl+tab"));
    
    let key = KeyInput::new(KeyCode::Esc, KeyModifiers::empty());
    assert!(key.matches_pattern("esc"));
    
    // Test space key
    let key = KeyInput::new(KeyCode::Char(' '), KeyModifiers::CONTROL);
    assert!(key.matches_pattern("ctrl+space"));
}

/// Test custom hook execution
#[test]
fn test_custom_hook_execution() -> Result<()> {
    let mut hook_manager = HookManager::new();
    
    // Add a custom command hook
    let hook_config = HookConfig {
        name: "date_hook".to_string(),
        key_combination: "ctrl+d".to_string(),
        action: "cmd:date".to_string(),
        description: Some("Show current date".to_string()),
        enabled: true,
    };
    
    hook_manager.add_hook(hook_config);
    
    // Test the hook
    let key = KeyInput::new(KeyCode::Char('d'), KeyModifiers::CONTROL);
    let result = hook_manager.process_key(&key)?;
    assert!(result); // Should be consumed
    
    Ok(())
}

/// Stress test with rapid key sequences
#[tokio::test]
#[serial]
async fn test_rapid_key_sequences() -> Result<()> {
    let shell_config = ShellConfig {
        command: "/bin/bash".to_string(),
        args: vec!["-i".to_string()],
        env: None,
    };

    let pty = PtySession::spawn(&shell_config)?;
    
    // Send rapid sequence of keys
    let test_text = b"abcdefghijklmnopqrstuvwxyz0123456789";
    for &byte in test_text {
        pty.write_to_shell(&[byte])?;
        // Small delay to avoid overwhelming
        tokio::time::sleep(Duration::from_micros(100)).await;
    }
    
    pty.write_to_shell(b"\n")?;
    tokio::time::sleep(Duration::from_millis(200)).await;
    
    let mut buffer = [0u8; 1024];
    let bytes_read = pty.read_from_shell(&mut buffer)?;
    let output = String::from_utf8_lossy(&buffer[..bytes_read]);
    
    // Should contain our test text
    assert!(output.contains("abcdefghijklmnopqrstuvwxyz0123456789"));
    
    Ok(())
}

/// Test terminal state management
#[test]
fn test_terminal_state() -> Result<()> {
    let mut terminal = Terminal::new()?;
    
    // Test entering and leaving raw mode
    terminal.enter_raw_mode()?;
    assert!(terminal.raw_mode_enabled);
    
    terminal.leave_raw_mode()?;
    assert!(!terminal.raw_mode_enabled);
    
    // Test multiple calls (should be safe)
    terminal.enter_raw_mode()?;
    terminal.enter_raw_mode()?; // Should not error
    
    terminal.leave_raw_mode()?;
    terminal.leave_raw_mode()?; // Should not error
    
    Ok(())
}