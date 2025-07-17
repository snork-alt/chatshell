use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::SystemTime;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    pub api_key: String,
    pub model: String,
    pub api_base: String,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            api_key: std::env::var("OPENAI_API_KEY").unwrap_or_default(),
            model: "gpt-4".to_string(),
            api_base: "https://api.openai.com/v1".to_string(),
            max_tokens: Some(1000),
            temperature: Some(0.7),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    pub tool_calls: Option<Vec<ToolCall>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub r#type: String,
    pub function: FunctionCall,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub r#type: String,
    pub function: ToolFunction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolFunction {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

#[derive(Debug, Clone, Serialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<ChatMessage>,
    tools: Option<Vec<Tool>>,
    tool_choice: Option<String>,
    max_tokens: Option<u32>,
    temperature: Option<f32>,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: ChatMessage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationContext {
    pub id: String,
    pub messages: Vec<ChatMessage>,
    pub created_at: SystemTime,
}

impl ConversationContext {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            messages: Vec::new(),
            created_at: SystemTime::now(),
        }
    }

    pub fn add_message(&mut self, message: ChatMessage) {
        self.messages.push(message);
    }

    pub fn clear(&mut self) {
        self.messages.clear();
        self.id = Uuid::new_v4().to_string();
        self.created_at = SystemTime::now();
    }
}

#[derive(Debug)]
pub struct LlmService {
    client: Client,
    config: LlmConfig,
    context: ConversationContext,
}

impl LlmService {
    pub fn new(config: LlmConfig) -> Result<Self> {
        let client = Client::new();
        let context = ConversationContext::new();
        
        Ok(Self {
            client,
            config,
            context,
        })
    }

    pub fn reset_context(&mut self) {
        self.context.clear();
    }

    pub fn get_context(&self) -> &ConversationContext {
        &self.context
    }

    fn get_system_prompt() -> String {
        r#"You are a specialized AI assistant designed to help users execute shell commands efficiently and safely. Your primary role is to:

1. Understand user requests and translate them into appropriate shell commands
2. Execute commands through the provided tool when requested
3. Provide explanations for commands when helpful
4. Suggest alternatives or improvements when appropriate
5. Be cautious with potentially dangerous commands

Guidelines:
- Always use the execute_command tool when you need to run shell commands
- Provide clear explanations of what commands do
- Ask for confirmation before running potentially destructive commands
- Suggest safer alternatives when possible
- Be concise but informative in your responses

You have access to a tool called "execute_command" that allows you to run shell commands. Use this tool whenever you need to execute commands to fulfill user requests."#.to_string()
    }

    fn get_shell_execution_tool() -> Tool {
        Tool {
            r#type: "function".to_string(),
            function: ToolFunction {
                name: "execute_command".to_string(),
                description: "Execute a shell command and return the output".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "command": {
                            "type": "string",
                            "description": "The shell command to execute"
                        },
                        "explanation": {
                            "type": "string",
                            "description": "Brief explanation of what this command does"
                        }
                    },
                    "required": ["command", "explanation"]
                }),
            },
        }
    }

    pub async fn process_user_prompt(&mut self, prompt: &str) -> Result<LlmResponse> {
        // Add system message if this is the first message
        if self.context.messages.is_empty() {
            self.context.add_message(ChatMessage {
                role: "system".to_string(),
                content: Self::get_system_prompt(),
                tool_calls: None,
            });
        }

        // Add user message
        self.context.add_message(ChatMessage {
            role: "user".to_string(),
            content: prompt.to_string(),
            tool_calls: None,
        });

        // Make API request
        let tools = vec![Self::get_shell_execution_tool()];
        let response = self.call_openai_api(tools).await?;

        // Process the response
        match response.choices.first() {
            Some(choice) => {
                let message = &choice.message;
                self.context.add_message(message.clone());

                if let Some(tool_calls) = &message.tool_calls {
                    if let Some(tool_call) = tool_calls.first() {
                        let function_args: Value = serde_json::from_str(&tool_call.function.arguments)
                            .context("Failed to parse tool call arguments")?;
                        
                        let command = function_args["command"]
                            .as_str()
                            .context("Missing command in tool call")?;
                        
                        let explanation = function_args["explanation"]
                            .as_str()
                            .unwrap_or("No explanation provided");

                        return Ok(LlmResponse::CommandRequest {
                            command: command.to_string(),
                            explanation: explanation.to_string(),
                            tool_call_id: tool_call.id.clone(),
                        });
                    }
                }

                Ok(LlmResponse::TextResponse {
                    content: message.content.clone(),
                })
            }
            None => Err(anyhow::anyhow!("No response from OpenAI API")),
        }
    }

    pub async fn process_command_result(&mut self, _tool_call_id: &str, _command: &str, output: &str, success: bool) -> Result<LlmResponse> {
        // Add the tool response to context
        self.context.add_message(ChatMessage {
            role: "tool".to_string(),
            content: if success {
                format!("Command executed successfully:\n{}", output)
            } else {
                format!("Command failed:\n{}", output)
            },
            tool_calls: None,
        });

        // Get follow-up response from the model
        let tools = vec![Self::get_shell_execution_tool()];
        let response = self.call_openai_api(tools).await?;

        match response.choices.first() {
            Some(choice) => {
                let message = &choice.message;
                self.context.add_message(message.clone());

                if let Some(tool_calls) = &message.tool_calls {
                    if let Some(tool_call) = tool_calls.first() {
                        let function_args: Value = serde_json::from_str(&tool_call.function.arguments)
                            .context("Failed to parse tool call arguments")?;
                        
                        let command = function_args["command"]
                            .as_str()
                            .context("Missing command in tool call")?;
                        
                        let explanation = function_args["explanation"]
                            .as_str()
                            .unwrap_or("No explanation provided");

                        return Ok(LlmResponse::CommandRequest {
                            command: command.to_string(),
                            explanation: explanation.to_string(),
                            tool_call_id: tool_call.id.clone(),
                        });
                    }
                }

                Ok(LlmResponse::TextResponse {
                    content: message.content.clone(),
                })
            }
            None => Err(anyhow::anyhow!("No response from OpenAI API")),
        }
    }

    async fn call_openai_api(&self, tools: Vec<Tool>) -> Result<ChatCompletionResponse> {
        let url = format!("{}/chat/completions", self.config.api_base);
        
        let request = ChatCompletionRequest {
            model: self.config.model.clone(),
            messages: self.context.messages.clone(),
            tools: Some(tools),
            tool_choice: Some("auto".to_string()),
            max_tokens: self.config.max_tokens,
            temperature: self.config.temperature,
        };

        let response = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .context("Failed to send request to OpenAI API")?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "OpenAI API request failed with status {}: {}",
                status,
                error_text
            ));
        }

        let api_response: ChatCompletionResponse = response
            .json()
            .await
            .context("Failed to parse OpenAI API response")?;

        Ok(api_response)
    }
}

#[derive(Debug, Clone)]
pub enum LlmResponse {
    TextResponse { content: String },
    CommandRequest { command: String, explanation: String, tool_call_id: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conversation_context() {
        let mut context = ConversationContext::new();
        assert!(context.messages.is_empty());
        
        context.add_message(ChatMessage {
            role: "user".to_string(),
            content: "Hello".to_string(),
            tool_calls: None,
        });
        
        assert_eq!(context.messages.len(), 1);
        assert_eq!(context.messages[0].role, "user");
        assert_eq!(context.messages[0].content, "Hello");
    }

    #[test]
    fn test_context_clear() {
        let mut context = ConversationContext::new();
        let original_id = context.id.clone();
        
        context.add_message(ChatMessage {
            role: "user".to_string(),
            content: "Hello".to_string(),
            tool_calls: None,
        });
        
        context.clear();
        
        assert!(context.messages.is_empty());
        assert_ne!(context.id, original_id);
    }
}