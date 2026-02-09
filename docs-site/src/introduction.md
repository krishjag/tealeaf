# TeaLeaf Data Format

**A schema-aware data format with human-readable text and compact binary representation.**

**~36% fewer data tokens than JSON for LLM applications, with zero accuracy loss.**

<span class="version-badge">v2.0.0-beta.4</span>

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

The existing data format landscape presents trade-offs that TeaLeaf attempts to bridge. TeaLeaf does not attempt to replace any of the formats listed below, but rather presents a different perspective that users can objectively compare to identify if it fits their specific use cases.

| Format | Observation |
|--------|-------------|
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

## Primary Use Case: LLM API Data Payloads

TeaLeaf is well-suited for assembling and managing context for large language models -- sending business data, analytics, and structured payloads to LLM APIs where token efficiency directly impacts API costs.

**Why TeaLeaf for LLM context:**
- **~36% fewer data tokens** — verified across Claude Sonnet 4.5 and GPT-5.2 (12 tasks, 10 domains; savings increase with larger datasets)
- **Zero accuracy loss** — [benchmark scores](https://github.com/krishjag/tealeaf/tree/main/accuracy-benchmark) within noise (0.988 vs 0.978 Anthropic, 0.901 vs 0.899 OpenAI)
- Binary format for fast cached context retrieval
- String deduplication (roles, field names, common values stored once)
- Human-readable text for prompt authoring

**Token savings example (retail orders dataset):**

| Format | Characters | Tokens (GPT-5.x) | Savings |
|--------|-----------|-------------------|---------|
| JSON | 36,791 | 9,829 | — |
| TeaLeaf | 14,542 | 5,632 | **43% fewer tokens** |

## Size Comparison

| Format | Small Object | 10K Points | 1K Users |
|--------|-------------|------------|----------|
| JSON | 1.00x | 1.00x | 1.00x |
| Protobuf | 0.38x | 0.65x | 0.41x |
| MessagePack | 0.35x | 0.63x | 0.38x |
| TeaLeaf Text | 1.38x | 0.87x | 0.63x |
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

TeaLeaf is licensed under the [MIT License](https://opensource.org/licenses/MIT).

Source code: [github.com/krishjag/tealeaf](https://github.com/krishjag/tealeaf)
