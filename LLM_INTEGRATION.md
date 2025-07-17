# LLM Integration in ChatShell

ChatShell now features integrated LLM (Large Language Model) capabilities that allow you to interact with OpenAI's API directly from your terminal. This feature enables you to get AI assistance for shell commands, automate tasks, and get intelligent suggestions.

## Features

### Core Functionality
- **Interactive Prompt Interface**: Press `Ctrl+Shift+L` to open an input dialog where you can type your request
- **Command Generation**: The AI analyzes your natural language request and generates appropriate shell commands
- **Command Editing**: Before execution, you can review and edit the AI-generated commands
- **Context Awareness**: The AI maintains conversation context across multiple interactions in the same session
- **Context Reset**: Press `Ctrl+Shift+Q` to clear the conversation history and start fresh

### AI Capabilities
- **Command Translation**: Convert natural language to shell commands
- **Error Analysis**: Get explanations and solutions for command failures
- **Best Practices**: Receive suggestions for safer or more efficient alternatives
- **Documentation**: Get explanations of what commands do and their parameters

## Setup and Configuration

### Prerequisites
1. **OpenAI API Key**: You need a valid OpenAI API key to use LLM features
2. **Internet Connection**: Required for API communication

### Configuration Methods

#### Method 1: Environment Variable (Recommended)
```bash
export OPENAI_API_KEY="your-api-key-here"
```

#### Method 2: Configuration File
Edit your configuration file (`~/.config/chatshell/config.toml`):

```toml
[llm]
api_key = "your-api-key-here"
model = "gpt-4"
api_base = "https://api.openai.com/v1"
max_tokens = 1000
temperature = 0.7
```

### Configuration Options

| Option | Description | Default |
|--------|-------------|---------|
| `api_key` | Your OpenAI API key | From `OPENAI_API_KEY` env var |
| `model` | OpenAI model to use | `"gpt-4"` |
| `api_base` | API base URL | `"https://api.openai.com/v1"` |
| `max_tokens` | Maximum tokens per response | `1000` |
| `temperature` | Response randomness (0.0-1.0) | `0.7` |

### Key Bindings Configuration

The default shortcuts are defined in your config file:

```toml
[[hooks]]
name = "llm_prompt"
key_combination = "ctrl+shift+l"
action = "llm:prompt"
description = "Open LLM prompt input"
enabled = true

[[hooks]]
name = "llm_reset"
key_combination = "ctrl+shift+q"
action = "llm:reset"
description = "Reset LLM conversation context"
enabled = true
```

You can customize these shortcuts by editing the `key_combination` field.

## Usage Examples

### Basic Usage

1. **Open the prompt**: Press `Ctrl+Shift+L`
2. **Type your request**: For example: "List all files in the current directory"
3. **Review the command**: The AI will generate something like `ls -la`
4. **Edit if needed**: You can modify the command before execution
5. **Execute**: Press Enter to run the command, or Esc to cancel

### Example Interactions

#### File Operations
```
User: "Find all Python files larger than 1MB"
AI: find . -name "*.py" -size +1M
```

#### System Administration
```
User: "Show me the top 10 processes using the most CPU"
AI: ps aux --sort=-%cpu | head -10
```

#### Git Operations
```
User: "Show me the git log for the last week"
AI: git log --since="1 week ago" --oneline
```

#### Text Processing
```
User: "Count the number of lines in all .js files"
AI: find . -name "*.js" -exec wc -l {} + | tail -1
```

### Advanced Features

#### Context-Aware Conversations
The AI remembers previous commands and their results:
```
User: "List all files"
AI: ls -la
[Command executes successfully]

User: "Now sort them by size"
AI: ls -laS
```

#### Error Handling
When commands fail, the AI can provide explanations and alternatives:
```
User: "Delete all .log files"
AI: rm *.log
[Command fails: No such file or directory]

AI: "It looks like there are no .log files in the current directory. 
     You could try: find . -name '*.log' -delete to search recursively."
```

### Best Practices

1. **Be Specific**: Provide clear, specific requests for better results
2. **Review Commands**: Always review generated commands before execution
3. **Use Context**: Build on previous interactions for complex tasks
4. **Reset When Needed**: Use `Ctrl+Shift+Q` to clear context for new tasks

## Troubleshooting

### Common Issues

#### LLM Features Not Available
**Problem**: Warning message about LLM features being disabled
**Solutions**:
- Verify your OpenAI API key is set correctly
- Check your internet connection
- Ensure the API key has sufficient credits

#### API Request Failures
**Problem**: Error messages about API requests failing
**Solutions**:
- Check your API key validity
- Verify you have sufficient OpenAI credits
- Ensure the API base URL is correct

#### Slow Response Times
**Problem**: Long delays when generating commands
**Solutions**:
- Check your internet connection
- Consider using a faster model (e.g., `gpt-3.5-turbo`)
- Reduce `max_tokens` in configuration

### Configuration Validation

To verify your configuration is working:
1. Start ChatShell: `./chatshell`
2. Look for the startup message: "LLM Assistant enabled. Press Ctrl+Shift+L to open prompt."
3. If you see warnings about LLM features being disabled, check your API key configuration

### Debug Mode

For debugging API issues, you can run ChatShell with debug logging:
```bash
RUST_LOG=debug ./chatshell
```

## Security Considerations

### API Key Security
- Never commit API keys to version control
- Use environment variables or secure configuration files
- Restrict API key permissions if possible

### Command Safety
- Always review generated commands before execution
- Be cautious with destructive operations (`rm`, `chmod`, etc.)
- Test commands in safe environments first

### Network Security
- All API communication uses HTTPS
- Consider using a VPN for sensitive operations
- Be aware that your prompts are sent to OpenAI's servers

## Limitations

### Current Limitations
- Requires internet connection for all LLM operations
- API usage is subject to OpenAI's rate limits and pricing
- Complex multi-step operations may require multiple interactions
- Cannot directly access file contents or system state

### Future Enhancements
- Offline mode with local models
- Enhanced context awareness with file content analysis
- Integration with more AI providers
- Advanced automation capabilities

## Tips for Effective Usage

### Writing Good Prompts
- Be specific about what you want to achieve
- Include relevant context (file types, directory structure, etc.)
- Mention any constraints or preferences
- Use examples when describing complex operations

### Command Categories That Work Well
- **File Operations**: Finding, copying, moving, organizing files
- **Text Processing**: Searching, filtering, transforming text
- **System Administration**: Process management, system monitoring
- **Development Tasks**: Git operations, build processes, testing
- **Data Analysis**: Log analysis, statistics, reporting

### Building Complex Workflows
1. Start with simple operations
2. Use context to build incrementally
3. Break complex tasks into smaller steps
4. Verify each step before proceeding

## Support and Feedback

### Getting Help
- Check the main README.md for general ChatShell usage
- Review configuration files for syntax examples
- Use the help command: Press `Ctrl+;` for general help

### Reporting Issues
When reporting LLM-related issues, include:
- Your configuration (without API keys)
- Error messages
- Steps to reproduce the problem
- Expected vs actual behavior

### Contributing
The LLM integration is part of the open-source ChatShell project. Contributions are welcome for:
- Additional AI provider integrations
- Enhanced user interface
- Performance improvements
- Documentation updates

## Technical Architecture

### Components
- **LLM Service**: Handles API communication and context management
- **Hook System**: Integrates LLM commands with keyboard shortcuts
- **Window Manager**: Provides popup interfaces for input and output
- **Configuration System**: Manages LLM settings and API keys

### Data Flow
1. User presses shortcut → Hook system activates
2. Input popup opens → User types request
3. LLM service processes request → API call to OpenAI
4. Response parsed → Command extracted or text displayed
5. Command popup shows → User can edit/confirm/cancel
6. Command executed → Results fed back to LLM for context

This architecture ensures secure, responsive, and user-friendly AI-assisted shell operations while maintaining the transparency and control that ChatShell is designed for.