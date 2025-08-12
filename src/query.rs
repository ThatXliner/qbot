/// Query language
/// I need to fix subtraction though; for how it's currently implemented, it's fundamentaly broken
use phf::phf_map;
use rapidfuzz::distance::levenshtein;
use std::{collections::VecDeque, fmt};
use tracing::{debug, info};

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

/// AST for query language
#[derive(Debug, Clone)]
pub enum Expr {
    Token(String),
    And(Box<Expr>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
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

/// Result after validation, ready for API
#[derive(Debug, Default, PartialEq)]
pub struct ApiQuery {
    pub categories: Vec<String>,
    pub subcategories: Vec<String>,
    pub alternate_subcategories: Vec<String>,
}

#[derive(Debug)]
pub enum QueryError {
    // Parse errors
    UnexpectedToken(String),
    UnexpectedEOF,
    // Validation errors
    InvalidCategory(String),
    ImpossibleBranch(String),
}

/// Simple tokenizer
/// (inlined since we only use it once)
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

/// Pratt parser
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

fn parse_or(tokens: &mut VecDeque<String>) -> Result<Expr, QueryError> {
    let mut node = parse_and(tokens)?;
    if let Some(tok) = tokens.front() {
        if tok == "+" {
            tokens.pop_front();
            let rhs = parse_or(tokens)?;
            node = Expr::Or(Box::new(node), Box::new(rhs));
        } else {
            return Err(QueryError::UnexpectedToken(tok.clone()));
        }
    }

    Ok(node)
}

fn parse_and(tokens: &mut VecDeque<String>) -> Result<Expr, QueryError> {
    let mut node = parse_not(tokens)?;
    if let Some(tok) = tokens.front() {
        if tok == "&" {
            tokens.pop_front();
            let rhs = parse_and(tokens)?;
            node = Expr::And(Box::new(node), Box::new(rhs));
        }
    }

    Ok(node)
}

fn parse_not(tokens: &mut VecDeque<String>) -> Result<Expr, QueryError> {
    let mut node = parse_primary(tokens)?;
    if let Some(tok) = tokens.front() {
        if tok == "-" {
            tokens.pop_front();
            let rhs = parse_primary(tokens)?;
            node = Expr::Not(Box::new(node), Box::new(rhs));
        }
    }

    Ok(node)
}

fn parse_primary(tokens: &mut VecDeque<String>) -> Result<Expr, QueryError> {
    if let Some(tok) = tokens.pop_front() {
        match tok.as_str() {
            "(" => {
                let expr = parse_expr(tokens)?;
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
                    let Some(c) = tokens.pop_front() else { break };
                    match c.as_str() {
                        "&" | "+" | "-" | "(" | ")" => {
                            tokens.push_front(c); // put it back... this somehow fits the borrow checker
                            break;
                        }
                        _ => {
                            buf.push(c);
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
const FUZZY_THRESHOLD: usize = 5;

fn match_against(
    comparator: &levenshtein::BatchComparator<char>,
    b: &Vec<String>,
) -> Option<String> {
    let mut distances: Vec<(String, usize)> = b
        .into_iter()
        .map(|item| (item.clone(), comparator.distance(item.chars())))
        .collect();

    distances.sort_by_key(|(_, dist)| *dist);

    let close_matches: Vec<String> = distances
        .iter()
        .filter(|(_, dist)| *dist < FUZZY_THRESHOLD)
        .map(|(item, _)| item.clone())
        .collect();

    info!("Close matches: {:?}", close_matches);

    if close_matches.len() == 1 {
        Some(close_matches[0].clone())
    } else {
        None
    }
}
/// Validate recursively
fn validate(expr: &Expr) -> Result<(Vec<String>, Vec<String>, Vec<String>), QueryError> {
    match expr {
        Expr::Token(t) => {
            // TODO: fuzzy match
            let norm = capitalize_token(t);
            let comparator = levenshtein::BatchComparator::new(norm.chars());
            for (key, value) in CATEGORIES.entries() {
                // not even sure if this is right
                if comparator.distance(key.chars()) < FUZZY_THRESHOLD {
                    return Ok((
                        vec![key.to_string()],
                        value.0.to_vec().iter().map(|s| s.to_string()).collect(),
                        value.1.to_vec().iter().map(|s| s.to_string()).collect(),
                    ));
                }
                // um these conversions are so inefficient guys
                if let Some(result) = match_against(
                    &comparator,
                    &value
                        .0
                        .to_vec()
                        .into_iter()
                        .map(|x| x.to_string())
                        .collect(),
                ) {
                    return Ok((vec![key.to_string()], vec![result], vec![]));
                }
                if let Some(result) = match_against(
                    &comparator,
                    &value
                        .1
                        .to_vec()
                        .into_iter()
                        .map(|x| x.to_string())
                        .collect(),
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
            // Not sure if this logic is right
            if cc.is_empty() {
                return Err(QueryError::ImpossibleBranch(format!("{} & {}", a, b)));
            }
            let sc: Vec<_> = asub.iter().filter(|x| bsub.contains(x)).cloned().collect();
            let alt: Vec<_> = aalt.iter().filter(|x| balt.contains(x)).cloned().collect();
            Ok((cc, sc, alt))
        }
        Expr::Or(a, b) => {
            let (mut ac, mut asub, mut aalt) = validate(a)?;
            let (bc, bsub, balt) = validate(b)?;
            ac.extend(bc);
            asub.extend(bsub);
            aalt.extend(balt);
            ac.sort();
            ac.dedup();
            asub.sort();
            asub.dedup();
            aalt.sort();
            aalt.dedup();
            Ok((ac, asub, aalt))
        }
        // TODO: make sure this doesn't kill the parent
        // category... that might require a rehaul
        Expr::Not(a, b) => {
            let (mut ac, mut asub, mut aalt) = validate(a)?;
            let (bc, bsub, balt) = validate(b)?;
            ac.retain(|x| !bc.contains(x));
            asub.retain(|x| !bsub.contains(x));
            aalt.retain(|x| !balt.contains(x));
            if ac.is_empty() && asub.is_empty() && aalt.is_empty() {
                return Err(QueryError::ImpossibleBranch(format!("{} - {}", a, b)));
            }
            Ok((ac, asub, aalt))
        }
    }
}

// Used only once: should inline
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

pub fn parse_query(query_str: &str) -> Result<ApiQuery, QueryError> {
    let mut tokens = tokenize(query_str);
    parse_expr(&mut tokens).and_then(|expr| build_api_query(&expr))
}
