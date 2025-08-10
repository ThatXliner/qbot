/// Query language
use phf::phf_map;
use tracing::debug;

use std::{collections::VecDeque, fmt};

/// category -> (subcategories, alternate subcategories)
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
enum Expr {
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
#[derive(Debug)]
pub struct ApiQuery {
    pub categories: Vec<String>,
    pub subcategories: Vec<String>,
    pub alternate_subcategories: Vec<String>,
}

impl Default for ApiQuery {
    fn default() -> Self {
        Self {
            categories: vec![],
            subcategories: vec![],
            alternate_subcategories: vec![],
        }
    }
}

#[derive(Debug)]
pub enum Error {
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
fn tokenize(input: &str) -> VecDeque<String> {
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
fn parse_expr(tokens: &mut VecDeque<String>) -> Result<Expr, Error> {
    let result = parse_or(tokens);
    if !tokens.is_empty() {
        Err(Error::UnexpectedToken(format!(
            "Unexpected tokens: {:?}",
            tokens
        )))
    } else {
        result
    }
}

fn parse_or(tokens: &mut VecDeque<String>) -> Result<Expr, Error> {
    let mut node = parse_and(tokens)?;
    if let Some(tok) = tokens.front() {
        if tok == "+" {
            tokens.pop_front();
            let rhs = parse_or(tokens)?;
            node = Expr::Or(Box::new(node), Box::new(rhs));
        } else {
            return Err(Error::UnexpectedToken(tok.clone()));
        }
    }

    Ok(node)
}

fn parse_and(tokens: &mut VecDeque<String>) -> Result<Expr, Error> {
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

fn parse_not(tokens: &mut VecDeque<String>) -> Result<Expr, Error> {
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

fn parse_primary(tokens: &mut VecDeque<String>) -> Result<Expr, Error> {
    if let Some(tok) = tokens.pop_front() {
        match tok.as_str() {
            "(" => {
                let expr = parse_expr(tokens)?;
                let Some(next_token) = tokens.front() else {
                    return Err(Error::UnexpectedEOF);
                };
                if next_token != ")" {
                    return Err(Error::UnexpectedToken(format!(
                        "Unexpected token {:?}, expected ')'",
                        next_token
                    )));
                }

                tokens.pop_front();
                Ok(expr)
            }
            // We shouldn't be seeing punctuation here...
            "&" | "+" | "-" | ")" => Err(Error::UnexpectedToken(tok)),
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
        Err(Error::UnexpectedEOF)
    }
}

/// Validate recursively
fn validate(expr: &Expr) -> Result<(Vec<String>, Vec<String>, Vec<String>), Error> {
    match expr {
        Expr::Token(t) => {
            // TODO: fuzzy match
            let norm = capitalize_token(t);
            for (key, value) in CATEGORIES.entries() {
                // not even sure if this is right
                if key.to_string() == norm {
                    return Ok((
                        vec![key.to_string()],
                        value.0.to_vec().iter().map(|s| s.to_string()).collect(),
                        value.1.to_vec().iter().map(|s| s.to_string()).collect(),
                    ));
                }
                if value.0.contains(&norm.as_str()) {
                    return Ok((vec![key.to_string()], vec![norm], vec![]));
                }
                if value.1.contains(&norm.as_str()) {
                    let misc_category = format!("Other {}", key);
                    assert!(value.0.contains(&misc_category.as_str()));
                    return Ok((vec![key.to_string()], vec![misc_category], vec![norm]));
                }
            }
            Err(Error::InvalidCategory(t.clone()))
        }
        Expr::And(a, b) => {
            let (ac, asub, aalt) = validate(a)?;
            let (bc, bsub, balt) = validate(b)?;
            let cc: Vec<_> = ac.iter().filter(|x| bc.contains(x)).cloned().collect();
            // Not sure if this logic is right
            if cc.is_empty() {
                return Err(Error::ImpossibleBranch(format!("{} & {}", a, b)));
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
        // TODO: this needs to be a bit uhh
        // changed so it won't be flip floppy
        // but instead cumulative
        Expr::Not(a, b) => {
            let (mut ac, mut asub, mut aalt) = validate(a)?;
            let (bc, bsub, balt) = validate(b)?;
            ac.retain(|x| !bc.contains(x));
            asub.retain(|x| !bsub.contains(x));
            aalt.retain(|x| !balt.contains(x));
            if ac.is_empty() && asub.is_empty() && aalt.is_empty() {
                return Err(Error::ImpossibleBranch(format!("{} - {}", a, b)));
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

fn build_api_query(expr: &Expr) -> Result<ApiQuery, Error> {
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

pub fn parse_query(query_str: &str) -> Result<ApiQuery, Error> {
    let mut tokens = tokenize(query_str);
    parse_expr(&mut tokens).and_then(|expr| build_api_query(&expr))
}
