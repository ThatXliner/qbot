# Contributing to QBot

Thank you for your interest in contributing to QBot! This document provides guidelines and information for contributors.

## 📋 Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Setup](#development-setup)
- [Making Contributions](#making-contributions)
- [Coding Standards](#coding-standards)
- [Testing](#testing)
- [Pull Request Process](#pull-request-process)
- [Issue Guidelines](#issue-guidelines)

## 🤝 Code of Conduct

We are committed to providing a welcoming and inclusive environment for all contributors. Please be respectful and constructive in all interactions.

### Our Standards

- Use welcoming and inclusive language
- Be respectful of differing viewpoints and experiences
- Gracefully accept constructive criticism
- Focus on what is best for the community
- Show empathy towards other community members

## 🚀 Getting Started

### Prerequisites

- [Rust](https://rustup.rs/) (latest stable version)
- Git
- A Discord application/bot token for testing
- (Optional) Ollama server for AI features

### Fork and Clone

1. Fork the repository on GitHub
2. Clone your fork locally:
   ```bash
   git clone https://github.com/YOUR_USERNAME/qbot.git
   cd qbot
   ```
3. Add the upstream repository:
   ```bash
   git remote add upstream https://github.com/ThatXliner/qbot.git
   ```

## 💻 Development Setup

1. **Install dependencies**:
   ```bash
   cargo build
   ```

2. **Set up environment variables**:
   ```bash
   export DISCORD_TOKEN="your_test_bot_token"
   export OLLAMA_URL="http://127.0.0.1:11434"  # Optional
   ```

3. **Run tests**:
   ```bash
   cargo test -- --skip judge_tests
   ```

4. **Start the bot**:
   ```bash
   cargo run
   ```

## 🛠 Making Contributions

### Types of Contributions

We welcome various types of contributions:

- **Bug fixes** - Fix issues and improve stability
- **Features** - Add new functionality and capabilities
- **Documentation** - Improve docs, examples, and guides
- **Tests** - Add test coverage and improve reliability
- **Performance** - Optimize code and reduce resource usage
- **Query Language** - Enhance the query parser and add features

### Contribution Workflow

1. **Check existing issues** - Look for existing issues or create a new one
2. **Create a branch** - Create a feature branch from `main`
3. **Make changes** - Implement your contribution
4. **Add tests** - Ensure your changes are well-tested
5. **Test thoroughly** - Run all tests and verify functionality
6. **Submit PR** - Create a pull request with a clear description

## 📝 Coding Standards

### Rust Style Guide

We follow standard Rust conventions:

- Use `cargo fmt` for consistent formatting
- Follow `cargo clippy` recommendations
- Use meaningful variable and function names
- Add documentation for public APIs
- Prefer explicit error handling over panics

### Code Organization

- Keep functions focused and single-purpose
- Group related functionality in modules
- Use appropriate visibility (`pub`, `pub(crate)`, private)
- Add comprehensive tests for new features

### Documentation

- Document all public functions and structs
- Include examples in documentation
- Update README.md for user-facing changes
- Keep comments concise and helpful

### Commit Messages

Write clear, descriptive commit messages:

```
feat(query): add support for regex patterns in queries

- Implement regex token parsing
- Add tests for regex functionality
- Update query language documentation

Closes #123
```

**Format**: `<type>(<scope>): <description>`

**Types**: `feat`, `fix`, `docs`, `test`, `refactor`, `perf`, `chore`

## 🧪 Testing

### Running Tests

```bash
# Run all tests (excluding external service tests)
cargo test -- --skip judge_tests

# Run specific test suites
cargo test utils_tests     # Utility function tests
cargo test qb_tests        # API client tests
cargo test query_tests     # Query language tests

# Run with coverage
cargo tarpaulin --verbose --workspace --timeout 120 --skip-clean
```

### Writing Tests

- Add unit tests for all new functions
- Include edge cases and error scenarios
- Use descriptive test names
- Mock external dependencies when possible
- Avoid tests that require network access or external services

### Test Structure

```rust
#[cfg(test)]
mod tests {
    use crate::module::*;

    #[test]
    fn test_function_name_behavior() {
        // Arrange
        let input = "test input";
        
        // Act
        let result = function_name(input);
        
        // Assert
        assert_eq!(result, expected_output);
    }
}
```

## 🔄 Pull Request Process

### Before Submitting

- [ ] Ensure all tests pass locally
- [ ] Run `cargo fmt` and `cargo clippy`
- [ ] Update documentation if needed
- [ ] Add tests for new functionality
- [ ] Check that CI will pass

### PR Description Template

```markdown
## Description
Brief description of changes

## Type of Change
- [ ] Bug fix
- [ ] New feature
- [ ] Documentation update
- [ ] Performance improvement
- [ ] Other (please describe)

## Testing
- [ ] All tests pass
- [ ] New tests added for new functionality
- [ ] Manual testing completed

## Checklist
- [ ] Code follows project style guidelines
- [ ] Self-review completed
- [ ] Documentation updated
- [ ] No breaking changes (or clearly documented)
```

### Review Process

1. **Automated checks** - CI must pass
2. **Code review** - At least one maintainer review
3. **Testing** - Verify functionality works as expected
4. **Documentation** - Ensure docs are updated
5. **Merge** - Squash and merge when approved

## 🐛 Issue Guidelines

### Reporting Bugs

Include the following information:

- **Environment**: OS, Rust version, dependencies
- **Steps to reproduce**: Clear, minimal reproduction steps
- **Expected behavior**: What should happen
- **Actual behavior**: What actually happens
- **Logs/errors**: Any relevant error messages
- **Additional context**: Screenshots, configurations, etc.

### Feature Requests

For new features, please include:

- **Use case**: Why is this feature needed?
- **Proposed solution**: How should it work?
- **Alternatives**: Other solutions considered
- **Additional context**: Examples, mockups, references

### Bug Report Template

```markdown
**Environment**
- OS: [e.g., Ubuntu 20.04]
- Rust version: [e.g., 1.70.0]
- QBot version: [e.g., v0.1.0]

**Description**
A clear description of the bug.

**To Reproduce**
1. Run command '...'
2. Enter query '...'
3. See error

**Expected Behavior**
What you expected to happen.

**Actual Behavior**
What actually happened.

**Logs**
```
Paste any relevant logs here
```

**Additional Context**
Any other context about the problem.
```

## 🎯 Areas Needing Help

We especially welcome contributions in these areas:

- **Query Language**: More operators, functions, and features
- **Performance**: Optimization and efficiency improvements  
- **Testing**: Increased test coverage and integration tests
- **Documentation**: Examples, tutorials, and API docs
- **Discord Features**: New commands and interactive features
- **AI Integration**: Improved answer checking and prompting

## 📚 Resources

### Learning Resources

- [Rust Book](https://doc.rust-lang.org/book/)
- [Cargo Book](https://doc.rust-lang.org/cargo/)
- [Discord API Documentation](https://discord.com/developers/docs)
- [Poise Framework Guide](https://docs.rs/poise/latest/poise/)

### Project Resources

- [QBReader API](https://www.qbreader.org/api-docs)
- [Query Language Docs](QUERY_LANGUAGE.md)
- [Architecture Overview](docs/architecture.md) (coming soon)

## 💬 Getting Help

If you need help with your contribution:

- Open a draft pull request for early feedback
- Ask questions in issue comments
- Reference existing similar code
- Check the documentation and examples

## 🙏 Recognition

Contributors will be:

- Listed in the project README
- Credited in release notes
- Given maintainer access for significant contributions
- Recognized in the Discord community

---

Thank you for contributing to QBot! Your efforts help make quiz bowl practice better for everyone. 🎉