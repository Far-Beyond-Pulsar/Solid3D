//! Recursive-descent parser for USDA files.
//!
//! Consumes the token stream produced by [`crate::lexer::tokenise`] and
//! builds a [`UsdDoc`](crate::document::UsdDoc).

use crate::document::{
    Attribute, Prim, Relationship, Specifier, StageMeta, UsdDoc, UsdValue,
};
use crate::lexer::{tokenise, Token};
use solid_rs::SolidError;

// ── Public entry point ────────────────────────────────────────────────────────

/// Parse a USDA source string into a [`UsdDoc`].
pub fn parse(src: &str) -> Result<UsdDoc, SolidError> {
    // Strip the `#usda 1.0` magic line — it looks like a comment to the lexer.
    let tokens = tokenise(src).map_err(|e| SolidError::parse(format!("USDA lex: {e}")))?;
    let mut p = Parser { tokens, pos: 0 };
    p.parse_doc()
}

// ── Parser state ──────────────────────────────────────────────────────────────

struct Parser {
    tokens: Vec<Token>,
    pos:    usize,
}

impl Parser {
    // ── Token stream helpers ─────────────────────────────────────────────────

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn peek2(&self) -> Option<&Token> {
        self.tokens.get(self.pos + 1)
    }

    fn advance(&mut self) -> Option<&Token> {
        let t = self.tokens.get(self.pos);
        self.pos += 1;
        t
    }

    fn expect_ident(&mut self) -> Result<String, SolidError> {
        match self.advance() {
            Some(Token::Ident(s)) => Ok(s.clone()),
            Some(t) => Err(SolidError::parse(format!("expected identifier, got {t:?}"))),
            None    => Err(SolidError::parse("unexpected end of input")),
        }
    }

    fn expect_string(&mut self) -> Result<String, SolidError> {
        match self.advance() {
            Some(Token::StringLit(s)) => Ok(s.clone()),
            Some(t) => Err(SolidError::parse(format!("expected string, got {t:?}"))),
            None    => Err(SolidError::parse("unexpected end of input")),
        }
    }

    fn expect(&mut self, expected: &Token) -> Result<(), SolidError> {
        match self.advance() {
            Some(t) if t == expected => Ok(()),
            Some(t) => Err(SolidError::parse(format!("expected {expected:?}, got {t:?}"))),
            None    => Err(SolidError::parse("unexpected end of input")),
        }
    }

    fn eat(&mut self, tok: &Token) -> bool {
        if self.peek() == Some(tok) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    // ── Document ─────────────────────────────────────────────────────────────

    fn parse_doc(&mut self) -> Result<UsdDoc, SolidError> {
        let mut doc = UsdDoc::new();

        // Optional stage metadata block `( … )`
        if self.peek() == Some(&Token::LParen) {
            doc.meta = self.parse_stage_meta()?;
        }

        // Root prims
        while self.pos < self.tokens.len() {
            if let Some(prim) = self.parse_prim()? {
                doc.root_prims.push(prim);
            }
        }

        Ok(doc)
    }

    // ── Stage metadata  ( key = value … ) ────────────────────────────────────

    fn parse_stage_meta(&mut self) -> Result<StageMeta, SolidError> {
        self.expect(&Token::LParen)?;
        let mut meta = StageMeta::default();

        while self.peek() != Some(&Token::RParen) && self.pos < self.tokens.len() {
            // skip relationship/prepend qualifiers at stage level
            self.eat(&Token::Prepend);
            self.eat(&Token::Append);

            let key = match self.advance() {
                Some(Token::Ident(s)) => s.clone(),
                Some(Token::RParen)   => { self.pos -= 1; break; }
                Some(t) => return Err(SolidError::parse(format!("stage meta key: got {t:?}"))),
                None    => break,
            };

            self.expect(&Token::Equals)?;
            let val = self.parse_value()?;

            match key.as_str() {
                "upAxis"        => meta.up_axis         = value_as_string(&val),
                "defaultPrim"   => meta.default_prim    = value_as_string(&val),
                "doc"           => meta.doc             = value_as_string(&val),
                "metersPerUnit" => meta.meters_per_unit = value_as_f64(&val),
                _               => { meta.extra.insert(key, format!("{val:?}")); }
            }
        }

        self.expect(&Token::RParen)?;
        Ok(meta)
    }

    // ── Prim ─────────────────────────────────────────────────────────────────

    fn parse_prim(&mut self) -> Result<Option<Prim>, SolidError> {
        let specifier = match self.peek() {
            Some(Token::Def)   => { self.advance(); Specifier::Def   }
            Some(Token::Over)  => { self.advance(); Specifier::Over  }
            Some(Token::Class) => { self.advance(); Specifier::Class }
            _ => return Ok(None),
        };

        // Optional type name — if the next token is a string literal, there is no type.
        let type_name = match self.peek() {
            Some(Token::Ident(_)) => {
                if let Some(Token::StringLit(_)) = self.peek2() {
                    // pattern: `def Xform "Name"`
                    match self.advance() {
                        Some(Token::Ident(s)) => s.clone(),
                        _ => String::new(),
                    }
                } else {
                    // pattern: `def "Name"` — untyped prim
                    String::new()
                }
            }
            _ => String::new(),
        };

        let name = self.expect_string()?;
        self.expect(&Token::LBrace)?;

        let mut prim = Prim::new(specifier, type_name, name);

        while self.peek() != Some(&Token::RBrace) && self.pos < self.tokens.len() {
            self.parse_prim_body_item(&mut prim)?;
        }

        self.expect(&Token::RBrace)?;
        Ok(Some(prim))
    }

    fn parse_prim_body_item(&mut self, prim: &mut Prim) -> Result<(), SolidError> {
        // Skip qualifiers
        let _custom  = self.eat(&Token::Custom);
        let _prepend = self.eat(&Token::Prepend);
        let _append  = self.eat(&Token::Append);
        let uniform  = self.eat(&Token::Uniform);

        match self.peek().cloned() {
            // ── Nested prim ────────────────────────────────────────────────
            Some(Token::Def) | Some(Token::Over) | Some(Token::Class) => {
                if let Some(child) = self.parse_prim()? {
                    prim.children.push(child);
                }
            }

            // ── Relationship  rel name = <target> ─────────────────────────
            Some(Token::Rel) => {
                self.advance();
                // relationship name may be qualified with colons (material:binding)
                let name = self.parse_qualified_name()?;
                let target = if self.eat(&Token::Equals) {
                    match self.peek().cloned() {
                        Some(Token::SdfPath(p)) => { self.advance(); Some(p) }
                        Some(Token::None)       => { self.advance(); None }
                        _ => None,
                    }
                } else { None };
                prim.relationships.push(Relationship { name, target });
            }

            // ── Attribute or inherit/variant-set/references (skip) ─────────
            Some(Token::Ident(type_name)) => {
                // Could be:
                // 1. `TypeName attrName = value`   (attribute)
                // 2. `inherits = <path>`            (skip)
                // 3. `references = @file@`          (skip)
                // 4. `variantSet "name" = { … }`    (skip)

                // Peek ahead two tokens to distinguish attribute from keywords
                if type_name == "inherits" || type_name == "references" || type_name == "payload" {
                    // skip to next `}` or next prim/attr start
                    self.advance(); // consume keyword
                    // skip optional = and value tokens until we reach a brace or new prim
                    if self.eat(&Token::Equals) {
                        self.skip_value();
                    }
                    return Ok(());
                }

                if type_name == "variantSet" {
                    self.skip_to_matching_brace();
                    return Ok(());
                }

                self.advance(); // consume type name

                let attr_name = self.parse_qualified_name()?;

                if self.eat(&Token::Equals) {
                    let val = self.parse_value()?;
                    let mut attr = Attribute::new(attr_name, type_name, val);
                    attr.uniform = uniform;
                    prim.attributes.push(attr);
                } else {
                    // Declaration without value — still record it.
                    let attr = Attribute {
                        name:      attr_name,
                        type_name,
                        value:     None,
                        uniform,
                    };
                    prim.attributes.push(attr);
                }
            }

            Some(Token::Inherits) | Some(Token::References) | Some(Token::Payload)
            | Some(Token::VariantSet) | Some(Token::Variant) => {
                self.skip_to_next_statement();
            }

            _ => {
                // Unknown token — skip it to avoid getting stuck.
                self.advance();
            }
        }

        Ok(())
    }

    // ── Value parsing ─────────────────────────────────────────────────────────

    fn parse_value(&mut self) -> Result<UsdValue, SolidError> {
        match self.peek().cloned() {
            Some(Token::Bool(b))      => { self.advance(); Ok(UsdValue::Bool(b)) }
            Some(Token::None)         => { self.advance(); Ok(UsdValue::String("None".into())) }
            Some(Token::Int(i))       => { self.advance(); Ok(UsdValue::Int(i)) }
            Some(Token::Float(f))     => { self.advance(); Ok(UsdValue::Float(f)) }
            Some(Token::StringLit(s)) => { self.advance(); Ok(UsdValue::String(s)) }
            Some(Token::AssetPath(s)) => { self.advance(); Ok(UsdValue::Asset(s)) }
            Some(Token::SdfPath(s))   => { self.advance(); Ok(UsdValue::String(s)) }
            Some(Token::Ident(s))     => { self.advance(); Ok(UsdValue::Token(s)) }
            Some(Token::LParen)       => self.parse_tuple(),
            Some(Token::LBracket)     => self.parse_array(),
            // Time-sampled block  { time: value, ... }  → extract first keyframe value
            Some(Token::LBrace)       => self.parse_time_samples(),
            Some(t) => Err(SolidError::parse(format!("expected value, got {t:?}"))),
            None    => Err(SolidError::parse("unexpected end of input in value")),
        }
    }

    /// Parse `(a, b)` or `(a, b, c)` tuples.
    fn parse_tuple(&mut self) -> Result<UsdValue, SolidError> {
        self.expect(&Token::LParen)?;
        let mut nums: Vec<f64> = Vec::new();
        while self.peek() != Some(&Token::RParen) {
            nums.push(self.parse_number()?);
            self.eat(&Token::Comma);
        }
        self.expect(&Token::RParen)?;
        match nums.len() {
            2 => Ok(UsdValue::Vec2f([nums[0], nums[1]])),
            3 => Ok(UsdValue::Vec3f([nums[0], nums[1], nums[2]])),
            4 => Ok(UsdValue::Vec4f([nums[0], nums[1], nums[2], nums[3]])),
            n => Err(SolidError::parse(format!("unsupported tuple length {n}"))),
        }
    }

    /// Parse `[value, value, …]` arrays.
    fn parse_array(&mut self) -> Result<UsdValue, SolidError> {
        self.expect(&Token::LBracket)?;

        if self.peek() == Some(&Token::RBracket) {
            self.advance();
            return Ok(UsdValue::Vec3fArray(vec![]));
        }

        // Peek to determine element type.
        match self.peek().cloned() {
            Some(Token::LParen) => {
                // array of tuples
                let mut items: Vec<[f64; 3]> = Vec::new();
                while self.peek() != Some(&Token::RBracket) {
                    self.expect(&Token::LParen)?;
                    let a = self.parse_number()?;
                    self.eat(&Token::Comma);
                    let b = self.parse_number()?;
                    self.eat(&Token::Comma);
                    // optional third element
                    let c = if self.peek() != Some(&Token::RParen) {
                        let v = self.parse_number()?;
                        self.eat(&Token::Comma);
                        v
                    } else { 0.0 };
                    self.expect(&Token::RParen)?;
                    items.push([a, b, c]);
                    self.eat(&Token::Comma);
                }
                self.expect(&Token::RBracket)?;
                Ok(UsdValue::Vec3fArray(items))
            }
            Some(Token::Int(_)) => {
                let mut items: Vec<i64> = Vec::new();
                while self.peek() != Some(&Token::RBracket) {
                    items.push(self.parse_int()?);
                    self.eat(&Token::Comma);
                }
                self.expect(&Token::RBracket)?;
                Ok(UsdValue::IntArray(items))
            }
            Some(Token::Float(_)) => {
                let mut items: Vec<f64> = Vec::new();
                while self.peek() != Some(&Token::RBracket) {
                    items.push(self.parse_number()?);
                    self.eat(&Token::Comma);
                }
                self.expect(&Token::RBracket)?;
                Ok(UsdValue::FloatArray(items))
            }
            Some(Token::StringLit(_)) => {
                let mut items: Vec<String> = Vec::new();
                while self.peek() != Some(&Token::RBracket) {
                    items.push(self.expect_string()?);
                    self.eat(&Token::Comma);
                }
                self.expect(&Token::RBracket)?;
                Ok(UsdValue::StringArray(items))
            }
            Some(Token::Ident(_)) => {
                let mut items: Vec<String> = Vec::new();
                while self.peek() != Some(&Token::RBracket) {
                    items.push(self.expect_ident()?);
                    self.eat(&Token::Comma);
                }
                self.expect(&Token::RBracket)?;
                Ok(UsdValue::TokenArray(items))
            }
            _ => {
                // Unknown array — skip and return empty
                self.skip_brackets(Token::LBracket, Token::RBracket);
                Ok(UsdValue::Vec3fArray(vec![]))
            }
        }
    }

    fn parse_number(&mut self) -> Result<f64, SolidError> {
        match self.advance() {
            Some(Token::Float(f)) => Ok(*f),
            Some(Token::Int(i))   => Ok(*i as f64),
            Some(t) => Err(SolidError::parse(format!("expected number, got {t:?}"))),
            None    => Err(SolidError::parse("unexpected end of input")),
        }
    }

    fn parse_int(&mut self) -> Result<i64, SolidError> {
        match self.advance() {
            Some(Token::Int(i))   => Ok(*i),
            Some(Token::Float(f)) => Ok(*f as i64),
            Some(t) => Err(SolidError::parse(format!("expected integer, got {t:?}"))),
            None    => Err(SolidError::parse("unexpected end of input")),
        }
    }

    /// Parses a (possibly colon-qualified) name like `xformOp:translate`.
    /// Since the lexer already merges colons into identifiers for most cases,
    /// this is usually just `expect_ident()`.
    fn parse_qualified_name(&mut self) -> Result<String, SolidError> {
        self.expect_ident()
    }

    // ── Skip helpers ─────────────────────────────────────────────────────────

    fn skip_value(&mut self) {
        match self.peek().cloned() {
            Some(Token::LParen)   => { self.skip_brackets(Token::LParen, Token::RParen); }
            Some(Token::LBracket) => { self.skip_brackets(Token::LBracket, Token::RBracket); }
            Some(Token::LBrace)   => { self.skip_brackets(Token::LBrace, Token::RBrace); }
            _ => { self.advance(); }
        }
    }

    fn skip_brackets(&mut self, open: Token, close: Token) {
        let mut depth = 0usize;
        while self.pos < self.tokens.len() {
            if self.tokens[self.pos] == open  { depth += 1; }
            if self.tokens[self.pos] == close {
                depth -= 1;
                self.pos += 1;
                if depth == 0 { break; }
                continue;
            }
            self.pos += 1;
        }
    }

    fn skip_to_matching_brace(&mut self) {
        self.skip_brackets(Token::LBrace, Token::RBrace);
    }

    fn skip_to_next_statement(&mut self) {
        // advance until we see something that looks like a new prim keyword,
        // attribute type, or closing brace.
        while self.pos < self.tokens.len() {
            match self.peek() {
                Some(Token::Def)
                | Some(Token::Over)
                | Some(Token::Class)
                | Some(Token::RBrace) => break,
                _ => { self.advance(); }
            }
        }
    }

    /// Parse a time-samples block `{ time: value, ... }` and return the first
    /// sample's value (or a placeholder).
    fn parse_time_samples(&mut self) -> Result<UsdValue, SolidError> {
        self.expect(&Token::LBrace)?;
        let mut first_val: Option<UsdValue> = None;

        while self.peek() != Some(&Token::RBrace) && self.pos < self.tokens.len() {
            // time key — may be a number or ident
            match self.peek().cloned() {
                Some(Token::Float(_)) | Some(Token::Int(_)) | Some(Token::Ident(_)) => {
                    self.advance();
                }
                _ => { self.advance(); continue; }
            }
            self.eat(&Token::Colon);
            match self.parse_value() {
                Ok(v) => {
                    if first_val.is_none() { first_val = Some(v); }
                }
                Err(_) => { self.skip_value(); }
            }
            self.eat(&Token::Comma);
        }
        self.expect(&Token::RBrace)?;
        Ok(first_val.unwrap_or(UsdValue::String("timeSamples".into())))
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn value_as_string(v: &UsdValue) -> Option<String> {
    match v {
        UsdValue::String(s) | UsdValue::Token(s) => Some(s.clone()),
        _ => None,
    }
}

fn value_as_f64(v: &UsdValue) -> Option<f64> {
    match v {
        UsdValue::Float(f) => Some(*f),
        UsdValue::Int(i)   => Some(*i as f64),
        _ => None,
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const SIMPLE_USDA: &str = r#"#usda 1.0
(
    upAxis = "Y"
    metersPerUnit = 0.01
    defaultPrim = "World"
)

def Xform "World" {
    def Mesh "Cube" {
        point3f[] points = [(0, 0, 0), (1, 0, 0), (1, 1, 0)]
        int[] faceVertexCounts = [3]
        int[] faceVertexIndices = [0, 1, 2]
        uniform token subdivisionScheme = "none"
    }
}
"#;

    #[test]
    fn parse_stage_meta() {
        let doc = parse(SIMPLE_USDA).unwrap();
        assert_eq!(doc.meta.up_axis.as_deref(), Some("Y"));
        assert!((doc.meta.meters_per_unit.unwrap() - 0.01).abs() < 1e-9);
        assert_eq!(doc.meta.default_prim.as_deref(), Some("World"));
    }

    #[test]
    fn parse_mesh_prim() {
        let doc = parse(SIMPLE_USDA).unwrap();
        let world = doc.root_prim("World").unwrap();
        assert_eq!(world.type_name, "Xform");
        let cube = world.children.iter().find(|c| c.name == "Cube").unwrap();
        assert_eq!(cube.type_name, "Mesh");

        let pts = cube.attr("points").unwrap();
        if let Some(UsdValue::Vec3fArray(pts)) = &pts.value {
            assert_eq!(pts.len(), 3);
        } else {
            panic!("points should be Vec3fArray");
        }
    }
}
