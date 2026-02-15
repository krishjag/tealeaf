# ADR-0005: Direct Value Representation Without Intermediate AST

- **Status:** Accepted
- **Date:** 2026-02-14
- **Applies to:** tealeaf-core (parser, writer, reader)

## Context

Compilers and language toolchains typically separate parsing into two phases: a **parser** that produces an Abstract Syntax Tree (AST) capturing the syntactic structure, and a **lowering pass** that transforms the AST into an Intermediate Representation (IR) suitable for analysis, optimization, or code generation. This separation is valuable when the language has control flow, expressions, type inference, or optimization passes that benefit from a distinct representation.

TeaLeaf is a **data serialization format**, not a programming language. Its text format (`.tl`) describes structured data — scalars, arrays, objects, maps, references, tags, and timestamps — with inline schema definitions (`@struct`, `@union`, `@table`). There are no expressions to evaluate, no control flow to analyze, no types to infer beyond what the schema declares, and no transformations to optimize.

The question is whether the parser should produce an intermediate syntax tree that is then lowered to the final `Value` representation, or whether it should construct `Value` nodes directly.

### Alternatives Considered

| Approach | Pros | Cons |
|----------|------|------|
| **AST → IR → Value** (traditional compiler) | Clear phase separation, easier to add language features later | Two redundant tree representations, allocation overhead, conversion code to maintain, no transformations to perform |
| **AST → Value** (syntax tree then lower) | Phase separation for error reporting | AST nodes mirror `Value` variants 1:1, extra allocation pass, conversion is mechanical copy |
| **Direct Value construction** (chosen) | Zero overhead, parser output is the final representation, no redundant allocations, simpler codebase | Parser must handle schema-directed disambiguation inline, error locations require token tracking |

## Decision

The parser constructs `Value` enum variants directly during parsing. There is no separate AST or IR layer.

The `Value` enum (`types.rs:354`) serves as both the parser output and the document model consumed by all downstream operations:

```rust
pub enum Value {
    Null, Bool(bool), Int(i64), UInt(u64), Float(f64),
    String(String), Bytes(Vec<u8>),
    Array(Vec<Value>),
    Object(ObjectMap<String, Value>),
    Map(Vec<(Value, Value)>),
    Ref(String),
    Tagged(String, Box<Value>),
    Timestamp(i64, i16),
    JsonNumber(String),
}
```

The parser (`parser.rs:333`) recurses through tokens and returns `Value` nodes at each level:

```rust
fn parse_value(&mut self, depth: usize) -> Result<Value> {
    // Directly constructs Value::Object, Value::Array, etc.
}
```

Schema definitions (`@struct`, `@union`) are accumulated into side tables (`parser.rs:16–17`) during parsing and packaged alongside the `Value` tree in the `TeaLeaf` document container (`lib.rs:44`):

```rust
pub struct TeaLeaf {
    pub schemas: IndexMap<String, Schema>,
    pub unions: IndexMap<String, Union>,
    pub data: IndexMap<String, Value>,
}
```

The binary writer (`writer.rs:305`) and reader (`reader.rs:820`) consume and produce the same `Value` type, completing a single-representation pipeline:

```
Text → Lexer → Parser → Value tree → Writer → .tlbx
                                   ↘ to_json() → JSON
.tlbx → Reader → Value tree → to_tl_with_schemas() → Text
                            ↘ to_json() → JSON
```

### Schema-Directed Parsing

The one area where a separate AST might have simplified the design is `@table` parsing. When the parser encounters a `@table` directive, it uses the referenced schema's field types to disambiguate positional tuple values — for example, knowing that a field is a nested `@struct` type determines whether parentheses represent a tuple (struct fields) or a grouped expression. This schema-directed parsing (`parser.rs:490`, `parse_value_for_field`) is handled inline rather than as a post-parse lowering step. The trade-off is acceptable because:

1. Schema definitions must appear before `@table` usage (top-to-bottom processing), so the schema is always available when needed
2. The disambiguation logic is localized to one function (`parse_value_for_field`)
3. The alternative — parsing ambiguous tuples into AST nodes and resolving later — would require the same schema lookup but with an extra allocation and traversal pass

## Consequences

### Positive

- **Zero allocation overhead.** Every node allocated during parsing is a node in the final document. No intermediate trees are built and discarded.

- **Single type to learn.** Contributors, binding authors (FFI, .NET), and derive macro implementations all work with one `Value` enum. There is no `AstNode` ↔ `Value` mapping to understand or maintain.

- **Simpler binary round-trip.** The writer's `encode_value` and the reader's `decode_value` operate on the same `Value` type the parser produces. Round-trip fidelity (text → binary → text) is straightforward because there is no lossy conversion between representations.

- **Smaller codebase.** No AST type definitions, no lowering pass, no conversion tests. The parser module is ~700 lines including all schema-directed logic.

### Negative

- **Error locations are token-based, not AST-based.** Parse errors report line and column from the token stream (`Token.line`, `Token.col`). A separate AST could carry richer span information (start/end positions, source ranges). In practice, the token position is sufficient for the error messages TeaLeaf produces.

- **Adding language features would require revisiting.** If TeaLeaf were to gain computed expressions, conditional includes, or macro expansion, a separate AST phase would become necessary for pre-evaluation transformations. This is explicitly out of scope — TeaLeaf is a data format, and features like `@include` are resolved during parsing by recursively invoking the parser on the included file.

- **Schema-directed parsing couples the parser to schema semantics.** The parser must understand `FieldType` to correctly parse `@table` rows. In an AST approach, the parser could produce generic tuple nodes and a later pass could resolve them. The coupling is acceptable because schema definitions are a core language feature, not an extension.

### Why Not Add It Later?

A common argument for AST layers is "we might need it later." For TeaLeaf, this is unlikely because:

1. **Data formats don't evolve into languages.** JSON, YAML, TOML, and Protocol Buffers all use direct value construction. None has needed an AST layer.
2. **The `Value` enum is extensible.** New data types (e.g., `JsonNumber` was added post-initial-design) are added as new enum variants without structural changes.
3. **If needed, it can be introduced non-destructively.** An AST layer would sit between the lexer and the current parser, and the current `parse_value` logic would become the lowering pass. No downstream code (writer, reader, bindings) would change.

## References

- `tealeaf-core/src/types.rs:354` — `Value` enum definition
- `tealeaf-core/src/parser.rs:333` — `parse_value()` direct construction
- `tealeaf-core/src/parser.rs:490` — `parse_value_for_field()` schema-directed parsing
- `tealeaf-core/src/writer.rs:305` — `encode_value()` consumes `Value` directly
- `tealeaf-core/src/reader.rs:820` — `decode_value()` produces `Value` directly
- `tealeaf-core/src/lib.rs:44` — `TeaLeaf` document container
