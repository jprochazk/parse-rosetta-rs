use std::collections::HashMap;

use logos::{Lexer, Logos, Span};

type Error = (String, Span);

type Result<T> = std::result::Result<T, Error>;

/// Represent any valid JSON value.
#[derive(Debug)]
pub enum Value {
    /// null.
    Null,
    /// true or false.
    Bool(bool),
    /// Any floating point number.
    Number(f64),
    /// Any quoted string.
    String(String),
    /// An array of values
    Array(Vec<Value>),
    /// An dictionary mapping keys and values.
    Object(HashMap<String, Value>),
}

/// All meaningful JSON tokens.
///
/// > NOTE: regexes for [`Token::Number`] and [`Token::String`] may not
/// > catch all possible values, especially for strings. If you find
/// > errors, please report them so that we can improve the regex.
#[derive(Clone, Copy, Debug, Logos)]
#[logos(skip r"[ \t\r\n\f]+")]
pub enum Token {
    #[token("true")]
    True,

    #[token("false")]
    False,

    #[token("{")]
    BraceOpen,

    #[token("}")]
    BraceClose,

    #[token("[")]
    BracketOpen,

    #[token("]")]
    BracketClose,

    #[token(":")]
    Colon,

    #[token(",")]
    Comma,

    #[token("null")]
    Null,

    #[regex(r"-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?")]
    Number,

    #[regex(r#""([^"\\]|\\["\\bnfrt]|u[a-fA-F0-9]{4})*""#)]
    String,
}

/// Parse a token stream into a JSON value.
pub fn parse_value(lexer: &mut Lexer<'_, Token>) -> Result<Value> {
    if let Some(token) = lexer.next() {
        match token {
            Ok(Token::True) => Ok(Value::Bool(true)),
            Ok(Token::False) => Ok(Value::Bool(false)),
            Ok(Token::BraceOpen) => parse_object(lexer),
            Ok(Token::BracketOpen) => parse_array(lexer),
            Ok(Token::Null) => Ok(Value::Null),
            Ok(Token::Number) => Ok(Value::Number(lexer.slice().parse::<f64>().unwrap())),
            Ok(Token::String) => Ok(Value::String(lexer.slice().to_owned())),
            _ => Err((
                "unexpected token here (context: value)".to_owned(),
                lexer.span(),
            )),
        }
    } else {
        Err(("empty values are not allowed".to_owned(), lexer.span()))
    }
}

/// Parse a token stream into an array and return when
/// a valid terminator is found.
///
/// > NOTE: we assume '[' was consumed.
fn parse_array(lexer: &mut Lexer<'_, Token>) -> Result<Value> {
    let mut array = Vec::new();
    let span = lexer.span();
    let mut awaits_comma = false;
    let mut awaits_value = false;

    while let Some(token) = lexer.next() {
        match token {
            Ok(Token::True) if !awaits_comma => {
                array.push(Value::Bool(true));
                awaits_value = false;
            }
            Ok(Token::False) if !awaits_comma => {
                array.push(Value::Bool(false));
                awaits_value = false;
            }
            Ok(Token::BraceOpen) if !awaits_comma => {
                let object = parse_object(lexer)?;
                array.push(object);
                awaits_value = false;
            }
            Ok(Token::BracketOpen) if !awaits_comma => {
                let sub_array = parse_array(lexer)?;
                array.push(sub_array);
                awaits_value = false;
            }
            Ok(Token::BracketClose) if !awaits_value => return Ok(Value::Array(array)),
            Ok(Token::Comma) if awaits_comma => awaits_value = true,
            Ok(Token::Null) if !awaits_comma => {
                array.push(Value::Null);
                awaits_value = false
            }
            Ok(Token::Number) if !awaits_comma => {
                array.push(Value::Number(lexer.slice().parse::<f64>().unwrap()));
                awaits_value = false;
            }
            Ok(Token::String) if !awaits_comma => {
                array.push(Value::String(lexer.slice().to_owned()));
                awaits_value = false;
            }
            _ => {
                return Err((
                    "unexpected token here (context: array)".to_owned(),
                    lexer.span(),
                ))
            }
        }
        awaits_comma = !awaits_value;
    }
    Err(("unmatched opening bracket defined here".to_owned(), span))
}

/// Parse a token stream into an object and return when
/// a valid terminator is found.
///
/// > NOTE: we assume '{' was consumed.
fn parse_object(lexer: &mut Lexer<'_, Token>) -> Result<Value> {
    let mut map = HashMap::new();
    let span = lexer.span();
    let mut awaits_comma = false;
    let mut awaits_key = false;

    while let Some(token) = lexer.next() {
        match token {
            Ok(Token::BraceClose) if !awaits_key => return Ok(Value::Object(map)),
            Ok(Token::Comma) if awaits_comma => awaits_key = true,
            Ok(Token::String) if !awaits_comma => {
                let key = lexer.slice().to_owned();
                match lexer.next() {
                    Some(Ok(Token::Colon)) => (),
                    _ => {
                        return Err((
                            "unexpected token here, expecting ':'".to_owned(),
                            lexer.span(),
                        ))
                    }
                }
                let value = parse_value(lexer)?;
                map.insert(key, value);
                awaits_key = false;
            }
            _ => {
                return Err((
                    "unexpected token here (context: object)".to_owned(),
                    lexer.span(),
                ))
            }
        }
        awaits_comma = !awaits_key;
    }
    Err(("unmatched opening brace defined here".to_owned(), span))
}
