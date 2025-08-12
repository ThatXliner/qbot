/// Query language for filtering quiz bowl questions
///
/// The query language supports filtering questions by categories and subcategories using
/// Boolean operators with proper precedence and associativity.
///
/// # Supported Operators (in order of precedence, highest first):
/// - Parentheses `()` - grouping expressions
/// - Minus `-` - subtraction/exclusion (left-associative)
/// - And `&` - intersection (left-associative)
/// - Or `+` - union (left-associative)
///
/// # Category Matching:
/// - Categories are matched case-insensitively with automatic capitalization
/// - Multi-word categories are supported (e.g., "American Literature")
/// - Subcategories within the same main category can be combined
/// - Alternate subcategories are handled automatically
///
/// # Examples:
/// - `Biology` - All biology questions
/// - `Science + History` - All science OR history questions
/// - `Biology & Chemistry` - All questions that are both biology AND chemistry
/// - `Science - Math` - All science questions EXCEPT math questions
/// - `(Biology + Chemistry) - Math` - Biology or chemistry questions, but exclude math
/// - `Science & (Biology + Chemistry)` - Science questions that are biology or chemistry
///
/// # Error Handling:
/// - Invalid categories are rejected with helpful error messages
/// - Impossible queries (e.g., conflicting categories) are detected
/// - Syntax errors provide context about unexpected tokens
///
/// I need to fix subtraction though; for how it's currently implemented, it's fundamentaly broken
use phf::phf_map;
use rapidfuzz::distance::levenshtein;
use std::{collections::VecDeque, fmt};
use tracing::debug;

/// category -> (subcategories, alternate subcategories)
/// TODO: aliases
pub static CATEGORIES: phf::Map<&'static str, (&'static [&'static str], &'static [&'static str])> = phf_map! {
    "Literature" => (&[
        "American Literature", "British Literature", "Classical Literature",
        "European Literature", "World Literature", "Other Literature"
    ], &[
        "Drama", "Long Fiction", "Poetry", "Short Fiction", "Misc Literature"
    ]),
    "History" => (&[
        "American History", "Ancient History", "European History",
        "World History", "Other History"
    ], &[]),
    "Science" => (&[
        "Biology", "Chemistry", "Physics", "Other Science"
    ], &[
        "Math", "Astronomy", "Computer Science", "Earth Science", "Engineering", "Misc Science"
    ]),
    "Fine Arts" => (&[
        "Visual Fine Arts", "Auditory Fine Arts", "Other Fine Arts"
    ], &[
        "Architecture", "Dance", "Film", "Jazz", "Musicals", "Opera", "Photography", "Misc Arts"
    ]),
    "Religion" => (&[], &[]),
    "Mythology" => (&[], &[]),
    "Philosophy" => (&[], &[]),
    "Social Science" => (&[], &[
        "Anthropology", "Economics", "Linguistics", "Psychology", "Sociology", "Other Social Science"
    ]),
    "Current Events" => (&[], &[]),
    "Geography" => (&[], &[]),
    "Other Academic" => (&[], &[]),
    "Pop Culture" => (&[
        "Movies", "Music", "Sports", "Television", "Video Games", "Other Pop Culture"
    ], &[]),
};

/// Abstract Syntax Tree for the query language
///
/// Represents the parsed structure of a query with proper operator precedence.
/// The tree is evaluated bottom-up to produce category filters for the API.
#[derive(Debug, Clone)]
pub enum Expr {
    /// A category or subcategory name (e.g., "Biology", "American Literature")
    Token(String),
    /// Logical AND - intersection of two expressions (higher precedence than OR)
    And(Box<Expr>, Box<Expr>),
    /// Logical OR - union of two expressions (lowest precedence)
    Or(Box<Expr>, Box<Expr>),
    /// Logical NOT - subtraction/exclusion of second expression from first (highest precedence after parentheses)
    Not(Box<Expr>, Box<Expr>), // A - B
}

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Expr::Token(t) => write!(f, "{}", t),
            Expr::And(a, b) => write!(f, "({} & {})", a, b),
            Expr::Or(a, b) => write!(f, "({} + {})", a, b),
            Expr::Not(a, b) => write!(f, "({} - {})", a, b),
        }
    }
}

/// Result after validation, ready for API consumption
///
/// This structure maps the logical query to the specific API parameters
/// needed by the QBReader API for filtering questions.
#[derive(Debug, PartialEq)]
pub struct ApiQuery {
    /// Main categories to include (e.g., ["Science", "History"])
    pub categories: Vec<String>,
    /// Specific subcategories to include (e.g., ["Biology", "Chemistry"])
    pub subcategories: Vec<String>,
    /// Alternate subcategories to include (e.g., ["Math", "Computer Science"])
    pub alternate_subcategories: Vec<String>,
    /// Number of questions to retrieve
    pub number: u32,
}

impl Default for ApiQuery {
    fn default() -> Self {
        ApiQuery {
            categories: Vec::new(),
            subcategories: Vec::new(),
            alternate_subcategories: Vec::new(),
            number: 1,
        }
    }
}

/// Errors that can occur during query parsing and validation
#[derive(Debug)]
pub enum QueryError {
    // Parse errors
    /// Unexpected token encountered during parsing (e.g., operator in wrong position)
    UnexpectedToken(String),
    /// Input ended unexpectedly (e.g., unclosed parentheses)
    UnexpectedEOF,
    // Validation errors
    /// Category or subcategory name not found in the known categories
    InvalidCategory(String),
    /// Query results in impossible constraints (e.g., "Biology & History")
    ImpossibleBranch(String),
}

/// Tokenize input string into operators and category names
///
/// Handles multi-word categories by preserving spaces until operators are encountered.
/// Operators: &, +, -, (, )
///
/// # Examples:
/// - `"Biology + Chemistry"` → `["Biology", "+", "Chemistry"]`
/// - `"American Literature & History"` → `["American", "Literature", "&", "History"]`
#[inline]
pub fn tokenize(input: &str) -> VecDeque<String> {
    let mut tokens = VecDeque::new();
    let mut buf = String::new();
    for c in input.chars() {
        match c {
            '&' | '+' | '-' | '(' | ')' => {
                if !buf.trim().is_empty() {
                    tokens.push_back(buf.trim().to_string());
                    buf.clear();
                }
                tokens.push_back(c.to_string());
            }
            ' ' => {
                if !buf.trim().is_empty() {
                    tokens.push_back(buf.trim().to_string());
                    buf.clear();
                }
            }
            _ => buf.push(c),
        }
    }
    if !buf.trim().is_empty() {
        tokens.push_back(buf.trim().to_string());
    }
    tokens
}

/// Parse a complete expression and ensure all tokens are consumed
///
/// This is the main entry point for parsing. It expects the entire input
/// to be a valid expression with no leftover tokens.
pub fn parse_expr(tokens: &mut VecDeque<String>) -> Result<Expr, QueryError> {
    let result = parse_or(tokens);
    if !tokens.is_empty() {
        Err(QueryError::UnexpectedToken(format!(
            "Unexpected tokens: {:?}",
            tokens
        )))
    } else {
        result
    }
}

/// Parse a sub-expression without requiring all tokens to be consumed
///
/// Used for parsing expressions inside parentheses where there may be
/// more tokens after the closing parenthesis.
fn parse_subexpr(tokens: &mut VecDeque<String>) -> Result<Expr, QueryError> {
    parse_or(tokens)
}

/// Parse OR expressions (lowest precedence)
///
/// Handles left-associative OR operations. Multiple OR operators
/// are parsed left-to-right: `A + B + C` becomes `(A + B) + C`
fn parse_or(tokens: &mut VecDeque<String>) -> Result<Expr, QueryError> {
    let mut node = parse_and(tokens)?;
    while let Some(tok) = tokens.front() {
        if tok == "+" {
            tokens.pop_front();
            let rhs = parse_and(tokens)?;
            node = Expr::Or(Box::new(node), Box::new(rhs));
        } else {
            break;
        }
    }

    Ok(node)
}

/// Parse AND expressions (medium precedence)
///
/// Handles left-associative AND operations. Multiple AND operators
/// are parsed left-to-right: `A & B & C` becomes `(A & B) & C`
fn parse_and(tokens: &mut VecDeque<String>) -> Result<Expr, QueryError> {
    let mut node = parse_not(tokens)?;
    while let Some(tok) = tokens.front() {
        if tok == "&" {
            tokens.pop_front();
            let rhs = parse_not(tokens)?;
            node = Expr::And(Box::new(node), Box::new(rhs));
        } else {
            break;
        }
    }

    Ok(node)
}

/// Parse NOT/minus expressions (highest precedence after parentheses)
///
/// Handles left-associative subtraction operations. Multiple minus operators
/// are parsed left-to-right: `A - B - C` becomes `(A - B) - C`
fn parse_not(tokens: &mut VecDeque<String>) -> Result<Expr, QueryError> {
    let mut node = parse_primary(tokens)?;
    while let Some(tok) = tokens.front() {
        if tok == "-" {
            tokens.pop_front();
            let rhs = parse_primary(tokens)?;
            node = Expr::Not(Box::new(node), Box::new(rhs));
        } else {
            break;
        }
    }

    Ok(node)
}

/// Parse primary expressions (categories, subcategories, and parenthesized expressions)
///
/// Handles:
/// - Category/subcategory names (including multi-word like "American Literature")
/// - Parenthesized sub-expressions
/// - Error detection for unexpected operators
fn parse_primary(tokens: &mut VecDeque<String>) -> Result<Expr, QueryError> {
    if let Some(tok) = tokens.pop_front() {
        match tok.as_str() {
            "(" => {
                let expr = parse_subexpr(tokens)?;
                let Some(next_token) = tokens.front() else {
                    return Err(QueryError::UnexpectedEOF);
                };
                if next_token != ")" {
                    return Err(QueryError::UnexpectedToken(format!(
                        "Unexpected token {:?}, expected ')'",
                        next_token
                    )));
                }

                tokens.pop_front();
                Ok(expr)
            }
            // We shouldn't be seeing punctuation here...
            "&" | "+" | "-" | ")" => Err(QueryError::UnexpectedToken(tok)),
            _ => {
                let mut buf = vec![tok];
                // The reason why we have this loop is so we can have support for multi-word categories
                loop {
                    let Some(c) = tokens.front() else { break };
                    match c.as_str() {
                        "&" | "+" | "-" | "(" | ")" => {
                            break;
                        }
                        _ => {
                            buf.push(tokens.pop_front().unwrap());
                        }
                    }
                }
                Ok(Expr::Token(buf.join(" ")))
            }
        }
    } else {
        Err(QueryError::UnexpectedEOF)
    }
}
const FUZZY_THRESHOLD: usize = 3;

fn match_against(
    comparator: &levenshtein::BatchComparator<char>,
    b: &Vec<String>,
) -> Option<String> {
    let mut distances: Vec<(String, usize)> = b
        .into_iter()
        .map(|item| {
            (
                item.clone(),
                comparator.distance(item.to_lowercase().chars()),
            )
        })
        .collect();

    distances.sort_by_key(|(_, dist)| *dist);

    let close_matches: Vec<String> = distances
        .iter()
        .filter(|(_, dist)| *dist < FUZZY_THRESHOLD)
        .map(|(item, _)| item.clone())
        .collect();

    if close_matches.len() == 1 {
        Some(close_matches[0].clone())
    } else {
        None
    }
}
/// Validate recursively
/// Validate and convert an expression tree to API query parameters
///
/// Recursively processes the AST and maps category/subcategory names to the
/// appropriate API parameters. Handles the complex logic of:
/// - Category name resolution and validation
/// - Operator semantics (AND, OR, NOT)
/// - Subcategory and alternate subcategory mapping
/// - Error detection for impossible queries
fn validate(expr: &Expr) -> Result<(Vec<String>, Vec<String>, Vec<String>), QueryError> {
    match expr {
        Expr::Token(t) => {
            let comparator = levenshtein::BatchComparator::new(t.to_lowercase().chars());
            for (key, value) in CATEGORIES.entries() {
                // Check if it's a main category (e.g., "Science")
                if comparator.distance(key.to_lowercase().chars()) < FUZZY_THRESHOLD {
                    return Ok((
                        vec![key.to_string()],
                        value.0.into_iter().map(|s| s.to_string()).collect(),
                        value.1.into_iter().map(|s| s.to_string()).collect(),
                    ));
                }
                // Check if it's a regular subcategory (e.g., "Biology" -> Science/Biology)
                if let Some(result) = match_against(
                    &comparator,
                    &value.0.into_iter().map(|x| x.to_string()).collect(),
                ) {
                    return Ok((vec![key.to_string()], vec![result], vec![]));
                }
                // Check if it's an alternate subcategory (e.g., "Math" -> Science/Other Science/Math)
                if let Some(result) = match_against(
                    &comparator,
                    &value.1.into_iter().map(|x| x.to_string()).collect(),
                ) {
                    let misc_category = format!("Other {}", key);
                    assert!(value.0.contains(&misc_category.as_str()));
                    return Ok((vec![key.to_string()], vec![misc_category], vec![result]));
                }
            }
            Err(QueryError::InvalidCategory(t.clone()))
        }
        Expr::And(a, b) => {
            let (ac, asub, aalt) = validate(a)?;
            let (bc, bsub, balt) = validate(b)?;
            let cc: Vec<_> = ac.iter().filter(|x| bc.contains(x)).cloned().collect();
            // Check if categories intersect - if not, it's impossible
            if cc.is_empty() {
                return Err(QueryError::ImpossibleBranch(format!("{} & {}", a, b)));
            }

            // For AND operation: prefer more specific constraints when one side is general
            // and the other is specific. When both are specific, combine them.
            let left_has_specifics = !asub.is_empty() || !aalt.is_empty();
            let right_has_specifics = !bsub.is_empty() || !balt.is_empty();

            let (result_subs, result_alts) = match (left_has_specifics, right_has_specifics) {
                (false, true) => {
                    // Left is general (full category), right is specific - use right
                    (bsub, balt)
                }
                (true, false) => {
                    // Right is general (full category), left is specific - use left
                    (asub, aalt)
                }
                (true, true) => {
                    // Both are specific - combine them (e.g., "Biology & Chemistry")
                    let mut combined_subs = asub;
                    combined_subs.extend(bsub);
                    combined_subs.sort();
                    combined_subs.dedup();

                    let mut combined_alts = aalt;
                    combined_alts.extend(balt);
                    combined_alts.sort();
                    combined_alts.dedup();

                    (combined_subs, combined_alts)
                }
                (false, false) => {
                    // Both are general categories - this case is handled by category intersection
                    (vec![], vec![])
                }
            };

            Ok((cc, result_subs, result_alts))
        }
        Expr::Or(a, b) => {
            let (mut ac, mut asub, mut aalt) = validate(a)?;
            let (bc, bsub, balt) = validate(b)?;
            // Union operation: combine all categories, subcategories, and alternates
            ac.extend(bc);
            asub.extend(bsub);
            aalt.extend(balt);
            // Remove duplicates
            ac.sort();
            ac.dedup();
            asub.sort();
            asub.dedup();
            aalt.sort();
            aalt.dedup();
            Ok((ac, asub, aalt))
        }
        // The minus operator subtracts the second operand from the first
        // Key insight: we need to be smart about what level to subtract at
        // - If both sides resolve to the same category, subtract at subcategory/alternate level
        // - If they resolve to different categories, subtract at category level
        Expr::Not(a, b) => {
            let (mut ac, mut asub, mut aalt) = validate(a)?;
            let (bc, bsub, balt) = validate(b)?;

            // Check if we're subtracting within the same category
            let common_categories: Vec<_> = ac.iter().filter(|x| bc.contains(x)).cloned().collect();

            if !common_categories.is_empty() {
                // We're in the same category, so subtract subcategories and alternates
                asub.retain(|x| !bsub.contains(x));
                aalt.retain(|x| !balt.contains(x));
                // Keep the common categories
                ac = common_categories;
            } else {
                // Different categories, subtract entire categories
                ac.retain(|x| !bc.contains(x));
                // If we removed all categories, this is an impossible branch
                if ac.is_empty() {
                    return Err(QueryError::ImpossibleBranch(format!("{} - {}", a, b)));
                }
            }

            // If we have no specific subcategories left but still have categories,
            // we need to include all subcategories of remaining categories
            if ac.is_empty() && asub.is_empty() && aalt.is_empty() {
                return Err(QueryError::ImpossibleBranch(format!("{} - {}", a, b)));
            }
            Ok((ac, asub, aalt))
        }
    }
}

/// Convert a token to proper capitalization for category matching
///
/// Each word is capitalized (first letter uppercase, rest lowercase)
/// to match the category naming convention in CATEGORIES.
///
/// # Examples:
/// - `"biology"` → `"Biology"`
/// - `"american literature"` → `"American Literature"`
#[inline]
fn capitalize_token(token: &str) -> String {
    token
        .split_whitespace()
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Build the final API query from a validated expression tree
///
/// This is the final step that converts the validated expression results
/// into the ApiQuery structure used by the QBReader API.
fn build_api_query(expr: &Expr) -> Result<ApiQuery, QueryError> {
    let (cats, subs, alts) = validate(expr)?;
    debug!("Debug normalized expression: {}", expr);
    debug!("Categories: {:?}", cats);
    debug!("Subcategories: {:?}", subs);
    debug!("Alternate subcategories: {:?}", alts);
    Ok(ApiQuery {
        categories: if cats.is_empty() { vec![] } else { cats },
        subcategories: if subs.is_empty() { vec![] } else { subs },
        alternate_subcategories: if alts.is_empty() { vec![] } else { alts },
    })
}

/// Parse a query string into API parameters
///
/// This is the main public interface for the query language. It takes a query string
/// and returns either an ApiQuery ready for the QBReader API, or a QueryError
/// describing what went wrong.
///
/// # Arguments
/// * `query_str` - The query string to parse (e.g., "Biology + Chemistry - Math")
///
/// # Returns
/// * `Ok(ApiQuery)` - Successfully parsed query ready for API use
/// * `Err(QueryError)` - Parsing or validation error with details
///
/// # Examples
/// ```rust
/// let result = parse_query("Biology + Chemistry");
/// assert!(result.is_ok());
///
/// let result = parse_query("InvalidCategory");
/// assert!(result.is_err());
/// ```
pub fn parse_query(query_str: &str) -> Result<ApiQuery, QueryError> {
    let mut tokens = tokenize(query_str);
    parse_expr(&mut tokens).and_then(|expr| build_api_query(&expr))
}
#[cfg(test)]
mod tests {
    use super::*;

    fn q(s: &str) -> Result<ApiQuery, QueryError> {
        parse_query(s)
    }

    #[test]
    fn single_category() {
        let r = q("Science").unwrap();
        assert!(r.categories.contains(&"Science".to_string()));
        assert!(r.subcategories.contains(&"Biology".to_string()));
    }

    #[test]
    fn single_subcategory() {
        let r = q("Biology").unwrap();
        assert_eq!(r.categories, vec!["Science"]);
        assert_eq!(r.subcategories, vec!["Biology"]);
    }

    #[test]
    fn alternate_subcategory() {
        let r = q("Math").unwrap();
        assert_eq!(r.categories, vec!["Science"]);
        assert_eq!(r.subcategories, vec!["Other Science"]);
        assert_eq!(r.alternate_subcategories, vec!["Math"]);
    }

    #[test]
    fn multi_word_category() {
        let r = q("American Literature").unwrap();
        assert_eq!(r.categories, vec!["Literature"]);
        assert_eq!(r.subcategories, vec!["American Literature"]);
    }

    #[test]
    fn and_operator_same_category() {
        let r = q("Biology & Chemistry").unwrap();
        assert_eq!(r.categories, vec!["Science"]);
        assert!(r.subcategories.contains(&"Biology".to_string()), "{:?}", r);
        assert!(r.subcategories.contains(&"Chemistry".to_string()));
    }

    #[test]
    fn and_operator_different_category_impossible() {
        let r = q("Biology & History");
        assert!(matches!(r, Err(QueryError::ImpossibleBranch(_))));
    }

    #[test]
    fn or_operator() {
        let r = q("Biology + History").unwrap();
        assert!(r.categories.contains(&"Science".to_string()));
        assert!(r.categories.contains(&"History".to_string()));
    }

    #[test]
    fn not_operator_removes_subcategory() {
        let r = q("Science - Math").unwrap();
        assert!(r.categories.contains(&"Science".to_string()), "{:?}", r);
        assert!(!r.alternate_subcategories.contains(&"Math".to_string()));
    }

    #[test]
    fn parentheses_override_precedence() {
        let r = q("Science & (Biology + Chemistry)").unwrap();
        assert_eq!(r.categories, vec!["Science"]);
        assert!(r.subcategories.contains(&"Biology".to_string()));
        assert!(r.subcategories.contains(&"Chemistry".to_string()));
    }

    #[test]
    fn unexpected_token_error() {
        let r = q("& Science");
        assert!(matches!(r, Err(QueryError::UnexpectedToken(_))));
    }

    #[test]
    fn unexpected_eof_error() {
        let mut tokens = tokenize("(");
        let r = parse_expr(&mut tokens);
        assert!(matches!(r, Err(QueryError::UnexpectedEOF)));
    }

    #[test]
    fn invalid_category_error() {
        let r = q("MadeUpCategory");
        assert!(matches!(r, Err(QueryError::InvalidCategory(_))));
    }

    #[test]
    fn lowercase_and_spacing() {
        let r = q("  biology  +   history  ").unwrap();
        assert!(r.categories.contains(&"Science".to_string()));
        assert!(r.categories.contains(&"History".to_string()));
    }

    #[test]
    fn complex_minus_operator_subcategory_subtraction() {
        // Test that "Science - Biology" removes Biology but keeps other Science subcategories
        let r = q("Science - Biology").unwrap();
        assert_eq!(r.categories, vec!["Science"]);
        assert!(!r.subcategories.contains(&"Biology".to_string()));
        assert!(r.subcategories.contains(&"Chemistry".to_string()));
        assert!(r.subcategories.contains(&"Physics".to_string()));
    }

    #[test]
    fn complex_minus_operator_alternate_subtraction() {
        // Test that "Science - Math" removes Math from alternate subcategories
        let r = q("Science - Math").unwrap();
        assert_eq!(r.categories, vec!["Science"]);
        assert!(!r.alternate_subcategories.contains(&"Math".to_string()));
        // Should still contain other Science subcategories and alternates
        assert!(r.subcategories.contains(&"Biology".to_string()));
        assert!(
            r.alternate_subcategories
                .contains(&"Computer Science".to_string())
        );
    }

    #[test]
    fn nested_parentheses() {
        // A more realistic test: Biology OR Chemistry, but excluding Math subjects
        let r = q("(Biology + Chemistry) - Math").unwrap();
        println!("(Biology + Chemistry) - Math: {:?}", r);
        assert_eq!(r.categories, vec!["Science"]);
        assert!(r.subcategories.contains(&"Biology".to_string()));
        assert!(r.subcategories.contains(&"Chemistry".to_string()));
        // Math should be excluded since we're doing (Biology + Chemistry) - Math
        assert!(!r.alternate_subcategories.contains(&"Math".to_string()));
    }

    #[test]
    fn multiple_and_operators() {
        let r = q("Science & Biology & Chemistry").unwrap();
        assert_eq!(r.categories, vec!["Science"]);
        assert!(r.subcategories.contains(&"Biology".to_string()));
        assert!(r.subcategories.contains(&"Chemistry".to_string()));
    }

    #[test]
    fn multiple_or_operators() {
        let r = q("Biology + Chemistry + Physics").unwrap();
        assert_eq!(r.categories, vec!["Science"]);
        assert!(r.subcategories.contains(&"Biology".to_string()));
        assert!(r.subcategories.contains(&"Chemistry".to_string()));
        assert!(r.subcategories.contains(&"Physics".to_string()));
    }

    #[test]
    fn multiple_minus_operators() {
        let r = q("Science - Math - Computer Science").unwrap();
        assert_eq!(r.categories, vec!["Science"]);
        assert!(!r.alternate_subcategories.contains(&"Math".to_string()));
        assert!(
            !r.alternate_subcategories
                .contains(&"Computer Science".to_string())
        );
        assert!(r.alternate_subcategories.contains(&"Astronomy".to_string()));
    }
}
