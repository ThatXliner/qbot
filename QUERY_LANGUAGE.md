# Query Language Documentation

The QBot query language allows you to filter quiz bowl questions using Boolean expressions with categories and subcategories.

## Basic Syntax

### Categories and Subcategories

Query terms can be:
- **Main categories**: `Science`, `History`, `Literature`, `Fine Arts`, etc.
- **Subcategories**: `Biology`, `American History`, `Poetry`, etc.
- **Multi-word terms**: `"American Literature"`, `"Computer Science"`, etc.

Category matching is case-insensitive and automatically capitalized.

### Operators

The query language supports three binary operators with the following precedence (highest to lowest):

1. **Minus (`-`)** - Subtraction/Exclusion
2. **And (`&`)** - Intersection  
3. **Or (`+`)** - Union

All operators are left-associative, meaning `A + B + C` is parsed as `(A + B) + C`.

### Parentheses

Use parentheses `()` to override operator precedence and group expressions.

## Operator Semantics

### Union (`+`)
Combines questions from multiple categories or subcategories.

**Examples:**
- `Biology + Chemistry` - Questions from Biology OR Chemistry
- `Science + History` - Questions from Science OR History  
- `Math + Physics + Biology` - Questions from any of these three areas

### Intersection (`&`)
Finds questions that match multiple criteria. This is most useful when combining:
- Specific subcategories within the same category
- A general category with specific constraints

**Examples:**
- `Biology & Chemistry` - Questions that are categorized as both Biology AND Chemistry
- `Science & (Biology + Chemistry)` - Science questions that are specifically Biology or Chemistry

### Subtraction (`-`)
Excludes questions from the second term from the first term.

**Examples:**
- `Science - Math` - All Science questions EXCEPT Math questions
- `Biology - Genetics` - Biology questions excluding Genetics topics
- `(Biology + Chemistry) - Math` - Biology or Chemistry questions, but exclude any Math overlap

## Complex Examples

### Nested Operations
```
Science & ((Biology + Chemistry) - Math)
```
Science questions that are either Biology or Chemistry, but excluding Math questions.

### Multiple Operators
```
Literature + History - "Current Events"
```
Literature or History questions, but excluding Current Events.

### Category Refinement
```
Science & Biology + Math
```
Due to precedence, this is parsed as `(Science & Biology) + Math`, giving you Biology questions plus all Math questions.

To get Science questions that are Biology or Math, use:
```
Science & (Biology + Math)
```

## Available Categories

### Main Categories
- Literature
- History  
- Science
- Fine Arts
- Religion
- Mythology
- Philosophy
- Social Science
- Current Events
- Geography
- Other Academic
- Pop Culture

### Example Subcategories

**Literature:**
- American Literature, British Literature, European Literature
- Drama, Poetry, Short Fiction, Long Fiction

**Science:**
- Biology, Chemistry, Physics, Other Science
- Math, Astronomy, Computer Science, Earth Science, Engineering

**History:**
- American History, Ancient History, European History, World History

**Fine Arts:**
- Visual Fine Arts, Auditory Fine Arts
- Architecture, Dance, Film, Photography

## Error Handling

The parser will return helpful error messages for:

- **Invalid Categories**: `"InvalidCategory" is not a recognized category`
- **Syntax Errors**: `Unexpected token "+" at beginning of expression`
- **Impossible Queries**: `"Biology & History" results in impossible constraints`
- **Unclosed Parentheses**: `"(" without matching ")"`

## Usage Tips

1. **Use quotes for multi-word categories** if your interface supports them, or just use spaces
2. **Combine specific subcategories** within the same main category using `&`
3. **Use parentheses liberally** to make complex queries clear
4. **Test simple queries first** before building complex expressions
5. **Remember operator precedence**: `-` binds tighter than `&`, which binds tighter than `+`

## Implementation Notes

The query language is implemented using a recursive descent parser with proper operator precedence. The parser:

1. **Tokenizes** the input into categories and operators
2. **Parses** according to operator precedence rules
3. **Validates** category names against the known category database
4. **Resolves** subcategory mappings and alternate subcategories
5. **Generates** API parameters for the QBReader service

The resulting queries are optimized for the QBReader API's category filtering system.