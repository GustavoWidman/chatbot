# Discord AI Chatbot

A sophisticated Discord bot that leverages large language models (LLMs) for creating immersive, context-aware conversations with long-term memory capabilities.

![Discord Bot](https://img.shields.io/badge/Discord-Bot-5865F2?style=for-the-badge&logo=discord&logoColor=white)
![Rust](https://img.shields.io/badge/Rust-1.85-orange?style=for-the-badge&logo=rust&logoColor=white)
![MIT License](https://img.shields.io/badge/License-MIT-blue?style=for-the-badge)

## 🌟 Features

- **Long-term memory** - Remembers past conversations using semantic search
- **Context awareness** - Maintains conversational context across messages
- **Freewill mode** - Bot can initiate conversations after periods of inactivity
- **Multiple LLM support** - Compatible with Gemini, OpenAI, Claude, and other providers
- **Docker ready** - Easy deployment with Docker and docker-compose
- **Custom roleplay guidelines** - Configurable personality and interaction styles
- **Temporal awareness** - Acknowledges time gaps between messages

## 📋 Prerequisites

- Rust (latest stable recommended)
- Discord bot token
- API keys for your preferred LLM provider (Gemini, OpenAI, etc.)
- Docker and docker-compose (optional, for containerized deployment)

## 🚀 Quick Start

### Installation

1. Clone the repository:

   ```bash
   git clone https://github.com/yourusername/chatbot.git
   cd chatbot
   ```

2. Copy the example config and customize it:

   ```bash
   cp config.example.toml config.toml
   # Edit config.toml with your API keys and preferences
   ```

3. Build the project:

   ```bash
   cargo build --release
   ```

### Supported LLM Providers

This project depends on my dependency [`rig-dyn`](https://github.com/GustavoWidman/rig-dyn) for dynamic LLM provider integration with the `rig` crate. Currently, the following LLM providers are supported:

- [`anthropic`](https://www.anthropic.com/)
- [`azure`](https://ai.azure.com/)
- [`cohere`](https://cohere.com/)
- [`deepseek`](https://deepseek.com/)
- [`galadriel`](https://galadriel.com/)
- [`gemini`](https://ai.google.dev/)
- [`groq`](https://groq.com/)
- [`huggingface`](https://huggingface.co/)
- [`hyperbolic`](https://hyperbolic.xyz/)
- [`mira`](https://mira.network/)
- [`moonshot`](https://www.moonshot.cn/)
- [`openai`](https://openai.com/)
- [`openrouter`](https://openrouter.ai/)
- [`ollama`](https://ollama.ai/)
- [`perplexity`](https://www.perplexity.ai/)
- [`xai`](https://x.ai/)

### Running

#### Using Cargo

```bash
cargo run --release
```

#### Using Docker

```bash
docker-compose up -d
```

## ⚙️ Configuration

The bot is configured via the `config.toml` file. Key configuration sections include:

### Discord Config

```toml
[config.discord]
token = "YOUR_DISCORD_BOT_TOKEN"
```

### LLM Config

```toml
[config.llm]
use_tools = true
force_lowercase = true
similarity_threshold = 0.5

[config.llm.completion]
model = "gemini-2.0-flash-thinking-exp-01-21"
provider = "gemini"
api_key = "YOUR_API_KEY"
```

### Memory Configuration

Configure memory storage parameters:

```toml
[config.llm.embedding]
model = "text-embedding-004"
provider = "gemini"
api_key = "YOUR_API_KEY"
qdrant_host = "127.0.0.1"
qdrant_port = 6334
qdrant_https = false
```

### Bot Personality

```toml
[config.context.system]
chatbot_name = "your_bot_name"
user_name = "default_user_name"
about = "Description of your bot's personality and background"
max_ltm = 100
age = "19 years old"
```

## 📚 Architecture

The bot consists of several key components:

- **Discord Integration**: Manages Discord events and message handling
- **Chat Engine**: Core conversational logic
- **Memory System**: Long-term and short-term memory management
  - Uses Qdrant vector database for semantic search
  - Stores conversation summaries for future recall
- **LLM Client**: Interfaces with various LLM providers

## 🔧 Commands

- `/clear` - Clear conversation history
- `/config` - Update configuration settings
- `/reload` - Reload the bot configuration

## 🤖 Memory Management

The bot uses a two-tiered memory system:

1. **Short-term memory (STM)**: Recent conversation history kept in active memory
2. **Long-term memory (LTM)**: Important information extracted and stored in a vector database for semantic search and recall

When the bot needs to recall information, it:

1. Embeds the current query using the embedding model
2. Searches the vector database for semantically similar memories
3. Incorporates relevant memories into its response context

## 🔄 Freewill Mode

The bot can initiate conversations after periods of inactivity:

- Configurable minimum and maximum wait times
- Probability increases exponentially with time since last interaction
- Summarizes conversation context before initiating new interactions

## 📝 License

This project is licensed under the MIT License - see the [LICENSE.txt](LICENSE.txt) file for details.

## 🙏 Acknowledgements

- Built with Rust, Serenity, and Qdrant
- Uses [rig](https://github.com/0xPlaygrounds/rig/) for LLM integration
