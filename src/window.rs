use crossterm::{
    cursor,
    event::{Event, KeyCode, KeyEvent, KeyModifiers},
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

    pub fn show_input_popup(&mut self, title: &str, initial_content: &str) -> Result<Option<String>> {
        // Split initial content into lines for display
        let lines: Vec<String> = initial_content.lines().map(|s| s.to_string()).collect();
        let content_width = lines.iter().map(|line| line.len()).max().unwrap_or(0);
        let min_width = title.len() + 4; // Account for borders and padding
        
        let window_width = std::cmp::max(content_width + 4, min_width).max(60) as u16; // Minimum width for input
        let window_height = (lines.len() + 6) as u16; // Content + borders + padding + input area
        
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

        self.draw_input_window(&window)?;
        
        // Handle input
        let result = self.handle_input(&window)?;
        
        self.clear_window(&window)?;
        
        Ok(result)
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

    fn draw_input_window(&self, window: &Window) -> Result<()> {
        let mut stdout = stdout();
        
        // Save cursor position
        stdout.queue(cursor::SavePosition)?;
        
        // Draw window background
        stdout.queue(SetBackgroundColor(Color::Blue))?;
        stdout.queue(SetForegroundColor(Color::White))?;
        
        // Draw top border with title
        stdout.queue(cursor::MoveTo(window.x, window.y))?;
        let title_with_padding = format!(" {} ", window.title);
        let title_padding = (window.width as usize).saturating_sub(title_with_padding.len());
        let left_padding = title_padding / 2;
        let right_padding = title_padding - left_padding;
        
        stdout.queue(Print(format!("{}{}{}",
            "═".repeat(left_padding),
            title_with_padding,
            "═".repeat(right_padding)
        )))?;
        
        // Draw content area
        for (i, line) in window.content.iter().enumerate() {
            stdout.queue(cursor::MoveTo(window.x, window.y + 1 + i as u16))?;
            stdout.queue(Print(format!("║ {:width$} ║", line, width = window.width as usize - 4)))?;
        }
        
        // Draw separator
        stdout.queue(cursor::MoveTo(window.x, window.y + 1 + window.content.len() as u16))?;
        stdout.queue(Print(format!("║{}║", "─".repeat(window.width as usize - 2))))?;
        
        // Draw input area
        let input_row = window.y + 2 + window.content.len() as u16;
        stdout.queue(cursor::MoveTo(window.x, input_row))?;
        stdout.queue(Print(format!("║ Input: {:width$} ║", "", width = window.width as usize - 11)))?;
        
        // Draw bottom border
        stdout.queue(cursor::MoveTo(window.x, window.y + window.height - 1))?;
        stdout.queue(Print("═".repeat(window.width as usize)))?;
        
        // Draw instructions
        let instructions = " Enter to confirm, Esc to cancel ";
        let instr_x = window.x + (window.width / 2) - (instructions.len() as u16 / 2);
        stdout.queue(cursor::MoveTo(instr_x, window.y + window.height))?;
        stdout.queue(SetBackgroundColor(Color::DarkGrey))?;
        stdout.queue(Print(instructions))?;
        
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

    fn handle_input(&self, window: &Window) -> Result<Option<String>> {
        let mut input = String::new();
        let input_row = window.y + 2 + window.content.len() as u16;
        let input_col = window.x + 9; // After "║ Input: "
        let max_input_width = window.width.saturating_sub(11) as usize; // Account for borders and "Input: "
        
        // Position cursor for input
        let mut stdout = stdout();
        stdout.queue(cursor::MoveTo(input_col, input_row))?;
        stdout.queue(cursor::Show)?;
        stdout.flush()?;
        
        loop {
            match crossterm::event::read()? {
                Event::Key(KeyEvent { code: KeyCode::Enter, .. }) => {
                    stdout.queue(cursor::Hide)?;
                    return Ok(Some(input));
                }
                Event::Key(KeyEvent { code: KeyCode::Esc, .. }) => {
                    stdout.queue(cursor::Hide)?;
                    return Ok(None);
                }
                Event::Key(KeyEvent { code: KeyCode::Backspace, .. }) => {
                    if !input.is_empty() {
                        input.pop();
                        self.redraw_input_line(&input, input_row, input_col, max_input_width)?;
                    }
                }
                Event::Key(KeyEvent { code: KeyCode::Char(c), modifiers, .. }) => {
                    // Handle Ctrl+C as cancel
                    if modifiers.contains(KeyModifiers::CONTROL) && c == 'c' {
                        stdout.queue(cursor::Hide)?;
                        return Ok(None);
                    }
                    
                    // Add character if there's space
                    if input.len() < max_input_width {
                        input.push(c);
                        self.redraw_input_line(&input, input_row, input_col, max_input_width)?;
                    }
                }
                _ => {
                    // Ignore other events
                }
            }
        }
    }

    fn redraw_input_line(&self, input: &str, row: u16, col: u16, max_width: usize) -> Result<()> {
        let mut stdout = stdout();
        
        // Clear the input area
        stdout.queue(cursor::MoveTo(col, row))?;
        stdout.queue(SetBackgroundColor(Color::Blue))?;
        stdout.queue(SetForegroundColor(Color::White))?;
        stdout.queue(Print(format!("{:width$}", "", width = max_width)))?;
        
        // Write the input
        stdout.queue(cursor::MoveTo(col, row))?;
        stdout.queue(Print(input))?;
        
        // Position cursor at end of input
        stdout.queue(cursor::MoveTo(col + input.len() as u16, row))?;
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