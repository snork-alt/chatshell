# ChatShell - A Transparent Shell Wrapper with Hooks

ChatShell is a transparent shell wrapper written in Rust that captures all keystrokes and passes them through to an underlying shell while providing a powerful plugin/hook system for special key combinations.

## Features

- **Transparent Operation**: Every keystroke is captured and passed through to the shell, making it completely transparent
- **Cross-Shell Support**: Works with any shell (bash, zsh, fish, etc.) via configuration
- **Hook System**: Trigger custom actions with special key combinations
- **Raw Terminal Mode**: Captures all input including control characters, function keys, etc.
- **Full TTY Support**: Supports terminal applications like vim, nano, htop, etc.
- **Configurable**: TOML-based configuration system
- **Signal Handling**: Proper signal forwarding and cleanup

## Installation

### Prerequisites

- Rust 1.70+ 
- Linux/Unix system (uses PTY functionality)

### Build from Source

```bash
git clone <repository-url>
cd chatshell
cargo build --release
```

The binary will be available at `target/release/chatshell`.

## Quick Start

1. **Create default configuration:**
   ```bash
   ./chatshell --create-config
   ```
   This creates a configuration file at `~/.config/chatshell/config.toml`

2. **Run ChatShell:**
   ```bash
   ./chatshell
   ```

3. **Try the default hook:**
   Press `Ctrl+;` to see the help message

## Configuration

The configuration file is located at `~/.config/chatshell/config.toml`:

```toml
[shell]
command = "/bin/bash"
args = ["-i"]

[[hooks]]
name = "help"
key_combination = "ctrl+;"
action = "fn:show_help"
description = "Show help information"
enabled = true

[[hooks]]
name = "time"
key_combination = "ctrl+t"
action = "fn:show_time"
description = "Show current time"
enabled = false

[[hooks]]
name = "config_info"
key_combination = "ctrl+shift+c"
action = "builtin:show_config"
description = "Show configuration info"
enabled = true
```

### Shell Configuration

Configure which shell to run:

```toml
[shell]
command = "/bin/zsh"           # Shell executable
args = ["-i", "--login"]       # Shell arguments
```

You can also set environment variables:

```toml
[shell]
command = "/bin/bash"
args = ["-i"]

[shell.env]
EDITOR = "vim"
CUSTOM_VAR = "value"
```

### Hook Configuration

Hooks are triggered by key combinations and can execute different types of actions:

#### Hook Structure

```toml
[[hooks]]
name = "unique_name"           # Unique identifier
key_combination = "ctrl+;"     # Key combination pattern
action = "command_here"        # Action to execute
description = "Description"    # Optional description
enabled = true                 # Enable/disable the hook
```

#### Key Combination Patterns

Supported modifiers: `ctrl`, `alt`, `shift`
Supported keys: `a-z`, `0-9`, `;`, `enter`, `tab`, `space`, `esc`, `backspace`

Examples:
- `ctrl+;`
- `alt+enter`
- `ctrl+shift+c`
- `ctrl+a`

#### Action Types

**1. Commands (`cmd:` prefix or default):**
```toml
action = "cmd:ls -la"          # Execute shell command
action = "echo 'Hello World'"  # Default is command type
```

**2. Built-in Functions (`fn:` prefix):**
```toml
action = "fn:show_help"        # Show help
action = "fn:show_time"        # Show current time
```

**3. Built-in Actions (`builtin:` prefix):**
```toml
action = "builtin:clear_screen"    # Clear the screen
action = "builtin:show_config"     # Show current configuration
```

### Example Configurations

**Development Environment:**
```toml
[[hooks]]
name = "git_status"
key_combination = "ctrl+g"
action = "git status --short"
description = "Quick git status"
enabled = true

[[hooks]]
name = "test_runner"
key_combination = "ctrl+shift+t"
action = "cargo test"
description = "Run tests"
enabled = true
```

**System Administration:**
```toml
[[hooks]]
name = "disk_usage"
key_combination = "ctrl+d"
action = "df -h"
description = "Show disk usage"
enabled = true

[[hooks]]
name = "process_list"
key_combination = "ctrl+p"
action = "ps aux | head -20"
description = "Show top processes"
enabled = true
```

## Command Line Options

```bash
chatshell [OPTIONS]

Options:
    -c, --config <FILE>      Configuration file path
    -s, --shell <SHELL>      Shell command to run (overrides config)
        --create-config      Create default configuration file and exit
    -h, --help              Print help information
    -V, --version           Print version information
```

## Usage Examples

### Basic Usage

```bash
# Use default configuration
chatshell

# Use custom configuration
chatshell --config /path/to/config.toml

# Override shell command
chatshell --shell /bin/fish
```

### In Scripts

```bash
#!/bin/bash
# wrapper.sh - Custom wrapper script

export CHATSHELL_CONFIG="/path/to/project/config.toml"
exec /path/to/chatshell --config "$CHATSHELL_CONFIG"
```

## Architecture

ChatShell consists of several key components:

1. **Terminal Handler**: Manages raw terminal mode and keyboard input capture
2. **PTY Manager**: Spawns and manages the shell process via pseudo-terminal
3. **Hook System**: Processes key combinations and executes actions
4. **Configuration System**: TOML-based configuration management
5. **Event Loop**: Coordinates input/output between terminal and shell

```
┌─────────────┐    ┌──────────────┐    ┌─────────────┐
│   Terminal  │ -> │  ChatShell   │ -> │    Shell    │
│    Input    │    │  Hook System │    │   Process   │
└─────────────┘    └──────────────┘    └─────────────┘
       ^                   |                    |
       |                   v                    v
       └───────────── Terminal Output ──────────┘
```

## Building Custom Hooks

You can extend ChatShell by adding custom hook actions:

### 1. Command Hooks
Execute any shell command:
```toml
[[hooks]]
name = "backup"
key_combination = "ctrl+b"
action = "rsync -av ~/docs/ ~/backup/docs/"
enabled = true
```

### 2. Script Hooks
Execute custom scripts:
```toml
[[hooks]]
name = "deploy"
key_combination = "ctrl+shift+d"
action = "/path/to/deploy-script.sh"
enabled = true
```

### 3. Function Hooks
Built-in function hooks (expandable in source code):
- `show_help`: Display help information
- `show_time`: Display current time

## Troubleshooting

### Common Issues

**1. "Permission denied" errors:**
```bash
# Ensure ChatShell is executable
chmod +x chatshell
```

**2. "Shell not found" errors:**
```bash
# Check shell path in configuration
which bash  # Verify shell exists
```

**3. "Config file not found":**
```bash
# Create default config
./chatshell --create-config
```

**4. Terminal display issues:**
```bash
# Reset terminal if display is corrupted
reset
```

### Debug Mode

For debugging, you can run with:
```bash
RUST_LOG=debug ./chatshell
```

## Security Considerations

- ChatShell runs with the same privileges as the user
- Hook commands execute with user permissions
- Configuration files should have appropriate permissions (600)
- Be careful with hook commands that might expose sensitive data

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests if applicable
5. Submit a pull request

### Development Setup

```bash
git clone <repository-url>
cd chatshell
cargo build
cargo test
cargo run -- --create-config
cargo run
```

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Acknowledgments

- Built with the Rust programming language
- Uses the `nix` crate for Unix system calls
- Uses the `crossterm` crate for terminal manipulation
- Inspired by various terminal multiplexers and shell wrappers