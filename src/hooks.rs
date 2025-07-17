use crate::config::HookConfig;
use crate::terminal::KeyInput;
use crate::window::WindowManager;
use crate::llm::{LlmService, LlmResponse};
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::process::{Command, Stdio};
use std::sync::Arc;
use tokio::sync::Mutex;

pub type HookAction = Box<dyn Fn(&KeyInput) -> Result<bool> + Send + Sync>;

#[derive(Debug)]
pub enum ActionType {
    Command(String),
    Function(String),
    Builtin(String),
    LlmPrompt,
    LlmReset,
}

#[derive(Debug)]
pub struct HookManager {
    hooks: HashMap<String, Hook>,
    window_manager: WindowManager,
    llm_service: Option<Arc<Mutex<LlmService>>>,
}

#[derive(Debug)]
pub struct Hook {
    pub config: HookConfig,
    pub action: ActionType,
}

impl Hook {
    pub fn new(config: HookConfig) -> Self {
        let action = Self::parse_action(&config.action);
        Hook { config, action }
    }

    fn parse_action(action_str: &str) -> ActionType {
        if action_str.starts_with("cmd:") {
            ActionType::Command(action_str[4..].to_string())
        } else if action_str.starts_with("fn:") {
            ActionType::Function(action_str[3..].to_string())
        } else if action_str.starts_with("builtin:") {
            ActionType::Builtin(action_str[8..].to_string())
        } else if action_str == "llm:prompt" {
            ActionType::LlmPrompt
        } else if action_str == "llm:reset" {
            ActionType::LlmReset
        } else {
            // Default to command
            ActionType::Command(action_str.to_string())
        }
    }

    pub fn matches(&self, key: &KeyInput) -> bool {
        if !self.config.enabled {
            return false;
        }
        key.matches_pattern(&self.config.key_combination)
    }

    pub async fn execute(&self, key: &KeyInput, window_manager: &mut WindowManager, llm_service: &Option<Arc<Mutex<LlmService>>>) -> Result<bool> {
        match &self.action {
            ActionType::Command(cmd) => self.execute_command(cmd, window_manager),
            ActionType::Function(func_name) => self.execute_function(func_name, key, window_manager),
            ActionType::Builtin(builtin_name) => self.execute_builtin(builtin_name, key, window_manager),
            ActionType::LlmPrompt => self.execute_llm_prompt(window_manager, llm_service).await,
            ActionType::LlmReset => self.execute_llm_reset(window_manager, llm_service).await,
        }
    }

    fn execute_command(&self, cmd: &str, window_manager: &mut WindowManager) -> Result<bool> {
        let output = Command::new("/bin/sh")
            .arg("-c")
            .arg(cmd)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .with_context(|| format!("Failed to execute command: {}", cmd))?;

        let content = if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            format!("Command failed:\n{}", stderr)
        } else {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if stdout.trim().is_empty() {
                "Command executed successfully (no output)".to_string()
            } else {
                stdout.trim().to_string()
            }
        };

        // Show result in popup window
        window_manager.show_popup(&format!("Command: {}", cmd), &content)?;

        // Return true to indicate the hook consumed the key event
        Ok(true)
    }

    fn execute_function(&self, func_name: &str, _key: &KeyInput, window_manager: &mut WindowManager) -> Result<bool> {
        match func_name {
            "show_help" => {
                let content = "=== ChatShell Help ===\n\nThis is a transparent shell wrapper.\nAll keystrokes are passed through to the underlying shell.\n\nSpecial key combinations can trigger hooks:\n- Ctrl+; : Show this help\n- Ctrl+T : Show current time\n- Ctrl+Shift+C : Show config info\n\nPress ESC to close this window.";
                window_manager.show_popup("Help", content)?;
                Ok(true)
            }
            "show_time" => {
                let now = chrono::Utc::now();
                let content = format!("Current time:\n{}\n\nLocal time:\n{}", 
                    now.format("%Y-%m-%d %H:%M:%S UTC"),
                    chrono::Local::now().format("%Y-%m-%d %H:%M:%S %Z"));
                window_manager.show_popup("Current Time", &content)?;
                Ok(true)
            }
            _ => {
                let content = format!("Unknown function: {}", func_name);
                window_manager.show_popup("Error", &content)?;
                Ok(false)
            }
        }
    }

    fn execute_builtin(&self, builtin_name: &str, _key: &KeyInput, window_manager: &mut WindowManager) -> Result<bool> {
        match builtin_name {
            "clear_screen" => {
                // For clear screen, we don't need a popup - just execute the action
                print!("\x1B[2J\x1B[H"); // ANSI clear screen and move cursor to home
                Ok(true)
            }
            "show_config" => {
                let content = format!("=== Current Hook Configuration ===\n\nName: {}\nKey: {}\nAction: {}\nEnabled: {}\n{}",
                    self.config.name,
                    self.config.key_combination,
                    self.config.action,
                    self.config.enabled,
                    self.config.description.as_ref().map(|d| format!("Description: {}", d)).unwrap_or_default()
                );
                window_manager.show_popup("Configuration", &content)?;
                Ok(true)
            }
            "toggle_hook" => {
                let content = "Hook toggle not implemented in this context";
                window_manager.show_popup("Toggle Hook", content)?;
                Ok(false)
            }
            _ => {
                let content = format!("Unknown builtin: {}", builtin_name);
                window_manager.show_popup("Error", &content)?;
                Ok(false)
            }
        }
    }

    async fn execute_llm_prompt(&self, window_manager: &mut WindowManager, llm_service: &Option<Arc<Mutex<LlmService>>>) -> Result<bool> {
        let Some(llm_service) = llm_service else {
            window_manager.show_popup("Error", "LLM service not available. Please check your configuration.")?;
            return Ok(true);
        };

        // Show input popup for user prompt
        match window_manager.show_input_popup("LLM Assistant", "Enter your prompt:") {
            Ok(Some(user_prompt)) => {
                // Process the prompt with LLM
                let mut llm = llm_service.lock().await;
                match llm.process_user_prompt(&user_prompt).await {
                    Ok(LlmResponse::TextResponse { content }) => {
                        window_manager.show_popup("LLM Response", &content)?;
                    }
                    Ok(LlmResponse::CommandRequest { command, explanation, tool_call_id }) => {
                        // Show command for user to edit/confirm
                        let prompt = format!("Command: {}\nExplanation: {}\n\nEdit command if needed:", command, explanation);
                        match window_manager.show_input_popup("Execute Command", &prompt) {
                            Ok(Some(final_command)) => {
                                // Execute the command
                                match Self::execute_shell_command(&final_command) {
                                    Ok(output) => {
                                        // Send result back to LLM
                                        match llm.process_command_result(&tool_call_id, &final_command, &output, true).await {
                                            Ok(LlmResponse::TextResponse { content }) => {
                                                window_manager.show_popup("Command Result", &content)?;
                                            }
                                            Ok(LlmResponse::CommandRequest { command, explanation, tool_call_id: _ }) => {
                                                // Handle follow-up commands recursively (for now, just show)
                                                window_manager.show_popup("Follow-up Command", &format!("{}\n\n{}", explanation, command))?;
                                            }
                                            Err(e) => {
                                                window_manager.show_popup("LLM Error", &format!("Error processing command result: {}", e))?;
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        let error_msg = format!("Command execution failed: {}", e);
                                        window_manager.show_popup("Command Error", &error_msg)?;
                                        // Also inform LLM of the failure
                                        let _ = llm.process_command_result(&tool_call_id, &final_command, &error_msg, false).await;
                                    }
                                }
                            }
                            Ok(None) => {
                                // User cancelled
                                window_manager.show_popup("Cancelled", "Command execution cancelled.")?;
                            }
                            Err(e) => {
                                window_manager.show_popup("Error", &format!("Error showing command popup: {}", e))?;
                            }
                        }
                    }
                    Err(e) => {
                        window_manager.show_popup("LLM Error", &format!("Error processing prompt: {}", e))?;
                    }
                }
            }
            Ok(None) => {
                // User cancelled
            }
            Err(e) => {
                window_manager.show_popup("Error", &format!("Error showing input popup: {}", e))?;
            }
        }

        Ok(true)
    }

    async fn execute_llm_reset(&self, window_manager: &mut WindowManager, llm_service: &Option<Arc<Mutex<LlmService>>>) -> Result<bool> {
        let Some(llm_service) = llm_service else {
            window_manager.show_popup("Error", "LLM service not available.")?;
            return Ok(true);
        };

        let mut llm = llm_service.lock().await;
        llm.reset_context();
        window_manager.show_popup("LLM Context Reset", "Conversation context has been reset.")?;
        Ok(true)
    }

    fn execute_shell_command(command: &str) -> Result<String> {
        let output = Command::new("/bin/sh")
            .arg("-c")
            .arg(command)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .with_context(|| format!("Failed to execute command: {}", command))?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            Ok(stdout.trim().to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(anyhow::anyhow!("Command failed: {}", stderr))
        }
    }
}

impl HookManager {
    pub fn new() -> Self {
        HookManager {
            hooks: HashMap::new(),
            window_manager: WindowManager::default(),
            llm_service: None,
        }
    }

    pub fn from_configs(configs: Vec<HookConfig>) -> Self {
        let mut manager = Self::new();
        for config in configs {
            manager.add_hook(config);
        }
        manager
    }

    pub fn set_llm_service(&mut self, llm_service: Arc<Mutex<LlmService>>) {
        self.llm_service = Some(llm_service);
    }

    pub fn add_hook(&mut self, config: HookConfig) {
        let hook = Hook::new(config.clone());
        self.hooks.insert(config.name.clone(), hook);
    }

    pub fn remove_hook(&mut self, name: &str) -> bool {
        self.hooks.remove(name).is_some()
    }

    pub fn get_hook(&self, name: &str) -> Option<&Hook> {
        self.hooks.get(name)
    }

    pub fn get_hook_mut(&mut self, name: &str) -> Option<&mut Hook> {
        self.hooks.get_mut(name)
    }

    pub fn enable_hook(&mut self, name: &str, enabled: bool) -> bool {
        if let Some(hook) = self.hooks.get_mut(name) {
            hook.config.enabled = enabled;
            true
        } else {
            false
        }
    }

    pub async fn process_key(&mut self, key: &KeyInput) -> Result<bool> {
        for hook in self.hooks.values() {
            if hook.matches(key) {
                match hook.execute(key, &mut self.window_manager, &self.llm_service).await {
                    Ok(consumed) => {
                        if consumed {
                            return Ok(true); // Key was consumed by hook
                        }
                    }
                    Err(e) => {
                        eprintln!("Hook '{}' execution failed: {}", hook.config.name, e);
                        // Continue processing other hooks
                    }
                }
            }
        }
        Ok(false) // No hook consumed the key
    }

    pub fn list_hooks(&self) -> Vec<&HookConfig> {
        self.hooks.values().map(|h| &h.config).collect()
    }

    pub fn list_enabled_hooks(&self) -> Vec<&HookConfig> {
        self.hooks
            .values()
            .filter(|h| h.config.enabled)
            .map(|h| &h.config)
            .collect()
    }
}

// Built-in hook functions that can be referenced in config
pub fn create_default_hooks() -> Vec<HookConfig> {
    vec![
        HookConfig {
            name: "help".to_string(),
            key_combination: "ctrl+;".to_string(),
            action: "fn:show_help".to_string(),
            description: Some("Show help information".to_string()),
            enabled: true,
        },
        HookConfig {
            name: "llm_prompt".to_string(),
            key_combination: "ctrl+shift+l".to_string(),
            action: "llm:prompt".to_string(),
            description: Some("Open LLM prompt input".to_string()),
            enabled: true,
        },
        HookConfig {
            name: "llm_reset".to_string(),
            key_combination: "ctrl+shift+q".to_string(),
            action: "llm:reset".to_string(),
            description: Some("Reset LLM conversation context".to_string()),
            enabled: true,
        },
        HookConfig {
            name: "time".to_string(),
            key_combination: "ctrl+t".to_string(),
            action: "fn:show_time".to_string(),
            description: Some("Show current time".to_string()),
            enabled: false, // Disabled by default
        },
        HookConfig {
            name: "clear".to_string(),
            key_combination: "ctrl+l".to_string(),
            action: "builtin:clear_screen".to_string(),
            description: Some("Clear screen".to_string()),
            enabled: false, // Let normal Ctrl+L pass through by default
        },
        HookConfig {
            name: "config_info".to_string(),
            key_combination: "ctrl+shift+c".to_string(),
            action: "builtin:show_config".to_string(),
            description: Some("Show configuration info".to_string()),
            enabled: true,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terminal::KeyInput;
    use crossterm::event::{KeyCode, KeyModifiers};

    #[test]
    fn test_hook_matching() {
        let config = HookConfig {
            name: "test".to_string(),
            key_combination: "ctrl+;".to_string(),
            action: "echo test".to_string(),
            description: None,
            enabled: true,
        };

        let hook = Hook::new(config);
        let key = KeyInput::new(KeyCode::Char(';'), KeyModifiers::CONTROL);
        
        assert!(hook.matches(&key));
    }

    #[test]
    fn test_hook_manager() {
        let mut manager = HookManager::new();
        let config = HookConfig {
            name: "test".to_string(),
            key_combination: "ctrl+a".to_string(),
            action: "builtin:clear_screen".to_string(),
            description: None,
            enabled: true,
        };

        manager.add_hook(config);
        assert!(manager.get_hook("test").is_some());
        assert!(manager.remove_hook("test"));
        assert!(manager.get_hook("test").is_none());
    }

    #[test]
    fn test_action_parsing() {
        let action = Hook::parse_action("cmd:ls -la");
        assert!(matches!(action, ActionType::Command(_)));

        let action = Hook::parse_action("fn:show_help");
        assert!(matches!(action, ActionType::Function(_)));

        let action = Hook::parse_action("builtin:clear_screen");
        assert!(matches!(action, ActionType::Builtin(_)));
    }
}