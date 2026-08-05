//! ASCII FBX parser.
//!
//! ASCII FBX files begin with `; FBX` and use a human-readable node/property
//! syntax.  This module tokenises the input and then produces the same
//! `FbxDocument` IR as the binary parser, so the rest of the pipeline is
//! shared.

use std::io::{Read, Seek, SeekFrom};

use solid_rs::traits::ReadSeek;
use solid_rs::{Result, SolidError};

use crate::document::{FbxDocument, FbxNode, FbxProperty};

// ── Detection ─────────────────────────────────────────────────────────────────

/// Returns `true` if the first bytes look like an ASCII FBX file.
/// The reader position is restored afterwards. A leading UTF-8 BOM is ignored.
pub(crate) fn detect(reader: &mut dyn ReadSeek) -> bool {
    let mut buf = [0u8; 16];
    let n = reader.read(&mut buf).unwrap_or(0);
    let data = buf[..n].strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(&buf[..n]);
    let ok = std::str::from_utf8(data)
        .map(|s| s.starts_with("; FBX") || s.starts_with(";FBX"))
        .unwrap_or(false);
    let _ = reader.seek(SeekFrom::Start(0));
    ok
}

// ── Entry point ───────────────────────────────────────────────────────────────

/// Parse an ASCII FBX document from `reader`.
pub(crate) fn parse(reader: &mut dyn ReadSeek) -> Result<FbxDocument> {
    let mut src = String::new();
    reader.read_to_string(&mut src).map_err(SolidError::Io)?;
    // Strip a leading UTF-8 BOM if present.
    if src.starts_with('\u{FEFF}') {
        src.remove(0);
    }

    let tokens = tokenize(&src)?;
    let mut parser = AsciiParser::new(tokens);
    let version = parser.sniff_version();
    let roots = parser.parse_nodes()?;

    Ok(FbxDocument { version, roots })
}

// ── Tokeniser ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Word(String),
    Integer(i64),
    Float(f64),
    Str(String),
    Colon,
    Comma,
    LBrace,
    RBrace,
    Star,
    Eof,
}

fn tokenize(src: &str) -> Result<Vec<Token>> {
    let src: Vec<char> = src.chars().collect();
    let mut pos = 0;
    let mut tokens = Vec::new();

    while pos < src.len() {
        let c = src[pos];

        // Whitespace (including newlines)
        if c.is_whitespace() {
            pos += 1;
            continue;
        }

        // Line comment
        if c == ';' {
            while pos < src.len() && src[pos] != '\n' {
                pos += 1;
            }
            continue;
        }

        match c {
            '{' => {
                tokens.push(Token::LBrace);
                pos += 1;
            }
            '}' => {
                tokens.push(Token::RBrace);
                pos += 1;
            }
            ':' => {
                tokens.push(Token::Colon);
                pos += 1;
            }
            ',' => {
                tokens.push(Token::Comma);
                pos += 1;
            }
            '*' => {
                tokens.push(Token::Star);
                pos += 1;
            }

            '"' => {
                pos += 1; // skip opening quote
                let mut s = String::new();
                loop {
                    if pos >= src.len() {
                        break;
                    }
                    match src[pos] {
                        '"' => {
                            pos += 1;
                            break;
                        }
                        '\\' => {
                            pos += 1;
                            if pos < src.len() {
                                s.push(match src[pos] {
                                    'n' => '\n',
                                    't' => '\t',
                                    c => c,
                                });
                                pos += 1;
                            }
                        }
                        c => {
                            s.push(c);
                            pos += 1;
                        }
                    }
                }
                tokens.push(Token::Str(s));
            }

            // Numbers (including negative)
            c if c.is_ascii_digit()
                || ((c == '-' || c == '+')
                    && pos + 1 < src.len()
                    && src[pos + 1].is_ascii_digit()) =>
            {
                let mut s = String::new();
                s.push(src[pos]);
                pos += 1;
                let mut has_dot = false;
                let mut has_exp = false;
                while pos < src.len() {
                    let nc = src[pos];
                    if nc.is_ascii_digit() {
                        s.push(nc);
                        pos += 1;
                    } else if nc == '.' && !has_dot {
                        has_dot = true;
                        s.push(nc);
                        pos += 1;
                    } else if (nc == 'e' || nc == 'E') && !has_exp {
                        has_exp = true;
                        s.push(nc);
                        pos += 1;
                        if pos < src.len() && (src[pos] == '+' || src[pos] == '-') {
                            s.push(src[pos]);
                            pos += 1;
                        }
                    } else {
                        break;
                    }
                }
                if has_dot || has_exp {
                    let f: f64 = s
                        .parse()
                        .map_err(|_| SolidError::parse(format!("bad float literal: {s}")))?;
                    tokens.push(Token::Float(f));
                } else {
                    let i: i64 = s
                        .parse()
                        .map_err(|_| SolidError::parse(format!("bad integer literal: {s}")))?;
                    tokens.push(Token::Integer(i));
                }
            }

            // Identifiers / keywords. Continue through '-' so dashed bare
            // identifiers like `Pre-Extrapolation` stay a single token.
            // (Numbers are consumed greedily first, so `-1` / `1e-5` are
            // unaffected.)
            c if c.is_alphabetic() || c == '_' => {
                let mut s = String::new();
                while pos < src.len()
                    && (src[pos].is_alphanumeric() || src[pos] == '_' || src[pos] == '-')
                {
                    s.push(src[pos]);
                    pos += 1;
                }
                tokens.push(Token::Word(s));
            }

            _ => {
                pos += 1;
            } // skip unknown chars
        }
    }

    tokens.push(Token::Eof);
    Ok(tokens)
}

// ── Parser ────────────────────────────────────────────────────────────────────

struct AsciiParser {
    tokens: Vec<Token>,
    pos: usize,
}

impl AsciiParser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn peek(&self) -> &Token {
        self.tokens.get(self.pos).unwrap_or(&Token::Eof)
    }

    fn peek2(&self) -> &Token {
        self.tokens.get(self.pos + 1).unwrap_or(&Token::Eof)
    }

    fn next(&mut self) -> Token {
        let t = self.tokens.get(self.pos).cloned().unwrap_or(Token::Eof);
        if self.pos < self.tokens.len() {
            self.pos += 1;
        }
        t
    }

    /// Try to extract the FBX version from a `FBXVersion: NNNN` node
    /// without advancing the parser position.
    fn sniff_version(&self) -> u32 {
        // Scan for `FBXVersion` Word followed by `:` and an Integer
        let ts = &self.tokens;
        for i in 0..ts.len().saturating_sub(2) {
            if ts[i] == Token::Word("FBXVersion".into()) && ts[i + 1] == Token::Colon {
                if let Token::Integer(v) = ts.get(i + 2).unwrap_or(&Token::Eof) {
                    return *v as u32;
                }
            }
        }
        7400 // default
    }

    // ── Node list ─────────────────────────────────────────────────────────────

    fn parse_nodes(&mut self) -> Result<Vec<FbxNode>> {
        let mut nodes = Vec::new();
        loop {
            match self.peek() {
                Token::Eof | Token::RBrace => break,
                Token::Word(_) => {
                    if let Some(n) = self.parse_node()? {
                        nodes.push(n);
                    }
                }
                _ => {
                    self.next();
                } // skip unexpected tokens
            }
        }
        Ok(nodes)
    }

    // ── Single node ───────────────────────────────────────────────────────────

    fn parse_node(&mut self) -> Result<Option<FbxNode>> {
        let name = match self.next() {
            Token::Word(w) => w,
            _ => return Ok(None),
        };

        // Expect ':'
        if self.peek() != &Token::Colon {
            return Ok(None);
        }
        self.next(); // consume ':'

        // Read properties until '{', '}', EOF, or start of next node (Word ':')
        let mut properties = Vec::new();
        loop {
            match self.peek() {
                Token::LBrace | Token::RBrace | Token::Eof => break,
                // Start of a new sibling node — stop reading properties
                Token::Word(_) if self.peek2() == &Token::Colon => break,
                Token::Comma => {
                    self.next();
                }
                Token::Star => {
                    // '*N { a: v1,v2,... }' — array property
                    if let Some(arr) = self.parse_array_property()? {
                        properties.push(arr);
                    }
                }
                _ => {
                    if let Some(p) = self.parse_scalar_property() {
                        properties.push(p);
                    }
                }
            }
        }

        // Optional block of children
        let mut children = Vec::new();
        if self.peek() == &Token::LBrace {
            self.next(); // consume '{'
            children = self.parse_nodes()?;
            if self.peek() == &Token::RBrace {
                self.next(); // consume '}'
            }
        }

        Ok(Some(FbxNode {
            name,
            properties,
            children,
        }))
    }

    // ── Array property: *N { a: v1,v2,... } ──────────────────────────────────

    fn parse_array_property(&mut self) -> Result<Option<FbxProperty>> {
        self.next(); // consume '*'

        // Count hint (we don't enforce it)
        let _count = match self.next() {
            Token::Integer(n) => n as usize,
            _ => 0,
        };

        if self.peek() != &Token::LBrace {
            return Ok(None);
        }
        self.next(); // consume '{'

        // Expect 'a' ':' then comma-separated numbers
        if let Token::Word(_) = self.peek() {
            self.next();
        } // 'a'
        if self.peek() == &Token::Colon {
            self.next();
        } // ':'

        let mut ints: Vec<i64> = Vec::new();
        let mut floats: Vec<f64> = Vec::new();
        let mut is_float = false;

        loop {
            match self.peek() {
                Token::RBrace | Token::Eof => break,
                Token::Comma => {
                    self.next();
                }
                Token::Integer(v) => {
                    let v = *v;
                    self.next();
                    ints.push(v);
                    floats.push(v as f64);
                }
                Token::Float(v) => {
                    let v = *v;
                    self.next();
                    is_float = true;
                    floats.push(v);
                    ints.push(v as i64);
                }
                _ => {
                    self.next();
                }
            }
        }

        if self.peek() == &Token::RBrace {
            self.next();
        } // consume '}'

        if is_float {
            Ok(Some(FbxProperty::ArrFloat64(floats)))
        } else {
            // Use i64 when any value exceeds i32 range (e.g. FBX animation timestamps).
            let needs_i64 = ints
                .iter()
                .any(|&v| v > i32::MAX as i64 || v < i32::MIN as i64);
            if needs_i64 {
                Ok(Some(FbxProperty::ArrInt64(ints)))
            } else {
                Ok(Some(FbxProperty::ArrInt32(
                    ints.into_iter().map(|v| v as i32).collect(),
                )))
            }
        }
    }

    // ── Scalar property ───────────────────────────────────────────────────────

    fn parse_scalar_property(&mut self) -> Option<FbxProperty> {
        match self.peek().clone() {
            Token::Integer(v) => {
                self.next();
                Some(FbxProperty::Int64(v))
            }
            Token::Float(v) => {
                self.next();
                Some(FbxProperty::Float64(v))
            }
            Token::Str(s) => {
                self.next();
                Some(FbxProperty::String(s))
            }
            // Bare identifiers are valid ASCII FBX string values; store them
            // instead of silently discarding the token.
            Token::Word(w) => {
                self.next();
                Some(FbxProperty::String(w))
            }
            _ => {
                self.next();
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_str(src: &str) -> FbxDocument {
        let bytes = src.as_bytes().to_vec();
        let mut cursor = std::io::Cursor::new(bytes);
        parse(&mut cursor).unwrap()
    }

    /// Regression: dashed bare identifiers such as `Pre-Extrapolation`
    /// must stay a single token instead of splitting into `Pre` +
    /// `Extrapolation` (which spawned a bogus node on parse).
    #[test]
    fn dashed_bare_identifiers_stay_single_token() {
        let tokens = tokenize("Pre-Extrapolation: 0").unwrap();
        assert!(
            matches!(&tokens[0], Token::Word(w) if w == "Pre-Extrapolation"),
            "expected single dashed token, got {tokens:?}"
        );
    }

    /// Regression: a Properties70 block with dashed properties and bare-word
    /// values must parse without spawning bogus sibling nodes or dropping
    /// values.
    #[test]
    fn dashed_identifiers_and_bare_word_values_parse() {
        let src = r#"; FBX 7.4.0 project file
Model: 1, "M", "Mesh"  {
    Version: 232
    Properties70:  {
        Pre-Extrapolation: 0
        Post-Extrapolation: 0
        Shading: Flat
    }
}
"#;
        let doc = parse_str(src);
        let model = doc
            .roots
            .iter()
            .find(|n| n.name == "Model")
            .expect("Model node");
        let props = model.child("Properties70").expect("Properties70 node");

        assert!(
            props.children.iter().any(|n| n.name == "Pre-Extrapolation"),
            "dashed property was not parsed as a node"
        );
        assert!(
            !props.children.iter().any(|n| n.name == "Extrapolation"),
            "dashed identifier was split into a bogus 'Extrapolation' node"
        );
        let shading = props
            .children
            .iter()
            .find(|n| n.name == "Shading")
            .expect("Shading node");
        assert!(
            shading.properties.iter().any(|p| p.as_str() == Some("Flat")),
            "bare-word value 'Flat' was dropped"
        );
    }

    /// Regression: negative numbers still tokenise correctly after the
    /// dashed-identifier change (`-1` must not become a word).
    #[test]
    fn negative_numbers_still_parse_as_integers() {
        let tokens = tokenize("-1,2,-3.5").unwrap();
        assert!(matches!(tokens[0], Token::Integer(-1)), "{tokens:?}");
        assert!(matches!(tokens[2], Token::Integer(2)), "{tokens:?}");
        assert!(
            matches!(tokens[4], Token::Float(v) if (v - -3.5).abs() < 1e-9),
            "{tokens:?}"
        );
    }
}
