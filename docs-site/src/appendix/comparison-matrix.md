# Comparison Matrix

How TeaLeaf compares to other data formats.

## Feature Comparison

| Feature | JSON | YAML | Protobuf | Avro | MsgPack | CBOR | TeaLeaf |
|---------|------|------|----------|------|---------|------|---------|
| Human-readable text | Yes | Yes | No* | No | No | No | Yes |
| Compact binary | No | No | Yes | Yes | Yes | Yes | Yes |
| Schema in text | No | No | External | External | No | No | Inline |
| Schema in binary | No | No | No | Yes | No | No | Yes |
| No codegen required | Yes | Yes | No | Partial | Yes | Yes | Yes |
| Comments | No | Yes | N/A | N/A | No | No | Yes |
| Built-in JSON conversion | -- | -- | No | No | No | No | Yes |
| String deduplication | No | No | No | No | No | No | Yes |
| Per-section compression | No | No | No | Yes | No | No | Yes |
| Null bitmaps | No | No | No | Yes | No | No | Yes |
| Random-access reading | No | No | No | No | No | No | Yes |

*Protobuf TextFormat exists but is rarely used.

## Size Comparison

| Format | Small Object | 10K Points | 1K Users |
|--------|-------------|------------|----------|
| JSON | 1.00x | 1.00x | 1.00x |
| YAML | ~1.1x | ~1.1x | ~1.1x |
| Protobuf | 0.38x | 0.65x | 0.41x |
| MessagePack | 0.35x | 0.63x | 0.38x |
| CBOR | ~0.40x | ~0.65x | ~0.42x |
| **TeaLeaf Binary** | 3.56x | **0.15x** | 0.47x |

## Speed Comparison

| Operation | JSON (serde) | Protobuf | MsgPack | TeaLeaf |
|-----------|-------------|----------|---------|---------|
| Parse text | Fast | N/A | N/A | Moderate |
| Decode binary | N/A | Fast | Fast | Moderate |
| Encode text | Fast | N/A | N/A | Moderate |
| Encode binary | N/A | Fast | Fast | Moderate |
| Random key access | O(n) parse | O(1) generated | N/A | O(1) hash |

## When to Use Each Format

### Use TeaLeaf When

| Scenario | Why |
|----------|-----|
| LLM context / prompts | Schema-first reduces token count |
| Config files (human-edited + deployed) | Text for editing, binary for deployment |
| Large tabular data | 6-7x compression with string dedup |
| Self-describing data exchange | No external schema files needed |
| Game save data / asset manifests | Compact, nested, self-describing |
| Scientific/sensor data | Null bitmaps for sparse data |

### Use JSON When

| Scenario | Why |
|----------|-----|
| Web APIs / REST | Universal support |
| Small payloads (< 1 KB) | No overhead |
| JavaScript-heavy applications | Native parsing |
| Human-only data (no binary needed) | Simpler tooling |

### Use Protobuf When

| Scenario | Why |
|----------|-----|
| RPC / gRPC services | First-class streaming support |
| Maximum decode speed | Generated code with known offsets |
| Schema evolution at scale | Field numbers + backward compat |
| Microservice communication | Established ecosystem |

### Use Avro When

| Scenario | Why |
|----------|-----|
| Hadoop / big data pipelines | Ecosystem integration |
| Schema registry workflows | Built-in evolution |
| Large-scale data lake storage | Block compression |

### Use MessagePack / CBOR When

| Scenario | Why |
|----------|-----|
| Tiny payloads (< 100 bytes) | Minimal overhead |
| Schemaless binary | No schema definition needed |
| Drop-in JSON replacement | Similar data model |

## Ecosystem Maturity

| Aspect | JSON | Protobuf | Avro | TeaLeaf |
|--------|------|----------|------|---------|
| Language support | Universal | 10+ languages | 5+ languages | Rust, .NET |
| Tooling | Extensive | Extensive | Moderate | CLI + libraries |
| Community | Massive | Large | Medium | Early |
| Specification maturity | RFC 8259 | Stable (proto3) | Apache spec | Beta |
| IDE support | Universal | Plugins | Plugins | Planned |

TeaLeaf is a young format (v2.0.0-beta.10). It fills a specific niche that existing formats don't serve well -- but it doesn't aim to replace established formats in their core use cases.
