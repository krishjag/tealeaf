# TeaLeaf Data Format

**A schema-aware document format with human-readable text and compact binary representation.**

<span class="version-badge">v2.0.0-beta.1</span>

---

## What is TeaLeaf?

TeaLeaf is a data format that bridges the gap between human-readable configuration and machine-efficient binary storage. A single `.tl` source file can be read and edited by humans, compiled to a compact `.tlbx` binary, and converted to/from JSON -- all with schemas inline.

**TeaLeaf** -- schemas with nested structures, compact positional data:

```tl
# Schema: define structure once
@struct Location (city, country)
@struct Department (name, location: Location)
@struct Employee (
  id: int,
  name,
  role,
  department: Department,
  skills: []string,
)

# Data: field names not repeated
employees: @table Employee [
  (1, "Alice", "Engineer",
    ("Platform", ("Seattle", "USA")),
    ["rust", "python"])
  (2, "Bob", "Designer",
    ("Product", ("Austin", "USA")),
    ["figma", "css"])
  (3, "Carol", "Manager",
    ("Platform", ("Seattle", "USA")),
    ["leadership", "agile"])
]
```

**JSON** -- no schema, names repeated:

```json
{
  "employees": [
    {
      "id": 1,
      "name": "Alice",
      "role": "Engineer",
      "department": {
        "name": "Platform",
        "location": { "city": "Seattle", "country": "USA" }
      },
      "skills": ["rust", "python"]
    },
    {
      "id": 2,
      "name": "Bob",
      "role": "Designer",
      "department": {
        "name": "Product",
        "location": { "city": "Austin", "country": "USA" }
      },
      "skills": ["figma", "css"]
    },
    {
      "id": 3,
      "name": "Carol",
      "role": "Manager",
      "department": {
        "name": "Platform",
        "location": { "city": "Seattle", "country": "USA" }
      },
      "skills": ["leadership", "agile"]
    }
  ]
}
```

## Key Features

| Feature | Description |
|---------|-------------|
| **Dual format** | Human-readable text (`.tl`) and compact binary (`.tlbx`) |
| **Inline schemas** | `@struct` definitions live alongside data -- no external `.proto` files |
| **JSON interop** | Bidirectional conversion with automatic schema inference |
| **String deduplication** | Binary format stores each unique string once |
| **Compression** | Per-section ZLIB compression with null bitmaps |
| **Comments** | `#` line comments in the text format |
| **Language bindings** | Native Rust, .NET (via FFI + source generator) |
| **CLI tooling** | `tealeaf compile`, `decompile`, `validate`, `info`, JSON conversion |

## Why TeaLeaf?

The existing data format landscape presents trade-offs that don't always align well with modern workflows:

| Format | Limitation |
|--------|------------|
| JSON | Verbose, no comments, no schema |
| YAML | Indentation-sensitive, error-prone at scale |
| Protobuf | Schema external, binary-only, requires codegen |
| Avro | Schema embedded but not human-readable |
| CSV/TSV | Too simple for nested or typed data |
| MessagePack/CBOR | Compact but schemaless |

TeaLeaf unifies these concerns:

- **Human-readable** text format with explicit types and comments
- **Compact binary** with embedded schemas -- no external schema files needed
- **Schema-first** design -- field names defined once, not repeated per record
- **No codegen required** -- schemas discovered at runtime
- **Built-in JSON conversion** for easy integration with existing tools

## Primary Use Case: Context Engineering

TeaLeaf was designed for **context engineering** in LLM applications -- structured prompts, tool definitions, conversation history -- where schema-first layout reduces token count while keeping data human-readable for authoring.

```tl
@struct Message (role: string, content: string, tokens: int?)
@struct Tool (name: string, description: string, params: []string)

system_prompt: "You are a helpful assistant."

tools: @table Tool [
  ("search", "Search the web", ["query"]),
]

history: @table Message [
  ("user", "Hello", 2),
  ("assistant", "Hi there!", 3),
]
```

**50 messages + 10 tools:** JSON ~15KB vs TeaLeaf Binary ~4KB

## Size Comparison

| Format | Small Object | 10K Points | 1K Users |
|--------|-------------|------------|----------|
| JSON | 1.00x | 1.00x | 1.00x |
| Protobuf | 0.38x | 0.65x | 0.41x |
| MessagePack | 0.35x | 0.63x | 0.38x |
| **TeaLeaf Compressed** | 3.56x | **0.15x** | 0.47x |

TeaLeaf has 64-byte header overhead (not ideal for tiny objects). For large arrays with compression, TeaLeaf achieves **6-7x better compression** than JSON.

> **Trade-off:** TeaLeaf decode is ~2-5x slower than Protobuf due to dynamic key-based access. Choose TeaLeaf when size matters more than decode speed.

## Project Structure

```
tealeaf/
├── tealeaf-core/       # Rust core: parser, compiler, reader, CLI
├── tealeaf-derive/     # Rust proc-macro: #[derive(ToTeaLeaf, FromTeaLeaf)]
├── tealeaf-ffi/        # C-compatible FFI layer
├── bindings/
│   └── dotnet/         # .NET bindings + source generator
├── canonical/          # Canonical test fixtures
├── spec/               # Format specification
└── examples/           # Example files and workflows
```

## Quick Links

- **Getting Started:** [Installation](./getting-started/installation.md) | [Quick Start](./getting-started/quick-start.md) | [Concepts](./getting-started/concepts.md)
- **Format:** [Text Format](./format/text-format.md) | [Type System](./format/type-system.md) | [Binary Format](./format/binary-format.md)
- **CLI:** [Command Reference](./cli/overview.md)
- **Rust:** [Overview](./rust/overview.md) | [Derive Macros](./rust/derive-macros.md)
- **.NET:** [Overview](./dotnet/overview.md) | [Source Generator](./dotnet/source-generator.md)
- **FFI:** [API Reference](./ffi/api-reference.md)
- **Guides:** [LLM Context](./guides/llm-context.md) | [Performance](./guides/performance.md)

## License

TeaLeaf is dual-licensed under [MIT](https://opensource.org/licenses/MIT) or [Apache-2.0](https://www.apache.org/licenses/LICENSE-2.0), at your option.

Source code: [github.com/krishjag/tealeaf](https://github.com/krishjag/tealeaf)
