//! USDA tokeniser.
//!
//! Converts a UTF-8 USDA source string into a flat `Vec<Token>` that the
//! parser then consumes.  The token stream intentionally omits comments and
//! insignificant whitespace.

/// A single lexical token from a USDA source file.
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // ── Keywords ─────────────────────────────────────────────────────────────
    Def,
    Over,
    Class,
    Uniform,
    Custom,
    Prepend,
    Append,
    Rel,
    Inherits,
    References,
    Payload,
    VariantSet,
    Variant,

    // ── Literals ─────────────────────────────────────────────────────────────
    /// Bare identifier or type token — `MyMesh`, `point3f`, `xformOp:translate`.
    Ident(String),
    /// Double-quoted or triple-quoted string literal (content only, no quotes).
    StringLit(String),
    /// `@asset_path@` — asset reference.
    AssetPath(String),
    /// `<SdfPath>` — relationship target or inherit path.
    SdfPath(String),
    /// Integer literal.
    Int(i64),
    /// Floating-point literal.
    Float(f64),
    /// `true` / `false`
    Bool(bool),
    /// `None` keyword (USD null/blocked opinion).
    None,

    // ── Punctuation ───────────────────────────────────────────────────────────
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Comma,
    Dot,
    Equals,
    Colon,
}

// ── Lexer ─────────────────────────────────────────────────────────────────────

/// Tokenise a USDA source string.
///
/// Returns a `Vec<Token>` on success, or an error string describing the first
/// unrecognised character or malformed literal.
pub fn tokenise(src: &str) -> Result<Vec<Token>, String> {
    let chars: Vec<char> = src.chars().collect();
    let mut pos = 0usize;
    let mut tokens = Vec::new();

    while pos < chars.len() {
        skip_whitespace_and_comments(&chars, &mut pos);
        if pos >= chars.len() {
            break;
        }

        let c = chars[pos];

        // ── Triple-quoted string ──────────────────────────────────────────────
        if c == '"' && chars.get(pos + 1) == Some(&'"') && chars.get(pos + 2) == Some(&'"') {
            pos += 3;
            let start = pos;
            while pos + 2 < chars.len()
                && !(chars[pos] == '"' && chars[pos + 1] == '"' && chars[pos + 2] == '"')
            {
                pos += 1;
            }
            let s: String = chars[start..pos].iter().collect();
            pos += 3;
            tokens.push(Token::StringLit(s));
            continue;
        }

        // ── Double-quoted string ─────────────────────────────────────────────
        if c == '"' {
            pos += 1;
            let mut s = String::new();
            while pos < chars.len() && chars[pos] != '"' {
                if chars[pos] == '\\' && pos + 1 < chars.len() {
                    pos += 1;
                    match chars[pos] {
                        'n' => s.push('\n'),
                        't' => s.push('\t'),
                        '"' => s.push('"'),
                        '\\' => s.push('\\'),
                        other => {
                            s.push('\\');
                            s.push(other);
                        }
                    }
                } else {
                    s.push(chars[pos]);
                }
                pos += 1;
            }
            pos += 1; // closing "
            tokens.push(Token::StringLit(s));
            continue;
        }

        // ── Asset path  @…@ ──────────────────────────────────────────────────
        if c == '@' {
            pos += 1;
            let start = pos;
            while pos < chars.len() && chars[pos] != '@' {
                pos += 1;
            }
            let s: String = chars[start..pos].iter().collect();
            pos += 1; // closing @
            tokens.push(Token::AssetPath(s));
            continue;
        }

        // ── SdfPath  <…> ─────────────────────────────────────────────────────
        if c == '<' {
            pos += 1;
            let start = pos;
            while pos < chars.len() && chars[pos] != '>' {
                pos += 1;
            }
            let s: String = chars[start..pos].iter().collect();
            pos += 1;
            tokens.push(Token::SdfPath(s));
            continue;
        }

        // ── Number  (optional leading minus handled as Ident '-' by caller) ──
        if c.is_ascii_digit()
            || (c == '-' && chars.get(pos + 1).map_or(false, |n| n.is_ascii_digit()))
        {
            let start = pos;
            if c == '-' {
                pos += 1;
            }
            while pos < chars.len() && chars[pos].is_ascii_digit() {
                pos += 1;
            }
            let is_float =
                pos < chars.len() && (chars[pos] == '.' || chars[pos] == 'e' || chars[pos] == 'E');
            if is_float {
                if pos < chars.len() && chars[pos] == '.' {
                    pos += 1;
                }
                while pos < chars.len() && chars[pos].is_ascii_digit() {
                    pos += 1;
                }
                if pos < chars.len() && (chars[pos] == 'e' || chars[pos] == 'E') {
                    pos += 1;
                    if pos < chars.len() && (chars[pos] == '+' || chars[pos] == '-') {
                        pos += 1;
                    }
                    while pos < chars.len() && chars[pos].is_ascii_digit() {
                        pos += 1;
                    }
                }
                // optional 'f' suffix
                if pos < chars.len() && chars[pos] == 'f' {
                    pos += 1;
                }
                let s: String = chars[start..pos].iter().collect();
                let v: f64 = s
                    .trim_end_matches('f')
                    .parse()
                    .map_err(|_| format!("bad float: {s}"))?;
                tokens.push(Token::Float(v));
            } else {
                let s: String = chars[start..pos].iter().collect();
                let v: i64 = s.parse().map_err(|_| format!("bad int: {s}"))?;
                tokens.push(Token::Int(v));
            }
            continue;
        }

        // ── Identifier / keyword ─────────────────────────────────────────────
        if c.is_alphabetic() || c == '_' {
            let start = pos;
            while pos < chars.len()
                && (chars[pos].is_alphanumeric() || chars[pos] == '_' || chars[pos] == ':')
            {
                pos += 1;
            }
            // also consume trailing [] for type names like `point3f[]`
            if pos + 1 < chars.len() && chars[pos] == '[' && chars[pos + 1] == ']' {
                pos += 2;
            }
            let word: String = chars[start..pos].iter().collect();
            tokens.push(keyword_or_ident(word));
            continue;
        }

        // ── Single-char punctuation ───────────────────────────────────────────
        let tok = match c {
            '(' => Token::LParen,
            ')' => Token::RParen,
            '{' => Token::LBrace,
            '}' => Token::RBrace,
            '[' => Token::LBracket,
            ']' => Token::RBracket,
            ',' => Token::Comma,
            '.' => Token::Dot,
            '=' => Token::Equals,
            ':' => Token::Colon,
            _ => return Err(format!("unexpected character {:?} at position {pos}", c)),
        };
        tokens.push(tok);
        pos += 1;
    }

    Ok(tokens)
}

fn keyword_or_ident(word: String) -> Token {
    match word.as_str() {
        "def" => Token::Def,
        "over" => Token::Over,
        "class" => Token::Class,
        "uniform" => Token::Uniform,
        "custom" => Token::Custom,
        "prepend" => Token::Prepend,
        "append" => Token::Append,
        "rel" => Token::Rel,
        "inherits" => Token::Inherits,
        "references" => Token::References,
        "payload" => Token::Payload,
        "variantSet" => Token::VariantSet,
        "variant" => Token::Variant,
        "true" => Token::Bool(true),
        "false" => Token::Bool(false),
        "None" => Token::None,
        _ => Token::Ident(word),
    }
}

fn skip_whitespace_and_comments(chars: &[char], pos: &mut usize) {
    while *pos < chars.len() {
        let c = chars[*pos];
        if c == ' ' || c == '\t' || c == '\r' || c == '\n' {
            *pos += 1;
        } else if c == '#' {
            // line comment — skip to end of line
            while *pos < chars.len() && chars[*pos] != '\n' {
                *pos += 1;
            }
        } else {
            break;
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenise_header() {
        let src = r#"#usda 1.0
( upAxis = "Y" )"#;
        let toks = tokenise(src).unwrap();
        assert!(toks.contains(&Token::LParen));
        assert!(toks.contains(&Token::Ident("upAxis".into())));
        assert!(toks.contains(&Token::StringLit("Y".into())));
        assert!(toks.contains(&Token::RParen));
    }

    #[test]
    fn tokenise_def_prim() {
        let src = r#"def Xform "World" {}"#;
        let toks = tokenise(src).unwrap();
        assert_eq!(toks[0], Token::Def);
        assert_eq!(toks[1], Token::Ident("Xform".into()));
        assert_eq!(toks[2], Token::StringLit("World".into()));
        assert_eq!(toks[3], Token::LBrace);
        assert_eq!(toks[4], Token::RBrace);
    }

    #[test]
    fn tokenise_float_array() {
        let src = "[1.0, -2.5f, 3e1]";
        let toks = tokenise(src).unwrap();
        assert_eq!(toks[1], Token::Float(1.0));
        assert_eq!(toks[3], Token::Float(-2.5));
        assert_eq!(toks[5], Token::Float(30.0));
    }
}
