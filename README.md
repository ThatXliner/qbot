# Quiz Bowl bot

A Discord bot for quiz bowl question practice with advanced query language support.

## Features

- Random quiz bowl questions from QBReader
- Advanced query language for filtering questions by category
- Interactive question reading with buzzing functionality
- Support for complex Boolean queries with proper operator precedence

## Query Language

The bot supports a powerful query language for filtering questions. See [QUERY_LANGUAGE.md](QUERY_LANGUAGE.md) for full documentation.

**Quick examples:**
- `Biology` - All biology questions
- `Science + History` - Science OR history questions
- `Biology & Chemistry` - Questions tagged as both biology AND chemistry
- `Science - Math` - Science questions excluding math
- `(Biology + Chemistry) - Math` - Biology or chemistry, but no math

## Usage

Use the `/tossup` command with an optional query parameter to get filtered questions.
