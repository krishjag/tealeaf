//! Parser for TeaLeaf text format

use std::collections::HashMap;
use std::path::Path;
use crate::{Error, Result, Value, Schema, Field, FieldType, Union, Variant};
use crate::lexer::{Token, TokenKind, Lexer};

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    schemas: HashMap<String, Schema>,
    unions: HashMap<String, Union>,
    base_path: Option<std::path::PathBuf>,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            pos: 0,
            schemas: HashMap::new(),
            unions: HashMap::new(),
            base_path: None,
        }
    }

    pub fn with_base_path(mut self, path: &Path) -> Self {
        self.base_path = path.parent().map(|p| p.to_path_buf());
        self
    }

    pub fn parse(&mut self) -> Result<HashMap<String, Value>> {
        let mut result = HashMap::new();

        while !self.at_end() {
            match self.current_kind() {
                TokenKind::Directive(d) => {
                    let directive = d.clone();
                    self.advance();
                    match directive.as_str() {
                        "struct" => self.parse_struct_def()?,
                        "union" => self.parse_union_def()?,
                        "include" => {
                            let included = self.parse_include()?;
                            for (k, v) in included {
                                result.insert(k, v);
                            }
                        }
                        _ => {}
                    }
                }
                TokenKind::Word(_) | TokenKind::String(_) => {
                    let (key, value) = self.parse_pair()?;
                    result.insert(key, value);
                }
                TokenKind::Ref(r) => {
                    let ref_name = r.clone();
                    self.advance();
                    self.expect(TokenKind::Colon)?;
                    let value = self.parse_value()?;
                    result.insert(format!("!{}", ref_name), value);
                }
                TokenKind::Eof => break,
                _ => { self.advance(); }
            }
        }

        Ok(result)
    }

    pub fn into_schemas(self) -> HashMap<String, Schema> {
        self.schemas
    }

    pub fn into_unions(self) -> HashMap<String, Union> {
        self.unions
    }

    // =========================================================================
    // Struct Definition
    // =========================================================================

    fn parse_struct_def(&mut self) -> Result<()> {
        let name = self.expect_word()?;
        self.expect(TokenKind::LParen)?;

        let mut schema = Schema::new(&name);

        while !self.check(TokenKind::RParen) {
            let field_name = self.expect_word()?;
            
            let field_type = if self.check(TokenKind::Colon) {
                self.advance();
                self.parse_field_type()?
            } else {
                FieldType::new("string")
            };

            schema.add_field(field_name, field_type);

            if self.check(TokenKind::Comma) {
                self.advance();
            }
        }

        self.expect(TokenKind::RParen)?;
        self.schemas.insert(name, schema);
        Ok(())
    }

    // =========================================================================
    // Union Definition
    // =========================================================================

    fn parse_union_def(&mut self) -> Result<()> {
        let name = self.expect_word()?;
        self.expect(TokenKind::LBrace)?;

        let mut union_type = Union::new(&name);

        while !self.check(TokenKind::RBrace) {
            let variant_name = self.expect_word()?;
            self.expect(TokenKind::LParen)?;

            let mut variant = Variant::new(&variant_name);

            while !self.check(TokenKind::RParen) {
                let field_name = self.expect_word()?;

                let field_type = if self.check(TokenKind::Colon) {
                    self.advance();
                    self.parse_field_type()?
                } else {
                    FieldType::new("string")
                };

                variant.fields.push(Field::new(field_name, field_type));

                if self.check(TokenKind::Comma) {
                    self.advance();
                }
            }

            self.expect(TokenKind::RParen)?;
            union_type.add_variant(variant);

            if self.check(TokenKind::Comma) {
                self.advance();
            }
        }

        self.expect(TokenKind::RBrace)?;
        self.unions.insert(name, union_type);
        Ok(())
    }

    // =========================================================================
    // Include Directive
    // =========================================================================

    fn parse_include(&mut self) -> Result<HashMap<String, Value>> {
        let path_str = match self.current_kind() {
            TokenKind::String(s) => s.clone(),
            TokenKind::Word(w) => w.clone(),
            _ => return Err(Error::UnexpectedToken {
                expected: "file path".to_string(),
                got: format!("{:?}", self.current_kind()),
            }),
        };
        self.advance();

        // Resolve path relative to current file
        let include_path = if let Some(ref base) = self.base_path {
            base.join(&path_str)
        } else {
            std::path::PathBuf::from(&path_str)
        };

        // Read and parse the included file
        let content = std::fs::read_to_string(&include_path)
            .map_err(|e| Error::ParseError(format!("Failed to include {}: {}", path_str, e)))?;

        let tokens = Lexer::new(&content).tokenize()?;
        let mut parser = Parser::new(tokens);
        if let Some(parent) = include_path.parent() {
            parser.base_path = Some(parent.to_path_buf());
        }

        let data = parser.parse()?;

        // Merge schemas and unions
        for (name, schema) in parser.schemas {
            self.schemas.insert(name, schema);
        }
        for (name, union_type) in parser.unions {
            self.unions.insert(name, union_type);
        }

        Ok(data)
    }

    fn parse_field_type(&mut self) -> Result<FieldType> {
        let mut type_str = String::new();
        
        // Handle array prefix
        if self.check(TokenKind::LBracket) {
            self.advance();
            self.expect(TokenKind::RBracket)?;
            type_str.push_str("[]");
        }

        // Base type
        type_str.push_str(&self.expect_word()?);

        // Nullable suffix
        if self.check(TokenKind::Question) {
            self.advance();
            type_str.push('?');
        }

        Ok(FieldType::parse(&type_str))
    }

    // =========================================================================
    // Key-Value Pairs
    // =========================================================================

    fn parse_pair(&mut self) -> Result<(String, Value)> {
        let key = match self.current_kind() {
            TokenKind::Word(w) => w.clone(),
            TokenKind::String(s) => s.clone(),
            _ => return Err(Error::UnexpectedToken {
                expected: "key".to_string(),
                got: format!("{:?}", self.current_kind()),
            }),
        };
        self.advance();
        self.expect(TokenKind::Colon)?;
        let value = self.parse_value()?;
        Ok((key, value))
    }

    // =========================================================================
    // Values
    // =========================================================================

    fn parse_value(&mut self) -> Result<Value> {
        match self.current_kind() {
            TokenKind::Null => { self.advance(); Ok(Value::Null) }
            TokenKind::Bool(b) => { let b = *b; self.advance(); Ok(Value::Bool(b)) }
            TokenKind::Int(i) => { let i = *i; self.advance(); Ok(Value::Int(i)) }
            TokenKind::Float(f) => { let f = *f; self.advance(); Ok(Value::Float(f)) }
            TokenKind::String(s) => { let s = s.clone(); self.advance(); Ok(Value::String(s)) }
            TokenKind::Word(w) => { let w = w.clone(); self.advance(); Ok(Value::String(w)) }
            TokenKind::Ref(r) => { let r = r.clone(); self.advance(); Ok(Value::Ref(r)) }
            TokenKind::Timestamp(ts) => { let ts = *ts; self.advance(); Ok(Value::Timestamp(ts)) }
            TokenKind::Tag(t) => {
                let tag = t.clone();
                self.advance();
                let inner = self.parse_value()?;
                Ok(Value::Tagged(tag, Box::new(inner)))
            }
            TokenKind::Directive(d) => {
                let directive = d.clone();
                self.advance();
                self.parse_directive_value(&directive)
            }
            TokenKind::LBrace => self.parse_object(),
            TokenKind::LBracket => self.parse_array(),
            TokenKind::LParen => self.parse_tuple(),
            _ => Err(Error::UnexpectedToken {
                expected: "value".to_string(),
                got: format!("{:?}", self.current_kind()),
            }),
        }
    }

    fn parse_directive_value(&mut self, directive: &str) -> Result<Value> {
        match directive {
            "table" => self.parse_table(),
            "map" => self.parse_map(),
            _ => Ok(Value::Null),
        }
    }

    fn parse_map(&mut self) -> Result<Value> {
        self.expect(TokenKind::LBrace)?;
        let mut pairs = Vec::new();

        while !self.check(TokenKind::RBrace) {
            // Parse key (can be string, int, or word)
            let key = match self.current_kind() {
                TokenKind::String(s) => { let s = s.clone(); self.advance(); Value::String(s) }
                TokenKind::Word(w) => { let w = w.clone(); self.advance(); Value::String(w) }
                TokenKind::Int(i) => { let i = *i; self.advance(); Value::Int(i) }
                _ => return Err(Error::UnexpectedToken {
                    expected: "map key".to_string(),
                    got: format!("{:?}", self.current_kind()),
                }),
            };

            self.expect(TokenKind::Colon)?;
            let value = self.parse_value()?;
            pairs.push((key, value));

            if self.check(TokenKind::Comma) {
                self.advance();
            }
        }

        self.expect(TokenKind::RBrace)?;
        Ok(Value::Map(pairs))
    }

    fn parse_table(&mut self) -> Result<Value> {
        let struct_name = self.expect_word()?;
        let schema = self.schemas
            .get(&struct_name)
            .ok_or_else(|| Error::UnknownStruct(struct_name.clone()))?
            .clone();

        self.expect(TokenKind::LBracket)?;

        let mut rows = Vec::new();
        while !self.check(TokenKind::RBracket) {
            let row = self.parse_tuple_with_schema(&schema)?;
            rows.push(row);
            if self.check(TokenKind::Comma) {
                self.advance();
            }
        }

        self.expect(TokenKind::RBracket)?;
        Ok(Value::Array(rows))
    }

    fn parse_tuple_with_schema(&mut self, schema: &Schema) -> Result<Value> {
        self.expect(TokenKind::LParen)?;

        let mut obj = HashMap::new();
        for field in &schema.fields {
            let value = self.parse_value_for_field(&field.field_type)?;
            obj.insert(field.name.clone(), value);
            if self.check(TokenKind::Comma) {
                self.advance();
            }
        }

        self.expect(TokenKind::RParen)?;
        Ok(Value::Object(obj))
    }

    fn parse_value_for_field(&mut self, field_type: &FieldType) -> Result<Value> {
        // Handle null
        if self.check(TokenKind::Null) {
            self.advance();
            return Ok(Value::Null);
        }

        // Handle nested struct
        if field_type.is_struct() {
            if let Some(schema) = self.schemas.get(&field_type.base).cloned() {
                return self.parse_tuple_with_schema(&schema);
            }
        }

        // Handle array
        if field_type.is_array {
            self.expect(TokenKind::LBracket)?;
            let mut arr = Vec::new();
            let inner_type = FieldType::new(&field_type.base);
            while !self.check(TokenKind::RBracket) {
                arr.push(self.parse_value_for_field(&inner_type)?);
                if self.check(TokenKind::Comma) {
                    self.advance();
                }
            }
            self.expect(TokenKind::RBracket)?;
            return Ok(Value::Array(arr));
        }

        // Regular value
        self.parse_value()
    }

    fn parse_object(&mut self) -> Result<Value> {
        self.expect(TokenKind::LBrace)?;
        let mut obj = HashMap::new();

        while !self.check(TokenKind::RBrace) {
            if let TokenKind::Ref(r) = self.current_kind() {
                let key = format!("!{}", r);
                self.advance();
                self.expect(TokenKind::Colon)?;
                let value = self.parse_value()?;
                obj.insert(key, value);
            } else {
                let (key, value) = self.parse_pair()?;
                obj.insert(key, value);
            }
            if self.check(TokenKind::Comma) {
                self.advance();
            }
        }

        self.expect(TokenKind::RBrace)?;
        Ok(Value::Object(obj))
    }

    fn parse_array(&mut self) -> Result<Value> {
        self.expect(TokenKind::LBracket)?;
        let mut arr = Vec::new();

        while !self.check(TokenKind::RBracket) {
            arr.push(self.parse_value()?);
            if self.check(TokenKind::Comma) {
                self.advance();
            }
        }

        self.expect(TokenKind::RBracket)?;
        Ok(Value::Array(arr))
    }

    fn parse_tuple(&mut self) -> Result<Value> {
        self.expect(TokenKind::LParen)?;
        let mut arr = Vec::new();

        while !self.check(TokenKind::RParen) {
            arr.push(self.parse_value()?);
            if self.check(TokenKind::Comma) {
                self.advance();
            }
        }

        self.expect(TokenKind::RParen)?;
        Ok(Value::Array(arr))
    }

    // =========================================================================
    // Helpers
    // =========================================================================

    fn current(&self) -> &Token {
        self.tokens.get(self.pos).unwrap_or(&Token {
            kind: TokenKind::Eof,
            line: 0,
            col: 0,
        })
    }

    fn current_kind(&self) -> &TokenKind {
        &self.current().kind
    }

    fn advance(&mut self) {
        if self.pos < self.tokens.len() {
            self.pos += 1;
        }
    }

    fn check(&self, expected: TokenKind) -> bool {
        std::mem::discriminant(self.current_kind()) == std::mem::discriminant(&expected)
    }

    fn expect(&mut self, expected: TokenKind) -> Result<()> {
        if self.check(expected.clone()) {
            self.advance();
            Ok(())
        } else {
            Err(Error::UnexpectedToken {
                expected: format!("{:?}", expected),
                got: format!("{:?}", self.current_kind()),
            })
        }
    }

    fn expect_word(&mut self) -> Result<String> {
        match self.current_kind() {
            TokenKind::Word(w) => {
                let w = w.clone();
                self.advance();
                Ok(w)
            }
            _ => Err(Error::UnexpectedToken {
                expected: "word".to_string(),
                got: format!("{:?}", self.current_kind()),
            }),
        }
    }

    fn at_end(&self) -> bool {
        matches!(self.current_kind(), TokenKind::Eof)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;

    fn parse(input: &str) -> Result<HashMap<String, Value>> {
        let tokens = Lexer::new(input).tokenize()?;
        Parser::new(tokens).parse()
    }

    #[test]
    fn test_simple_values() {
        let data = parse("a: 1, b: hello, c: true, d: ~").unwrap();
        assert_eq!(data.get("a").unwrap().as_int(), Some(1));
        assert_eq!(data.get("b").unwrap().as_str(), Some("hello"));
        assert_eq!(data.get("c").unwrap().as_bool(), Some(true));
        assert!(data.get("d").unwrap().is_null());
    }

    #[test]
    fn test_object() {
        let data = parse("obj: {x: 1, y: 2}").unwrap();
        let obj = data.get("obj").unwrap().as_object().unwrap();
        assert_eq!(obj.get("x").unwrap().as_int(), Some(1));
        assert_eq!(obj.get("y").unwrap().as_int(), Some(2));
    }

    #[test]
    fn test_array() {
        let data = parse("arr: [1, 2, 3]").unwrap();
        let arr = data.get("arr").unwrap().as_array().unwrap();
        assert_eq!(arr.len(), 3);
        assert_eq!(arr[0].as_int(), Some(1));
    }

    #[test]
    fn test_struct_and_table() {
        let input = r#"
            @struct point (x: int, y: int)
            points: @table point [
                (1, 2),
                (3, 4),
            ]
        "#;
        let tokens = Lexer::new(input).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let data = parser.parse().unwrap();
        
        let points = data.get("points").unwrap().as_array().unwrap();
        assert_eq!(points.len(), 2);
        
        let p0 = points[0].as_object().unwrap();
        assert_eq!(p0.get("x").unwrap().as_int(), Some(1));
        assert_eq!(p0.get("y").unwrap().as_int(), Some(2));
    }
}
