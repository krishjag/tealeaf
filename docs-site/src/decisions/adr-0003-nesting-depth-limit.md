# ADR-0003: Maximum Nesting Depth Limit (256)

- **Status:** Accepted
- **Date:** 2026-02-06
- **Applies to:** tealeaf-core (parser, binary reader)

## Context

TeaLeaf accepts untrusted input in production — user-supplied `.tl` files, `.tlbx` binaries from external sources, and JSON strings from APIs. Recursive data structures (arrays, objects, maps, tagged values) create call stacks proportional to input nesting depth. Without a limit, an attacker can craft a payload like `key: [[[[...` with thousands of levels, causing a stack overflow and process termination.

Two constants enforce the limit:

| Constant | File | Value |
|----------|------|-------|
| `MAX_PARSE_DEPTH` | `parser.rs` | 256 |
| `MAX_DECODE_DEPTH` | `reader.rs` | 256 |

Both constants are set to the same value to ensure text-binary parity: any document that parses successfully from `.tl` text can also round-trip through `.tlbx` binary without hitting a different depth ceiling.

The limit is checked at every recursive entry point:

- **Parser:** `parse_value()` — arrays, objects, maps, tuples, tagged values
- **Reader:** `decode_value()`, `decode_array()`, `decode_object()`, `decode_struct()`, `decode_struct_array()`, `decode_map()`

When exceeded, both paths return a descriptive error rather than panicking or overflowing the stack.

## Ecosystem Comparison

| Parser / Library | Default Max Depth | Configurable? |
|------------------|-------------------|---------------|
| **TeaLeaf** | **256** | **No (compile-time constant)** |
| serde_json (Rust) | 128 | Yes (`disable_recursion_limit`) |
| serde_yaml (Rust) | 128 | No |
| System.Text.Json (.NET) | 64 | Yes (`MaxDepth`) |
| ASP.NET Core (default) | 32 | Yes |
| Jackson (Java) | 1000 (v2), 500 (v3) | Yes |
| Go encoding/json | 10,000 | No |
| Python json (stdlib) | ~1,000 (interpreter limit) | Via `sys.setrecursionlimit` |
| Protocol Buffers (Java/C++) | 100 | Yes |
| Protocol Buffers (Go) | 10,000 | Yes |
| rmp-serde (MessagePack) | 1,024 | Yes |
| CBOR (ciborium, Rust) | 128 | Yes |
| toml (Rust) | None | No (vulnerable to stack overflow) |

### Observations

- **Conservative defaults are trending down.** Jackson reduced from 1,000 to 500 in v3. .NET defaults to 64. Protocol Buffers targets 100.
- **128 is the most common Rust ecosystem default** (serde_json, serde_yaml, ciborium).
- **No production data format needs > 100 levels.** Deeply nested structures indicate either machine-generated intermediate representations or adversarial input.
- **Formats without limits have CVEs.** The toml crate's lack of depth limiting is tracked as an open issue. Python's reliance on interpreter limits has caused production crashes.

## Decision

Set `MAX_PARSE_DEPTH` and `MAX_DECODE_DEPTH` to **256**.

### Why 256 over 128?

TeaLeaf schemas add implicit nesting. A `@struct` with an array of `@struct`-typed objects creates 3 levels of nesting (object → array → object) for what the user perceives as one level of structure. With schema compositions, 128 could be reached in complex but legitimate documents. 256 provides a 2x margin above the Rust ecosystem default while remaining well within safe stack bounds.

### Why not configurable?

- **Simplicity.** A compile-time constant is zero-cost at runtime (no configuration plumbing, no state to manage).
- **Consistent behavior.** All TeaLeaf implementations (Rust, FFI, .NET) enforce the same limit. A configurable limit would require coordination across language boundaries.
- **256 is generous enough.** No known use case requires deeper nesting. If a legitimate need arises, the constant can be bumped in a patch release without breaking any public API.

### Stack safety margin

On x86-64 Linux with the default 8 MB stack, each recursive call uses roughly 200–400 bytes of stack frame. At 256 depth, the worst case is ~100 KB — well under 2% of the available stack. This leaves ample room for the caller's own stack frames and for platforms with smaller stacks (e.g., 1 MB thread stacks).

## Test Coverage

| Test | Location | What it verifies |
|------|----------|------------------|
| `test_parse_depth_256_succeeds` | `parser.rs` | 200-level nesting parses successfully |
| `test_fuzz_deeply_nested_arrays_no_stack_overflow` | `parser.rs` | 500-level nesting returns error (no crash) |
| `parse_deep_nesting_ok` | `adversarial.rs` | 7-level nesting succeeds in adversarial harness |
| `fuzz_structured` depth=3 | `fuzz_structured.rs` | Structure-aware fuzzer bounds depth to 3 |
| `canonical/large_data.tl` | Canonical suite | Deep nesting fixture round-trips correctly |

## Consequences

### Positive

- **Stack overflow protection.** Malicious or malformed input with extreme nesting is rejected with a clear error message instead of crashing the process.
- **Text-binary parity.** The same limit in parser and reader means any document that parses from text will also decode from binary, and vice versa.
- **Predictable resource usage.** Callers can reason about maximum stack consumption without inspecting input.

### Negative

- **Theoretical limitation.** Documents with more than 256 levels of nesting are rejected. In practice, no known data format use case requires this depth.
- **Not configurable.** Users who need deeper nesting must rebuild from source with a modified constant. This is an intentional trade-off for simplicity.

### Neutral

- **No performance cost.** The depth check is a single integer comparison per recursive call — unmeasurable relative to the cost of decoding a value.
