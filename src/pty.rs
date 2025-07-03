THIS SHOULD BE A LINTER ERRORuse nix::pty::{forkpty, ForkptyResult};
use nix::sys::signal::{self, Signal};
use nix::sys::wait::{waitpid, WaitPidFlag, WaitStatus};
use nix::unistd::{execvp, Pid};
use std::ffi::CString;
use std::io::{Read, Write};
use std::os::unix::io::{AsRawFd, RawFd, FromRawFd};
use anyhow::{Context, Result};
use crate::config::ShellConfig;

#[derive(Debug)]
pub struct PtySession {
    pub master_fd: RawFd,
    pub child_pid: Pid,
}

impl PtySession {
    pub fn spawn(shell_config: &ShellConfig) -> Result<Self> {
        // Create PTY pair
        let fork_result = forkpty(None, None);

        match fork_result {
            Ok(Some(result)) => {
                // Parent process - return the PTY session
                Ok(PtySession {
                    master_fd: result.master.as_raw_fd(),
                    child_pid: result.fork_result,
                })
            }
            Ok(None) => {
                // Child process - exec the shell
                Self::exec_shell(shell_config)
                    .with_context(|| "Failed to exec shell")?;
                
                // This should never be reached
                std::process::exit(1);
            }
            Err(e) => Err(anyhow::anyhow!("forkpty failed: {}", e)),
        }
    }

    fn exec_shell(shell_config: &ShellConfig) -> Result<()> {
        // Set environment variables if specified
        if let Some(env) = &shell_config.env {
            for (key, value) in env {
                std::env::set_var(key, value);
            }
        }

        // Prepare command and arguments
        let command = CString::new(shell_config.command.clone())
            .with_context(|| "Invalid shell command")?;
        
        let mut args: Vec<CString> = Vec::new();
        args.push(command.clone()); // argv[0] should be the command itself
        
        for arg in &shell_config.args {
            args.push(CString::new(arg.clone())
                .with_context(|| format!("Invalid argument: {}", arg))?);
        }

        // Execute the shell
        execvp(&command, &args)
            .with_context(|| format!("Failed to execute shell: {}", shell_config.command))?;
        
        Ok(())
    }

    pub fn write_to_shell(&self, data: &[u8]) -> Result<usize> {
        let mut file = unsafe { std::fs::File::from_raw_fd(self.master_fd) };
        let result = file.write(data)
            .with_context(|| "Failed to write to shell");
        std::mem::forget(file); // Don't close the fd
        result
    }

    pub fn read_from_shell(&self, buffer: &mut [u8]) -> Result<usize> {
        let mut file = unsafe { std::fs::File::from_raw_fd(self.master_fd) };
        let result = file.read(buffer)
            .with_context(|| "Failed to read from shell");
        std::mem::forget(file); // Don't close the fd
        result
    }

    pub fn resize_pty(&self, rows: u16, cols: u16) -> Result<()> {
        use nix::libc::{winsize, ioctl, TIOCSWINSZ};

        let ws = winsize {
            ws_row: rows,
            ws_col: cols,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };

        let result = unsafe {
            ioctl(self.master_fd, TIOCSWINSZ, &ws as *const winsize)
        };

        if result == -1 {
            Err(anyhow::anyhow!("Failed to resize PTY"))
        } else {
            Ok(())
        }
    }

    pub fn send_signal(&self, signal: Signal) -> Result<()> {
        signal::kill(self.child_pid, signal)
            .with_context(|| format!("Failed to send signal {:?} to child process", signal))?;
        Ok(())
    }

    pub fn is_child_alive(&self) -> bool {
        match waitpid(self.child_pid, Some(WaitPidFlag::WNOHANG)) {
            Ok(WaitStatus::StillAlive) => true,
            _ => false,
        }
    }

    pub fn wait_for_child(&self) -> Result<WaitStatus> {
        waitpid(self.child_pid, None)
            .with_context(|| "Failed to wait for child process")
    }
}

impl Drop for PtySession {
    fn drop(&mut self) {
        // Try to terminate the child process gracefully
        if self.is_child_alive() {
            let _ = self.send_signal(Signal::SIGTERM);
            
            // Give it a moment to terminate
            std::thread::sleep(std::time::Duration::from_millis(100));
            
            // Force kill if still alive
            if self.is_child_alive() {
                let _ = self.send_signal(Signal::SIGKILL);
            }
        }
        
        // Close the master fd
        unsafe {
            libc::close(self.master_fd);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ShellConfig;

    #[test]
    fn test_pty_creation() {
        let shell_config = ShellConfig {
            command: "/bin/echo".to_string(),
            args: vec!["hello".to_string()],
            env: None,
        };

        let pty = PtySession::spawn(&shell_config);
        assert!(pty.is_ok());
    }
}