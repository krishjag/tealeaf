//! Parser for TeaLeaf text format

use std::path::Path;
use indexmap::IndexMap;
use crate::{Error, Result, Value, Schema, Field, FieldType, Union, Variant};
use crate::types::ObjectMap;
use crate::lexer::{Token, TokenKind, Lexer};

/// Maximum recursion depth for nested parse_value calls (arrays, objects, maps, tuples, tags).
/// Matches the binary reader's MAX_DECODE_DEPTH to ensure text↔binary parity.
const MAX_PARSE_DEPTH: usize = 256;

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    schemas: IndexMap<String, Schema>,
    unions: IndexMap<String, Union>,
    base_path: Option<std::path::PathBuf>,
    /// Tracks included file paths for cycle detection
    include_stack: Vec<std::path::PathBuf>,
    /// Indicates the source was a root-level JSON array (set by @root-array directive)
    is_root_array: bool,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            pos: 0,
            schemas: IndexMap::new(),
            unions: IndexMap::new(),
            base_path: None,
            include_stack: Vec::new(),
            is_root_array: false,
        }
    }

    pub fn with_base_path(mut self, path: &Path) -> Self {
        self.base_path = path.parent().map(|p| p.to_path_buf());
        self
    }

    pub fn parse(&mut self) -> Result<IndexMap<String, Value>> {
        let mut result = IndexMap::new();

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
                        "root-array" => {
                            // Marks this document as representing a root-level JSON array
                            self.is_root_array = true;
                        }
                        _ => {
                            // Unknown top-level directive: silently ignored (spec §1.18).
                            // Consume same-line argument for forward compatibility —
                            // a future directive like @custom foo should not leave
                            // "foo" to be misparsed as a key.
                            let directive_line = self.tokens[self.pos - 1].line;
                            if !self.at_end()
                                && self.current().line == directive_line
                                && self.can_start_value()
                            {
                                let _ = self.parse_value(0)?;
                            }
                        }
                    }
                }
                TokenKind::Word(_) | TokenKind::String(_) => {
                    let (key, value) = self.parse_pair(0)?;
                    result.insert(key, value);
                }
                TokenKind::Ref(r) => {
                    let ref_name = r.clone();
                    self.advance();
                    self.expect(TokenKind::Colon)?;
                    let value = self.parse_value(0)?;
                    result.insert(format!("!{}", ref_name), value);
                }
                TokenKind::Eof => break,
                _ => { self.advance(); }
            }
        }

        Ok(result)
    }

    pub fn into_schemas(self) -> IndexMap<String, Schema> {
        self.schemas
    }

    pub fn into_unions(self) -> IndexMap<String, Union> {
        self.unions
    }

    /// Consume the parser and return both schemas and unions.
    pub fn into_schemas_and_unions(self) -> (IndexMap<String, Schema>, IndexMap<String, Union>) {
        (self.schemas, self.unions)
    }

    /// Check if the @root-array directive was present
    pub fn is_root_array(&self) -> bool {
        self.is_root_array
    }

    // =========================================================================
    // Struct Definition
    // =========================================================================

    fn parse_struct_def(&mut self) -> Result<()> {
        let name = self.expect_word()?;
        self.expect(TokenKind::LParen)?;

        let mut schema = Schema::new(&name);

        while !self.check(TokenKind::RParen) {
            // Field names must be unquoted names per spec grammar
            let field_name = match self.current_kind() {
                TokenKind::Word(w) => {
                    let w = w.clone();
                    self.advance();
                    w
                }
                _ => return Err(Error::UnexpectedToken {
                    expected: "field name".to_string(),
                    got: format!("{:?}", self.current_kind()),
                }),
            };

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

    fn parse_include(&mut self) -> Result<IndexMap<String, Value>> {
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

        // Cycle detection and depth limit
        let canonical = include_path.canonicalize()
            .unwrap_or_else(|_| include_path.clone());
        if self.include_stack.contains(&canonical) {
            return Err(Error::ParseError(format!(
                "Circular include detected: {}", canonical.display()
            )));
        }
        if self.include_stack.len() >= 32 {
            return Err(Error::ParseError(
                "Include depth exceeds limit of 32".into()
            ));
        }

        // Read and parse the included file
        let content = std::fs::read_to_string(&include_path)
            .map_err(|e| Error::ParseError(format!("Failed to include {}: {}", path_str, e)))?;

        let tokens = Lexer::new(&content).tokenize()?;
        let mut parser = Parser::new(tokens);
        if let Some(parent) = include_path.parent() {
            parser.base_path = Some(parent.to_path_buf());
        }
        // Propagate include stack and accumulated schemas/unions to child parser
        // so that schemas from earlier includes are available in later includes.
        parser.include_stack = self.include_stack.clone();
        parser.include_stack.push(canonical);
        parser.schemas = self.schemas.clone();
        parser.unions = self.unions.clone();

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
        let base = self.expect_word()?;

        // Reject value-only types that cannot be schema field types (spec §2.1)
        match base.as_str() {
            "object" | "map" | "tuple" | "ref" | "tagged" => {
                return Err(Error::ParseError(
                    format!("'{}' is a value type and cannot be used as a schema field type", base)
                ));
            }
            _ => {}
        }

        type_str.push_str(&base);

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

    fn parse_pair(&mut self, depth: usize) -> Result<(String, Value)> {
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
        let value = self.parse_value(depth)?;
        Ok((key, value))
    }

    // =========================================================================
    // Values
    // =========================================================================

    fn parse_value(&mut self, depth: usize) -> Result<Value> {
        if depth > MAX_PARSE_DEPTH {
            return Err(Error::ParseError("maximum parse nesting depth exceeded".into()));
        }
        match self.current_kind() {
            TokenKind::Null => { self.advance(); Ok(Value::Null) }
            TokenKind::Bool(b) => { let b = *b; self.advance(); Ok(Value::Bool(b)) }
            TokenKind::Int(i) => { let i = *i; self.advance(); Ok(Value::Int(i)) }
            TokenKind::UInt(u) => { let u = *u; self.advance(); Ok(Value::UInt(u)) }
            TokenKind::JsonNumber(s) => { let s = s.clone(); self.advance(); Ok(Value::JsonNumber(s)) }
            TokenKind::Float(f) => { let f = *f; self.advance(); Ok(Value::Float(f)) }
            TokenKind::String(s) => { let s = s.clone(); self.advance(); Ok(Value::String(s)) }
            TokenKind::Bytes(b) => { let b = b.clone(); self.advance(); Ok(Value::Bytes(b)) }
            TokenKind::Word(w) => { let w = w.clone(); self.advance(); Ok(Value::String(w)) }
            TokenKind::Ref(r) => { let r = r.clone(); self.advance(); Ok(Value::Ref(r)) }
            TokenKind::Timestamp(ts, tz) => { let ts = *ts; let tz = *tz; self.advance(); Ok(Value::Timestamp(ts, tz)) }
            TokenKind::Tag(t) => {
                let tag = t.clone();
                self.advance();
                let inner = self.parse_value(depth + 1)?;
                Ok(Value::Tagged(tag, Box::new(inner)))
            }
            TokenKind::Directive(d) => {
                let directive = d.clone();
                self.advance();
                self.parse_directive_value(&directive, depth)
            }
            TokenKind::LBrace => self.parse_object(depth + 1),
            TokenKind::LBracket => self.parse_array(depth + 1),
            TokenKind::LParen => self.parse_tuple(depth + 1),
            _ => Err(Error::UnexpectedToken {
                expected: "value".to_string(),
                got: format!("{:?}", self.current_kind()),
            }),
        }
    }

    fn parse_directive_value(&mut self, directive: &str, depth: usize) -> Result<Value> {
        match directive {
            "table" => self.parse_table(depth),
            "map" => self.parse_map(depth),
            _ => {
                // Unknown directive in value position: consume argument, return null (spec §1.18)
                if self.can_start_value() {
                    let _ = self.parse_value(depth)?;
                }
                Ok(Value::Null)
            }
        }
    }

    /// Returns true if the current token can begin a value expression.
    fn can_start_value(&self) -> bool {
        matches!(
            self.current_kind(),
            TokenKind::Null
                | TokenKind::Bool(_)
                | TokenKind::Int(_)
                | TokenKind::UInt(_)
                | TokenKind::Float(_)
                | TokenKind::String(_)
                | TokenKind::Bytes(_)
                | TokenKind::Word(_)
                | TokenKind::Ref(_)
                | TokenKind::Timestamp(_, _)
                | TokenKind::JsonNumber(_)
                | TokenKind::Tag(_)
                | TokenKind::Directive(_)
                | TokenKind::LBrace
                | TokenKind::LBracket
                | TokenKind::LParen
        )
    }

    fn parse_map(&mut self, depth: usize) -> Result<Value> {
        self.expect(TokenKind::LBrace)?;
        let mut pairs = Vec::new();

        while !self.check(TokenKind::RBrace) {
            // Parse key (string, name, or integer per spec grammar:
            // map_key = string | name | integer)
            let key = match self.current_kind() {
                TokenKind::String(s) => { let s = s.clone(); self.advance(); Value::String(s) }
                TokenKind::Word(w) => { let w = w.clone(); self.advance(); Value::String(w) }
                TokenKind::Int(i) => { let i = *i; self.advance(); Value::Int(i) }
                TokenKind::UInt(u) => { let u = *u; self.advance(); Value::UInt(u) }
                _ => return Err(Error::UnexpectedToken {
                    expected: "map key".to_string(),
                    got: format!("{:?}", self.current_kind()),
                }),
            };

            self.expect(TokenKind::Colon)?;
            let value = self.parse_value(depth + 1)?;
            pairs.push((key, value));

            if self.check(TokenKind::Comma) {
                self.advance();
            }
        }

        self.expect(TokenKind::RBrace)?;
        Ok(Value::Map(pairs))
    }

    fn parse_table(&mut self, depth: usize) -> Result<Value> {
        let struct_name = self.expect_word()?;
        let schema = self.schemas
            .get(&struct_name)
            .ok_or_else(|| Error::UnknownStruct(struct_name.clone()))?
            .clone();

        self.expect(TokenKind::LBracket)?;

        let mut rows = Vec::new();
        while !self.check(TokenKind::RBracket) {
            let row = self.parse_tuple_with_schema(&schema, depth + 1)?;
            rows.push(row);
            if self.check(TokenKind::Comma) {
                self.advance();
            }
        }

        self.expect(TokenKind::RBracket)?;
        Ok(Value::Array(rows))
    }

    fn parse_tuple_with_schema(&mut self, schema: &Schema, depth: usize) -> Result<Value> {
        self.expect(TokenKind::LParen)?;

        let mut obj = ObjectMap::new();
        for field in &schema.fields {
            let value = self.parse_value_for_field(&field.field_type, depth)?;
            obj.insert(field.name.clone(), value);
            if self.check(TokenKind::Comma) {
                self.advance();
            }
        }

        self.expect(TokenKind::RParen)?;
        Ok(Value::Object(obj))
    }

    fn parse_value_for_field(&mut self, field_type: &FieldType, depth: usize) -> Result<Value> {
        // Handle null
        if self.check(TokenKind::Null) {
            self.advance();
            return Ok(Value::Null);
        }

        // Handle nested struct — schema names shadow built-in types, so check
        // by name rather than relying on is_struct() which lacks schema context.
        // The LParen guard disambiguates: struct tuples always start with `(`,
        // while primitive values (int, bool, string, etc.) never do.
        if !field_type.is_array && self.check(TokenKind::LParen) {
            if let Some(schema) = self.schemas.get(&field_type.base).cloned() {
                return self.parse_tuple_with_schema(&schema, depth + 1);
            }
        }

        // Handle array
        if field_type.is_array {
            self.expect(TokenKind::LBracket)?;
            let mut arr = Vec::new();
            let inner_type = FieldType::new(&field_type.base);
            while !self.check(TokenKind::RBracket) {
                arr.push(self.parse_value_for_field(&inner_type, depth + 1)?);
                if self.check(TokenKind::Comma) {
                    self.advance();
                }
            }
            self.expect(TokenKind::RBracket)?;
            return Ok(Value::Array(arr));
        }

        // Regular value
        self.parse_value(depth)
    }

    fn parse_object(&mut self, depth: usize) -> Result<Value> {
        self.expect(TokenKind::LBrace)?;
        let mut obj = ObjectMap::new();

        while !self.check(TokenKind::RBrace) {
            if let TokenKind::Ref(r) = self.current_kind() {
                let key = format!("!{}", r);
                self.advance();
                self.expect(TokenKind::Colon)?;
                let value = self.parse_value(depth)?;
                obj.insert(key, value);
            } else {
                let (key, value) = self.parse_pair(depth)?;
                obj.insert(key, value);
            }
            if self.check(TokenKind::Comma) {
                self.advance();
            }
        }

        self.expect(TokenKind::RBrace)?;
        Ok(Value::Object(obj))
    }

    fn parse_array(&mut self, depth: usize) -> Result<Value> {
        self.expect(TokenKind::LBracket)?;
        let mut arr = Vec::new();

        while !self.check(TokenKind::RBracket) {
            arr.push(self.parse_value(depth)?);
            if self.check(TokenKind::Comma) {
                self.advance();
            }
        }

        self.expect(TokenKind::RBracket)?;
        Ok(Value::Array(arr))
    }

    fn parse_tuple(&mut self, depth: usize) -> Result<Value> {
        self.expect(TokenKind::LParen)?;
        let mut arr = Vec::new();

        while !self.check(TokenKind::RParen) {
            arr.push(self.parse_value(depth)?);
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

    fn parse(input: &str) -> Result<IndexMap<String, Value>> {
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

    // -------------------------------------------------------------------------
    // Union parsing
    // -------------------------------------------------------------------------

    #[test]
    fn test_union_def() {
        let input = r#"
            @union Shape {
                Circle(radius: float),
                Rectangle(width: float, height: float),
                Point(),
            }
        "#;
        let tokens = Lexer::new(input).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        parser.parse().unwrap();
        let unions = parser.into_unions();
        let shape = unions.get("Shape").unwrap();
        assert_eq!(shape.variants.len(), 3);
        assert_eq!(shape.variants[0].name, "Circle");
        assert_eq!(shape.variants[0].fields.len(), 1);
        assert_eq!(shape.variants[1].name, "Rectangle");
        assert_eq!(shape.variants[1].fields.len(), 2);
        assert_eq!(shape.variants[2].name, "Point");
        assert_eq!(shape.variants[2].fields.len(), 0);
    }

    // -------------------------------------------------------------------------
    // Map parsing
    // -------------------------------------------------------------------------

    #[test]
    fn test_map_value() {
        let data = parse("m: @map {1: one, 2: two}").unwrap();
        let m = data.get("m").unwrap().as_map().unwrap();
        assert_eq!(m.len(), 2);
        assert_eq!(m[0].0.as_int(), Some(1));
        assert_eq!(m[0].1.as_str(), Some("one"));
        assert_eq!(m[1].0.as_int(), Some(2));
        assert_eq!(m[1].1.as_str(), Some("two"));
    }

    #[test]
    fn test_map_with_string_keys() {
        let data = parse(r#"m: @map {"key1": 10, "key2": 20}"#).unwrap();
        let m = data.get("m").unwrap().as_map().unwrap();
        assert_eq!(m.len(), 2);
    }

    #[test]
    fn test_map_empty() {
        let data = parse("m: @map {}").unwrap();
        let m = data.get("m").unwrap().as_map().unwrap();
        assert_eq!(m.len(), 0);
    }

    // -------------------------------------------------------------------------
    // Ref and Tagged values
    // -------------------------------------------------------------------------

    #[test]
    fn test_ref_value() {
        let data = parse("config: !base_config").unwrap();
        assert_eq!(data.get("config").unwrap().as_ref_name(), Some("base_config"));
    }

    #[test]
    fn test_tagged_value() {
        let data = parse("status: :ok 200").unwrap();
        let (tag, inner) = data.get("status").unwrap().as_tagged().unwrap();
        assert_eq!(tag, "ok");
        assert_eq!(inner.as_int(), Some(200));
    }

    #[test]
    fn test_tagged_null() {
        let data = parse("status: :none ~").unwrap();
        let (tag, inner) = data.get("status").unwrap().as_tagged().unwrap();
        assert_eq!(tag, "none");
        assert!(inner.is_null());
    }

    // -------------------------------------------------------------------------
    // Tuple and nested structures
    // -------------------------------------------------------------------------

    #[test]
    fn test_tuple_value() {
        let data = parse("point: (1, 2, 3)").unwrap();
        let arr = data.get("point").unwrap().as_array().unwrap();
        assert_eq!(arr.len(), 3);
        assert_eq!(arr[0].as_int(), Some(1));
        assert_eq!(arr[1].as_int(), Some(2));
        assert_eq!(arr[2].as_int(), Some(3));
    }

    #[test]
    fn test_nested_object() {
        let data = parse("outer: {inner: {x: 1}}").unwrap();
        let outer = data.get("outer").unwrap().as_object().unwrap();
        let inner = outer.get("inner").unwrap().as_object().unwrap();
        assert_eq!(inner.get("x").unwrap().as_int(), Some(1));
    }

    #[test]
    fn test_nested_arrays() {
        let data = parse("matrix: [[1, 2], [3, 4]]").unwrap();
        let matrix = data.get("matrix").unwrap().as_array().unwrap();
        assert_eq!(matrix.len(), 2);
        let row0 = matrix[0].as_array().unwrap();
        assert_eq!(row0[0].as_int(), Some(1));
    }

    // -------------------------------------------------------------------------
    // Struct fields with types
    // -------------------------------------------------------------------------

    #[test]
    fn test_struct_with_nullable_field() {
        let input = r#"
            @struct user (name: string, email: string?)
            users: @table user [
                (alice, "a@test.com"),
                (bob, ~),
            ]
        "#;
        let tokens = Lexer::new(input).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let data = parser.parse().unwrap();
        let schemas = parser.into_schemas();

        let schema = schemas.get("user").unwrap();
        assert!(schema.fields[1].field_type.nullable);

        let users = data.get("users").unwrap().as_array().unwrap();
        assert_eq!(users.len(), 2);
        assert!(users[1].as_object().unwrap().get("email").unwrap().is_null());
    }

    #[test]
    fn test_struct_with_array_field() {
        let input = r#"
            @struct item (name: string, tags: []string)
            items: @table item [
                (widget, [cool, useful]),
            ]
        "#;
        let tokens = Lexer::new(input).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let data = parser.parse().unwrap();

        let items = data.get("items").unwrap().as_array().unwrap();
        let tags = items[0].as_object().unwrap().get("tags").unwrap().as_array().unwrap();
        assert_eq!(tags.len(), 2);
    }

    // -------------------------------------------------------------------------
    // Root-array directive
    // -------------------------------------------------------------------------

    #[test]
    fn test_root_array_directive() {
        let input = "@root-array\nroot: [1, 2, 3]";
        let tokens = Lexer::new(input).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        parser.parse().unwrap();
        assert!(parser.is_root_array());
    }

    // -------------------------------------------------------------------------
    // Ref key at top level
    // -------------------------------------------------------------------------

    #[test]
    fn test_ref_key_at_top_level() {
        let input = "!defaults: {theme: dark}";
        let data = parse(input).unwrap();
        assert!(data.contains_key("!defaults"));
        let obj = data.get("!defaults").unwrap().as_object().unwrap();
        assert_eq!(obj.get("theme").unwrap().as_str(), Some("dark"));
    }

    // -------------------------------------------------------------------------
    // String keys
    // -------------------------------------------------------------------------

    #[test]
    fn test_string_key() {
        let data = parse(r#""my key": 42"#).unwrap();
        assert_eq!(data.get("my key").unwrap().as_int(), Some(42));
    }

    // -------------------------------------------------------------------------
    // Error cases
    // -------------------------------------------------------------------------

    #[test]
    fn test_unexpected_token_error() {
        let result = parse("] invalid");
        // The parser may skip unexpected tokens or error
        // Just ensure it doesn't panic
        let _ = result;
    }

    #[test]
    fn test_missing_colon_error() {
        // A word followed by a value without colon
        let input = "key value";
        let result = parse(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_unknown_struct_in_table() {
        let input = "data: @table nonexistent [(1, 2)]";
        let result = parse(input);
        assert!(result.is_err());
    }

    // -------------------------------------------------------------------------
    // Struct field type defaults
    // -------------------------------------------------------------------------

    #[test]
    fn test_struct_field_without_type() {
        let input = r#"
            @struct simple (name, value)
            items: @table simple [
                (hello, world),
            ]
        "#;
        let tokens = Lexer::new(input).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let data = parser.parse().unwrap();
        let schemas = parser.into_schemas();

        // Fields without explicit type default to "string"
        let schema = schemas.get("simple").unwrap();
        assert_eq!(schema.fields[0].field_type.base, "string");
        assert_eq!(schema.fields[1].field_type.base, "string");

        let items = data.get("items").unwrap().as_array().unwrap();
        assert_eq!(items[0].as_object().unwrap().get("name").unwrap().as_str(), Some("hello"));
    }

    // -------------------------------------------------------------------------
    // Unknown directive
    // -------------------------------------------------------------------------

    #[test]
    fn test_unknown_directive_ignored() {
        // Directive on its own line — next line is a key-value, not an argument
        let data = parse("@custom_directive\nkey: value").unwrap();
        assert_eq!(data.get("key").unwrap().as_str(), Some("value"));
    }

    #[test]
    fn test_unknown_directive_consumes_same_line_argument() {
        // Same-line word argument: consumed, not misparsed as a key
        let data = parse("@custom foo\nkey: value").unwrap();
        assert!(data.get("foo").is_none(), "foo should be consumed as directive arg, not a key");
        assert_eq!(data.get("key").unwrap().as_str(), Some("value"));

        // Same-line array argument
        let data = parse("@custom [1, 2, 3]\nkey: value").unwrap();
        assert_eq!(data.get("key").unwrap().as_str(), Some("value"));

        // Same-line object argument
        let data = parse("@custom {a: 1}\nkey: value").unwrap();
        assert_eq!(data.get("key").unwrap().as_str(), Some("value"));

        // No argument (directive alone on line)
        let data = parse("@custom\nkey: value").unwrap();
        assert_eq!(data.get("key").unwrap().as_str(), Some("value"));

        // No argument (directive at end of file)
        let data = parse("key: value\n@custom").unwrap();
        assert_eq!(data.get("key").unwrap().as_str(), Some("value"));

        // Argument on next line: NOT consumed
        let data = parse("@custom\nfoo: bar").unwrap();
        assert_eq!(data.get("foo").unwrap().as_str(), Some("bar"));
    }

    #[test]
    fn test_unknown_directive_value_consumes_argument() {
        // Spec §1.18: unknown directive in value position consumes argument, returns null
        let data = parse("key: @unknown [1, 2, 3]\nother: 42").unwrap();
        assert!(data.get("key").unwrap().is_null(), "unknown directive value should be null");
        assert_eq!(data.get("other").unwrap().as_int(), Some(42), "next key should parse normally");

        // With object argument
        let data = parse("key: @unknown {a: 1}\nother: ok").unwrap();
        assert!(data.get("key").unwrap().is_null());
        assert_eq!(data.get("other").unwrap().as_str(), Some("ok"));

        // With simple argument
        let data = parse("key: @unknown 42\nother: ok").unwrap();
        assert!(data.get("key").unwrap().is_null());
        assert_eq!(data.get("other").unwrap().as_str(), Some("ok"));

        // Without argument (just the directive)
        let data = parse("arr: [@unknown, 1, 2]").unwrap();
        let arr = data.get("arr").unwrap().as_array().unwrap();
        assert!(arr[0].is_null());
        assert_eq!(arr[1].as_int(), Some(1));
    }

    // -------------------------------------------------------------------------
    // Object with ref keys
    // -------------------------------------------------------------------------

    #[test]
    fn test_object_with_ref_key() {
        let data = parse("obj: {!base: 1, key: 2}").unwrap();
        let obj = data.get("obj").unwrap().as_object().unwrap();
        assert!(obj.contains_key("!base"));
        assert_eq!(obj.get("!base").unwrap().as_int(), Some(1));
        assert_eq!(obj.get("key").unwrap().as_int(), Some(2));
    }

    // -------------------------------------------------------------------------
    // Nested struct in table
    // -------------------------------------------------------------------------

    #[test]
    fn test_nested_struct_in_table() {
        let input = r#"
            @struct addr (city: string, zip: string)
            @struct person (name: string, home: addr)
            people: @table person [
                (alice, (Boston, "02101")),
                (bob, (NYC, "10001")),
            ]
        "#;
        let tokens = Lexer::new(input).tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let data = parser.parse().unwrap();

        let people = data.get("people").unwrap().as_array().unwrap();
        let alice_home = people[0].as_object().unwrap().get("home").unwrap().as_object().unwrap();
        assert_eq!(alice_home.get("city").unwrap().as_str(), Some("Boston"));
    }

    #[test]
    fn test_include_cycle_detection() {
        // Create a file that includes itself
        let dir = std::env::temp_dir();
        let file_path = dir.join("test_cycle_self.tl");
        std::fs::write(&file_path, "@include \"test_cycle_self.tl\"\nval: 1").unwrap();

        let content = std::fs::read_to_string(&file_path).unwrap();
        let tokens = Lexer::new(&content).tokenize().unwrap();
        let mut parser = Parser::new(tokens).with_base_path(&file_path);
        let result = parser.parse();
        assert!(result.is_err(), "Should detect self-referencing include");
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Circular include"), "Error should mention circular include: {}", err_msg);

        std::fs::remove_file(&file_path).ok();
    }

    #[test]
    fn test_include_mutual_cycle_detection() {
        // Create two files that include each other: A -> B -> A
        let dir = std::env::temp_dir();
        let file_a = dir.join("test_cycle_a.tl");
        let file_b = dir.join("test_cycle_b.tl");
        std::fs::write(&file_a, "@include \"test_cycle_b.tl\"\na_val: 1").unwrap();
        std::fs::write(&file_b, "@include \"test_cycle_a.tl\"\nb_val: 2").unwrap();

        let content = std::fs::read_to_string(&file_a).unwrap();
        let tokens = Lexer::new(&content).tokenize().unwrap();
        let mut parser = Parser::new(tokens).with_base_path(&file_a);
        let result = parser.parse();
        assert!(result.is_err(), "Should detect mutual cycle between A and B");
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Circular include"), "Error should mention circular include: {}", err_msg);

        std::fs::remove_file(&file_a).ok();
        std::fs::remove_file(&file_b).ok();
    }

    #[test]
    fn test_include_stack_propagated_to_child() {
        // Verify that the include_stack starts empty
        let parser = Parser::new(vec![]);
        assert!(parser.include_stack.is_empty(), "New parser should have empty include stack");
    }

    // -------------------------------------------------------------------------
    // Bytes literal parsing
    // -------------------------------------------------------------------------

    #[test]
    fn test_bytes_literal_value() {
        let data = parse(r#"payload: b"cafef00d""#).unwrap();
        let val = data.get("payload").unwrap();
        assert_eq!(val.as_bytes(), Some(&[0xca, 0xfe, 0xf0, 0x0d][..]));
    }

    #[test]
    fn test_bytes_literal_empty_value() {
        let data = parse(r#"empty: b"""#).unwrap();
        let val = data.get("empty").unwrap();
        assert_eq!(val.as_bytes(), Some(&[][..]));
    }

    #[test]
    fn test_bytes_literal_in_array() {
        let data = parse(r#"arr: [b"cafe", b"babe"]"#).unwrap();
        let arr = data.get("arr").unwrap().as_array().unwrap();
        assert_eq!(arr[0].as_bytes(), Some(&[0xca, 0xfe][..]));
        assert_eq!(arr[1].as_bytes(), Some(&[0xba, 0xbe][..]));
    }

    #[test]
    fn test_bytes_literal_in_object() {
        let data = parse(r#"obj: {data: b"ff00"}"#).unwrap();
        let obj = data.get("obj").unwrap().as_object().unwrap();
        assert_eq!(obj.get("data").unwrap().as_bytes(), Some(&[0xff, 0x00][..]));
    }

    // -------------------------------------------------------------------------
    // Fuzz regression tests (full TeaLeaf::parse path)
    // -------------------------------------------------------------------------

    #[test]
    fn test_fuzz_deeply_nested_arrays_no_stack_overflow() {
        // Crafted input with 500 nested arrays — exceeds MAX_PARSE_DEPTH (256)
        let depth = 500;
        let input = format!("key: {}{}", "[".repeat(depth), "]".repeat(depth));
        let result = crate::TeaLeaf::parse(&input);
        match result {
            Err(e) => {
                let err = format!("{}", e);
                assert!(err.contains("nesting depth"), "Error should mention nesting depth: {}", err);
            }
            Ok(_) => panic!("Should fail with depth exceeded, not succeed"),
        }
    }

    #[test]
    fn test_fuzz_deeply_nested_objects_no_stack_overflow() {
        // Crafted input with 500 nested objects
        let depth = 500;
        let mut input = String::from("key: ");
        for i in 0..depth {
            input.push_str(&format!("{{k{}: ", i));
        }
        input.push_str("1");
        for _ in 0..depth {
            input.push('}');
        }
        let result = crate::TeaLeaf::parse(&input);
        assert!(result.is_err(), "Should fail with depth exceeded, not stack overflow");
    }

    #[test]
    fn test_fuzz_deeply_nested_tags_no_stack_overflow() {
        // Crafted input with 500 nested tags: :a :b :c ... value
        let depth = 500;
        let mut input = String::from("key: ");
        for i in 0..depth {
            input.push_str(&format!(":t{} ", i));
        }
        input.push_str("42");
        let result = crate::TeaLeaf::parse(&input);
        assert!(result.is_err(), "Should fail with depth exceeded, not stack overflow");
    }

    #[test]
    fn test_parse_depth_256_succeeds() {
        // 200 levels of nesting should succeed (within MAX_PARSE_DEPTH=256)
        let depth = 200;
        let input = format!("key: {}1{}", "[".repeat(depth), "]".repeat(depth));
        let result = crate::TeaLeaf::parse(&input);
        if let Err(e) = &result {
            panic!("200 levels of nesting should be fine: {}", e);
        }
    }

    #[test]
    fn test_fuzz_crash_e42e_full_parse_no_panic() {
        // Regression: fuzz_parse crash-e42e7ae2f5127519e7e60e87d1cbfbc2a5bf878d
        // Must not panic through TeaLeaf::parse (the actual fuzz path)
        let input = "\"0B\u{10}\u{3}#\"0BP\u{07FE}-----\u{061D}\u{07FE}\u{07FE}-----\u{061D}\u{3}#\"0B\u{10}\u{3}#\"0BP\u{07FE}-----\u{061D}\u{07FE}\u{07FE}-----\u{061D}\u{07FE}";
        let _ = crate::TeaLeaf::parse(input);
    }

    #[test]
    fn test_fuzz_crash_d038_full_parse_no_panic() {
        // Regression: fuzz_parse crash-d0387cbd639a8db9789ab68057f3c58c7bebbfa5
        // Large input with repeated date-like patterns. Must not panic.
        let input = "z\" \"-\"\t; \"\"\")\"\"\"　8]　]　02)3313312)313-333-333-3332)33-133-3-33331333302)33";
        let _ = crate::TeaLeaf::parse(input);
    }

    #[test]
    fn test_reject_value_only_schema_field_types() {
        // Spec §2.1: object, map, tuple, ref, tagged are value types, not schema field types
        for bad_type in &["object", "map", "tuple", "ref", "tagged"] {
            let input = format!("@struct Bad (field: {})\n", bad_type);
            let result = crate::TeaLeaf::parse(&input);
            assert!(result.is_err(), "should reject '{}' as schema field type", bad_type);
            let err = format!("{}", result.err().unwrap());
            assert!(err.contains("value type"), "error for '{}' should mention 'value type': {}", bad_type, err);
        }
        // Array of value-only type should also be rejected
        let result = crate::TeaLeaf::parse("@struct Bad (field: []object)\n");
        assert!(result.is_err(), "should reject '[]object' as schema field type");

        // Valid types should still work
        for good_type in &["string", "int", "int8", "float", "bool", "bytes", "timestamp", "MyStruct"] {
            let input = format!("@struct Good (field: {})\n", good_type);
            assert!(crate::TeaLeaf::parse(&input).is_ok(), "'{}' should be accepted", good_type);
        }
    }
}
