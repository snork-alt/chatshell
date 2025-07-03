use crossterm::{
    cursor,
    event::{DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, size, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io::{stdout, Stdout, Write};
use anyhow::{Context, Result};

#[derive(Debug)]
pub struct Terminal {
    stdout: Stdout,
    raw_mode_enabled: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KeyInput {
    pub code: KeyCode,
    pub modifiers: KeyModifiers,
    pub raw_bytes: Vec<u8>,
}

impl KeyInput {
    pub fn new(code: KeyCode, modifiers: KeyModifiers) -> Self {
        Self {
            code,
            modifiers,
            raw_bytes: Self::key_to_bytes(code, modifiers),
        }
    }

    pub fn from_event(event: KeyEvent) -> Self {
        Self::new(event.code, event.modifiers)
    }

    pub fn matches_pattern(&self, pattern: &str) -> bool {
        let pattern_lower = pattern.to_lowercase();
        
        // Parse pattern like "ctrl+;" or "alt+enter"
        let parts: Vec<&str> = pattern_lower.split('+').collect();
        if parts.len() < 2 {
            return false;
        }

        let (modifier_parts, key_part) = parts.split_at(parts.len() - 1);
        let key_part = key_part[0];

        // Check if modifiers match
        let mut expected_modifiers = KeyModifiers::empty();
        for modifier in modifier_parts {
            match *modifier {
                "ctrl" => expected_modifiers |= KeyModifiers::CONTROL,
                "alt" => expected_modifiers |= KeyModifiers::ALT,
                "shift" => expected_modifiers |= KeyModifiers::SHIFT,
                _ => return false,
            }
        }

        if self.modifiers != expected_modifiers {
            return false;
        }

        // Check if key matches
        match key_part {
            ";" => matches!(self.code, KeyCode::Char(';')),
            "enter" => matches!(self.code, KeyCode::Enter),
            "tab" => matches!(self.code, KeyCode::Tab),
            "space" => matches!(self.code, KeyCode::Char(' ')),
            "esc" => matches!(self.code, KeyCode::Esc),
            "backspace" => matches!(self.code, KeyCode::Backspace),
            key if key.len() == 1 => {
                if let Some(ch) = key.chars().next() {
                    matches!(self.code, KeyCode::Char(c) if c.to_lowercase().next() == Some(ch))
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    fn key_to_bytes(code: KeyCode, modifiers: KeyModifiers) -> Vec<u8> {
        match (code, modifiers.contains(KeyModifiers::CONTROL)) {
            (KeyCode::Char(c), true) => {
                // Control characters
                let control_code = (c.to_ascii_uppercase() as u8) - b'A' + 1;
                vec![control_code]
            }
            (KeyCode::Char(c), false) => {
                if modifiers.contains(KeyModifiers::ALT) {
                    vec![27, c as u8] // ESC + char for Alt
                } else {
                    vec![c as u8]
                }
            }
            (KeyCode::Enter, _) => vec![b'\r'],
            (KeyCode::Tab, _) => vec![b'\t'],
            (KeyCode::Backspace, _) => vec![127],
            (KeyCode::Esc, _) => vec![27],
            (KeyCode::Up, _) => vec![27, 91, 65],
            (KeyCode::Down, _) => vec![27, 91, 66],
            (KeyCode::Right, _) => vec![27, 91, 67],
            (KeyCode::Left, _) => vec![27, 91, 68],
            (KeyCode::Home, _) => vec![27, 91, 72],
            (KeyCode::End, _) => vec![27, 91, 70],
            (KeyCode::PageUp, _) => vec![27, 91, 53, 126],
            (KeyCode::PageDown, _) => vec![27, 91, 54, 126],
            (KeyCode::Delete, _) => vec![27, 91, 51, 126],
            (KeyCode::Insert, _) => vec![27, 91, 50, 126],
            (KeyCode::F(n), _) => {
                match n {
                    1..=4 => vec![27, 79, 80 + (n - 1) as u8],
                    5..=12 => vec![27, 91, 49, 53 + (n - 5) as u8, 126],
                    _ => vec![], // Unsupported F-key
                }
            }
            _ => vec![], // Other keys not supported
        }
    }
}

impl Terminal {
    pub fn new() -> Result<Self> {
        Ok(Terminal {
            stdout: stdout(),
            raw_mode_enabled: false,
        })
    }

    pub fn enter_raw_mode(&mut self) -> Result<()> {
        if !self.raw_mode_enabled {
            enable_raw_mode()
                .with_context(|| "Failed to enable raw mode")?;
            self.raw_mode_enabled = true;
        }
        Ok(())
    }

    pub fn leave_raw_mode(&mut self) -> Result<()> {
        if self.raw_mode_enabled {
            disable_raw_mode()
                .with_context(|| "Failed to disable raw mode")?;
            self.raw_mode_enabled = false;
        }
        Ok(())
    }

    pub fn setup_alternate_screen(&mut self) -> Result<()> {
        execute!(
            self.stdout,
            EnterAlternateScreen,
            EnableMouseCapture,
            cursor::Hide
        )
        .with_context(|| "Failed to setup alternate screen")?;
        Ok(())
    }

    pub fn restore_screen(&mut self) -> Result<()> {
        execute!(
            self.stdout,
            LeaveAlternateScreen,
            DisableMouseCapture,
            cursor::Show
        )
        .with_context(|| "Failed to restore screen")?;
        Ok(())
    }

    pub fn size(&self) -> Result<(u16, u16)> {
        size().with_context(|| "Failed to get terminal size")
    }

    pub fn flush(&mut self) -> Result<()> {
        self.stdout.flush()
            .with_context(|| "Failed to flush stdout")
    }

    pub fn write(&mut self, data: &[u8]) -> Result<usize> {
        let bytes_written = self.stdout.write(data)
            .with_context(|| "Failed to write to stdout")?;
        self.flush()?;
        Ok(bytes_written)
    }

    pub fn read_event(&self) -> Result<Event> {
        crossterm::event::read()
            .with_context(|| "Failed to read terminal event")
    }

    pub fn poll_event(&self, timeout: std::time::Duration) -> Result<bool> {
        crossterm::event::poll(timeout)
            .with_context(|| "Failed to poll for terminal events")
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        let _ = self.leave_raw_mode();
        let _ = self.restore_screen();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_pattern_matching() {
        let key = KeyInput::new(KeyCode::Char(';'), KeyModifiers::CONTROL);
        assert!(key.matches_pattern("ctrl+;"));
        assert!(!key.matches_pattern("alt+;"));
        assert!(!key.matches_pattern("ctrl+a"));
    }

    #[test]
    fn test_key_to_bytes() {
        let key = KeyInput::new(KeyCode::Char('a'), KeyModifiers::CONTROL);
        assert_eq!(key.raw_bytes, vec![1]); // Ctrl+A = 1

        let key = KeyInput::new(KeyCode::Enter, KeyModifiers::empty());
        assert_eq!(key.raw_bytes, vec![13]); // Enter = \r
    }

    #[test]
    fn test_alt_key_combination() {
        let key = KeyInput::new(KeyCode::Char('a'), KeyModifiers::ALT);
        assert_eq!(key.raw_bytes, vec![27, 97]); // ESC + 'a'
    }
}