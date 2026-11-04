//! Minimal zero-dependency JSON parser (crate-internal).
//!
//! Sufficient for advance input and general JSON value handling.

use crate::advance::AdvanceError;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum JsonValue {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<JsonValue>),
    Object(Vec<(String, JsonValue)>),
}

pub(crate) struct JsonParser<'a> {
    s: &'a str,
    pos: usize,
}

impl<'a> JsonParser<'a> {
    pub(crate) fn new(s: &'a str) -> Self {
        Self { s, pos: 0 }
    }

    pub(crate) fn peek(&self) -> Option<char> {
        self.s[self.pos..].chars().next()
    }

    pub(crate) fn bump(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.pos += c.len_utf8();
        Some(c)
    }

    pub(crate) fn skip_ws(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_whitespace() {
                self.bump();
            } else {
                break;
            }
        }
    }

    pub(crate) fn parse(&mut self) -> Result<JsonValue, AdvanceError> {
        self.skip_ws();
        self.parse_value()
    }

    pub(crate) fn parse_value(&mut self) -> Result<JsonValue, AdvanceError> {
        self.skip_ws();
        match self.peek() {
            Some('{') => self.parse_object(),
            Some('[') => self.parse_array(),
            Some('"') => self.parse_string().map(JsonValue::String),
            Some('t') => self.expect_word("true").map(|_| JsonValue::Bool(true)),
            Some('f') => self.expect_word("false").map(|_| JsonValue::Bool(false)),
            Some('n') => self.expect_word("null").map(|_| JsonValue::Null),
            Some(c) if c == '-' || c.is_ascii_digit() => self.parse_number(),
            other => Err(AdvanceError {
                message: format!("expected JSON value, got {:?}", other),
            }),
        }
    }

    pub(crate) fn parse_object(&mut self) -> Result<JsonValue, AdvanceError> {
        self.bump(); // '{'
        let mut pairs = Vec::new();
        self.skip_ws();
        if self.peek() == Some('}') {
            self.bump();
            return Ok(JsonValue::Object(pairs));
        }
        loop {
            self.skip_ws();
            let key = self.parse_string()?;
            self.skip_ws();
            if self.bump() != Some(':') {
                return Err(AdvanceError {
                    message: "expected ':' after object key".to_string(),
                });
            }
            let value = self.parse_value()?;
            pairs.push((key, value));
            self.skip_ws();
            match self.bump() {
                Some(',') => continue,
                Some('}') => break,
                other => {
                    return Err(AdvanceError {
                        message: format!("expected ',' or '}}' in object, got {:?}", other),
                    })
                }
            }
        }
        Ok(JsonValue::Object(pairs))
    }

    pub(crate) fn parse_array(&mut self) -> Result<JsonValue, AdvanceError> {
        self.bump(); // '['
        let mut items = Vec::new();
        self.skip_ws();
        if self.peek() == Some(']') {
            self.bump();
            return Ok(JsonValue::Array(items));
        }
        loop {
            let value = self.parse_value()?;
            items.push(value);
            self.skip_ws();
            match self.bump() {
                Some(',') => continue,
                Some(']') => break,
                other => {
                    return Err(AdvanceError {
                        message: format!("expected ',' or ']' in array, got {:?}", other),
                    })
                }
            }
        }
        Ok(JsonValue::Array(items))
    }

    pub(crate) fn parse_string(&mut self) -> Result<String, AdvanceError> {
        if self.bump() != Some('"') {
            return Err(AdvanceError {
                message: "expected string".to_string(),
            });
        }
        let mut out = String::new();
        loop {
            match self.bump() {
                Some('"') => break,
                Some('\\') => match self.bump() {
                    Some('"') => out.push('"'),
                    Some('\\') => out.push('\\'),
                    Some('/') => out.push('/'),
                    Some('b') => out.push('\u{0008}'),
                    Some('f') => out.push('\u{000c}'),
                    Some('n') => out.push('\n'),
                    Some('r') => out.push('\r'),
                    Some('t') => out.push('\t'),
                    Some('u') => {
                        let hex: String = (0..4).filter_map(|_| self.bump()).collect();
                        let code = u32::from_str_radix(&hex, 16).map_err(|_| AdvanceError {
                            message: "invalid unicode escape".to_string(),
                        })?;
                        let c = char::from_u32(code).ok_or_else(|| AdvanceError {
                            message: "invalid unicode codepoint".to_string(),
                        })?;
                        out.push(c);
                    }
                    other => {
                        return Err(AdvanceError {
                            message: format!("invalid escape sequence {:?}", other),
                        })
                    }
                },
                Some(c) => out.push(c),
                None => {
                    return Err(AdvanceError {
                        message: "unterminated string".to_string(),
                    })
                }
            }
        }
        Ok(out)
    }

    pub(crate) fn parse_number(&mut self) -> Result<JsonValue, AdvanceError> {
        let start = self.pos;
        if self.peek() == Some('-') {
            self.bump();
        }
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                self.bump();
            } else {
                break;
            }
        }
        if self.peek() == Some('.') {
            self.bump();
            while let Some(c) = self.peek() {
                if c.is_ascii_digit() {
                    self.bump();
                } else {
                    break;
                }
            }
        }
        if let Some(c) = self.peek() {
            if c == 'e' || c == 'E' {
                self.bump();
                if let Some(c2) = self.peek() {
                    if c2 == '+' || c2 == '-' {
                        self.bump();
                    }
                }
                while let Some(c2) = self.peek() {
                    if c2.is_ascii_digit() {
                        self.bump();
                    } else {
                        break;
                    }
                }
            }
        }
        let num_str = &self.s[start..self.pos];
        num_str
            .parse::<f64>()
            .map(JsonValue::Number)
            .map_err(|_| AdvanceError {
                message: format!("invalid number {}", num_str),
            })
    }

    pub(crate) fn expect_word(&mut self, word: &str) -> Result<(), AdvanceError> {
        for c in word.chars() {
            if self.bump() != Some(c) {
                return Err(AdvanceError {
                    message: format!("expected '{}'", word),
                });
            }
        }
        Ok(())
    }
}

pub(crate) fn parse_json(source: &str) -> Result<JsonValue, AdvanceError> {
    let mut p = JsonParser::new(source);
    let value = p.parse()?;
    p.skip_ws();
    if p.pos != p.s.len() {
        return Err(AdvanceError {
            message: "trailing data after JSON value".to_string(),
        });
    }
    Ok(value)
}

pub(crate) fn obj_get<'a>(obj: &'a [(String, JsonValue)], key: &str) -> Option<&'a JsonValue> {
    obj.iter().find(|(k, _)| k == key).map(|(_, v)| v)
}

pub(crate) fn as_str(v: &JsonValue) -> Option<&str> {
    match v {
        JsonValue::String(s) => Some(s),
        _ => None,
    }
}

pub(crate) fn as_array(v: &JsonValue) -> Option<&[JsonValue]> {
    match v {
        JsonValue::Array(a) => Some(a),
        _ => None,
    }
}

pub(crate) fn as_object(v: &JsonValue) -> Option<&[(String, JsonValue)]> {
    match v {
        JsonValue::Object(o) => Some(o),
        _ => None,
    }
}

pub(crate) fn as_number(v: &JsonValue) -> Option<f64> {
    match v {
        JsonValue::Number(n) => Some(*n),
        _ => None,
    }
}

pub(crate) fn escape_json_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{0008}' => out.push_str("\\b"),
            '\u{000c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
