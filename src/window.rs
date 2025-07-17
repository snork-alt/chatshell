use crossterm::{
    cursor,
    event::{Event, KeyCode, KeyEvent},
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal::{Clear, ClearType},
    QueueableCommand,
};
use std::io::{stdout, Write};
use anyhow::Result;

#[derive(Debug)]
pub struct WindowManager {
    pub terminal_size: (u16, u16), // (cols, rows)
}

#[derive(Debug)]
pub struct Window {
    pub title: String,
    pub content: Vec<String>,
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

impl WindowManager {
    pub fn new() -> Result<Self> {
        let terminal_size = crossterm::terminal::size()?;
        Ok(WindowManager { terminal_size })
    }

    pub fn show_popup(&mut self, title: &str, content: &str) -> Result<()> {
        // Split content into lines and calculate window dimensions
        let lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
        let content_width = lines.iter().map(|line| line.len()).max().unwrap_or(0);
        let min_width = title.len() + 4; // Account for borders and padding
        
        let window_width = std::cmp::max(content_width + 4, min_width) as u16;
        let window_height = (lines.len() + 4) as u16; // Content + borders + padding
        
        // Center the window
        let x = (self.terminal_size.0.saturating_sub(window_width)) / 2;
        let y = (self.terminal_size.1.saturating_sub(window_height)) / 2;
        
        let window = Window {
            title: title.to_string(),
            content: lines,
            x,
            y,
            width: window_width,
            height: window_height,
        };

        self.draw_window(&window)?;
        self.wait_for_close()?;
        self.clear_window(&window)?;
        
        Ok(())
    }

    fn draw_window(&self, window: &Window) -> Result<()> {
        let mut stdout = stdout();
        
        // Save cursor position
        stdout.queue(cursor::SavePosition)?;
        
        // Draw window background and borders
        for row in 0..window.height {
            stdout.queue(cursor::MoveTo(window.x, window.y + row))?;
            
            if row == 0 {
                // Top border
                stdout.queue(SetBackgroundColor(Color::Blue))?;
                stdout.queue(SetForegroundColor(Color::White))?;
                stdout.queue(Print("┌"))?;
                for _ in 1..window.width - 1 {
                    stdout.queue(Print("─"))?;
                }
                stdout.queue(Print("┐"))?;
            } else if row == 1 {
                // Title row
                stdout.queue(SetBackgroundColor(Color::Blue))?;
                stdout.queue(SetForegroundColor(Color::White))?;
                stdout.queue(Print("│"))?;
                
                let title_padding = ((window.width - 2) as usize).saturating_sub(window.title.len());
                let left_padding = title_padding / 2;
                let right_padding = title_padding - left_padding;
                
                for _ in 0..left_padding {
                    stdout.queue(Print(" "))?;
                }
                stdout.queue(Print(&window.title))?;
                for _ in 0..right_padding {
                    stdout.queue(Print(" "))?;
                }
                
                stdout.queue(Print("│"))?;
            } else if row == 2 {
                // Separator row
                stdout.queue(SetBackgroundColor(Color::Blue))?;
                stdout.queue(SetForegroundColor(Color::White))?;
                stdout.queue(Print("├"))?;
                for _ in 1..window.width - 1 {
                    stdout.queue(Print("─"))?;
                }
                stdout.queue(Print("┤"))?;
            } else if row == window.height - 1 {
                // Bottom border
                stdout.queue(SetBackgroundColor(Color::Blue))?;
                stdout.queue(SetForegroundColor(Color::White))?;
                stdout.queue(Print("└"))?;
                for _ in 1..window.width - 1 {
                    stdout.queue(Print("─"))?;
                }
                stdout.queue(Print("┘"))?;
            } else {
                // Content rows
                stdout.queue(SetBackgroundColor(Color::Blue))?;
                stdout.queue(SetForegroundColor(Color::White))?;
                stdout.queue(Print("│"))?;
                
                stdout.queue(SetBackgroundColor(Color::DarkBlue))?;
                stdout.queue(SetForegroundColor(Color::White))?;
                
                let content_row = row - 3; // Account for title and borders
                if content_row < window.content.len() as u16 {
                    let line = &window.content[content_row as usize];
                    stdout.queue(Print(" "))?; // Left padding
                    stdout.queue(Print(line))?;
                    
                    // Right padding
                    let line_len = line.len();
                    let available_width = (window.width - 3) as usize; // -3 for borders and left padding
                    if line_len < available_width {
                        for _ in 0..(available_width - line_len) {
                            stdout.queue(Print(" "))?;
                        }
                    }
                } else {
                    // Empty content row
                    for _ in 0..window.width - 2 {
                        stdout.queue(Print(" "))?;
                    }
                }
                
                stdout.queue(SetBackgroundColor(Color::Blue))?;
                stdout.queue(SetForegroundColor(Color::White))?;
                stdout.queue(Print("│"))?;
            }
        }
        
        // Draw close instruction at bottom
        let close_msg = "Press ESC to close";
        let close_x = window.x + window.width - close_msg.len() as u16 - 2;
        let close_y = window.y + window.height - 1;
        
        stdout.queue(cursor::MoveTo(close_x, close_y))?;
        stdout.queue(SetBackgroundColor(Color::Blue))?;
        stdout.queue(SetForegroundColor(Color::Yellow))?;
        stdout.queue(Print(close_msg))?;
        
        stdout.queue(ResetColor)?;
        stdout.flush()?;
        
        Ok(())
    }

    fn wait_for_close(&self) -> Result<()> {
        loop {
            match crossterm::event::read()? {
                Event::Key(KeyEvent { code: KeyCode::Esc, .. }) => {
                    break;
                }
                _ => {
                    // Ignore other events
                }
            }
        }
        Ok(())
    }

    fn clear_window(&self, window: &Window) -> Result<()> {
        let mut stdout = stdout();
        
        // Clear the window area
        for row in 0..window.height {
            stdout.queue(cursor::MoveTo(window.x, window.y + row))?;
            stdout.queue(Clear(ClearType::UntilNewLine))?;
        }
        
        // Restore cursor position
        stdout.queue(cursor::RestorePosition)?;
        stdout.queue(ResetColor)?;
        stdout.flush()?;
        
        Ok(())
    }
}

impl Default for WindowManager {
    fn default() -> Self {
        WindowManager::new().unwrap_or(WindowManager {
            terminal_size: (80, 24),
        })
    }
} 