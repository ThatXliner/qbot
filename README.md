# QBot - Quiz Bowl Discord Bot

[![CI](https://github.com/ThatXliner/qbot/actions/workflows/ci.yml/badge.svg)](https://github.com/ThatXliner/qbot/actions/workflows/ci.yml)

A sophisticated Discord bot for quiz bowl question practice featuring an advanced query language for filtering questions by category. Built with Rust for high performance and reliability.

## 🎯 Features

- 🔍 **Smart Question Filtering**: Advanced query language with Boolean operations for precise question selection
- 👓 **Interactive Question Reading**: Real-time question reading with buzzing functionality
- 🧠 **AI-Powered Answer Checking**: Intelligent answer validation using LLM integration
- 📚 **Comprehensive Categories**: Support for all major quiz bowl categories and subcategories
- :zap: **Real-time Feedback**: Instant validation and prompting for incorrect answers
<!-- - **Multiple Question Support**: Read 1-10 questions in sequence with automatic transitions-->

## 🚀 Quick Start

Install the bot now here: https://discord.com/oauth2/authorize?client_id=1404873488312828066

### Prerequisites

- [Rust](https://rustup.rs/) (latest stable version)
- Discord bot token
- (Optional) Gemini for AI answer checking

### Installation

### With Docker

Simply just clone the repository:

```bash
git clone https://github.com/ThatXliner/qbot.git
cd qbot
```

Then run:
```bash
docker run -it --rm -e DISCORD_TOKEN=$DISCORD_TOKEN -e GEMINI_API_KEY=$GEMINI_API_KEY ghcr.io/thatxliner/qbot:main
```

### Manually
1. **Clone the repository**:
   ```bash
   git clone https://github.com/ThatXliner/qbot.git
   cd qbot
   ```

2. **Build the project**:
   ```bash
   cargo build --release
   ```

3. **Set up environment variables**:
   ```bash
   export DISCORD_TOKEN="your_discord_bot_token"
   export GEMINI_API_KEY="your_gemini_api_key"  # Optional, for AI features
   ```

4. **Run the bot**:
   ```bash
   cargo run --release
   ```

## 📖 Usage

### Basic Commands

- **`/tossup [query] [number]`** - Get quiz bowl questions
  - `query` (optional): Filter using [query language](#query-language-operators), otherwise pick from a random category
  - **Buzzing**: Message `buzz` during question reading to buzz in
  - **Answer Checking**: Type answers for AI-powered validation

- **`/categories [category]`** - Browse available categories
  - Without parameters: Shows all main categories
  - With category name: Shows subcategories

- **`/query <expression>`** - Test query language expressions

- **`/help [topic]`** - Get help about commands or topics

### Query Language Examples

```bash
/tossup query:Biology                    # Biology questions
/tossup query:Science + History          # Science OR History questions
/tossup query:Biology & Chemistry        # Questions tagged as both
/tossup query:Science - Math             # Science excluding Math
/tossup query:(Biology + Chemistry) - Math number:3  # 3 questions, Biology or Chemistry but no Math
```

## 🔧 Development

### Running Tests

```bash
# Run all tests (excluding integration tests that need external services)
cargo test -- --skip judge_tests

# Run specific test modules
cargo test utils_tests
cargo test qb_tests
cargo test query_tests

# Run with coverage
cargo tarpaulin --verbose --workspace --timeout 120 --skip-clean
```

### Code Quality

```bash
# Format code
cargo fmt

# Run linter
cargo clippy
```

### Development Dependencies

The project uses:
- **Discord Integration**: `poise` and `serenity` for Discord bot functionality
- **HTTP Client**: `reqwest` for QBReader API communication
- **Query Processing**: Custom recursive descent parser
- **AI Integration**: `llm` crate; Google Gemini
- **Async Runtime**: `tokio` for async/await support

### Project Structure

```
src/
├── main.rs           # Bot setup and Discord commands
├── query.rs          # Query language parser and processor
├── qb.rs            # QBReader API client and data structures
├── read.rs          # Interactive question reading logic
├── check.rs         # AI-powered answer validation
├── utils.rs         # Utility functions for text processing
└── *_tests.rs       # Comprehensive unit tests
```

## 🎮 Interactive Question Reading

When a question is being read:

1. **Question Progression**: Questions are read word-by-word in chunks
2. **Buzzing**: Type `buzz` to buzz in and attempt an answer
3. **Answer Submission**: Type your answer after buzzing
4. **AI Validation**: Answers are checked against the correct answer using LLM
5. **Feedback**: Get immediate feedback on correctness with explanations

## 📊 Categories

### Main Categories
- Literature (American, British, European, World)
- History (American, Ancient, European, World)
- Science (Biology, Chemistry, Physics, Math, Computer Science)
- Fine Arts (Visual, Auditory, Architecture, Film)
- Religion, Mythology, Philosophy
- Social Science, Current Events, Geography
- Other Academic, Pop Culture

### Query Language Operators

| Operator | Precedence | Description | Example |
|----------|------------|-------------|---------|
| `()`     | Highest    | Grouping    | `(Science + History)` |
| `-`      | High       | Exclusion   | `Science - Math` |
| `&`      | Medium     | Intersection| `Biology & Chemistry` |
| `+`      | Low        | Union       | `Science + History` |

## 🤝 Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

### Quick contribution checklist:
- [ ] Fork the repository
- [ ] Create a feature branch
- [ ] Add tests for new functionality
- [ ] Ensure all tests pass
- [ ] Follow Rust formatting conventions
- [ ] Submit a pull request

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- [QBReader](https://www.qbreader.org/) for providing the quiz bowl question database
- The Rust community for excellent crates and tooling
- Quiz bowl community for feedback and feature requests

## 🔗 Links

- [QBReader API Documentation](https://www.qbreader.org/api-docs)
- [Query Language Documentation](QUERY_LANGUAGE.md)
- [Discord Developer Portal](https://discord.com/developers/applications)
- [Get a Google Gemini API Key](https://ai.google.dev/gemini-api/docs/api-key)

---

**Made with ❤️ for the quiz bowl community**
