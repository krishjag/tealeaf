# ADR-0002: Fuzzing Architecture and Strategy

- **Status:** Accepted
- **Date:** 2026-02-06
- **Applies to:** tealeaf-core

## Context

TeaLeaf is a data format with multiple serialization paths: text parsing, text serialization, binary compilation, binary reading, and JSON import/export. Each path accepts untrusted input in production scenarios (user-supplied `.tl` files, `.tlbx` binaries, JSON strings from external APIs). Malformed or adversarial input must never cause undefined behavior, panics in non-roundtrip code paths, or memory safety violations.

The project already had unit tests, canonical fixture tests, and adversarial tests (hand-crafted malformed inputs). However, these approaches have inherent limitations:

1. **Unit/fixture tests are author-biased.** They test cases the developer thought of, missing emergent edge cases from format interactions (e.g., deeply nested structures with unicode escapes inside hex-prefixed numbers).

2. **Adversarial tests are finite.** The hand-crafted corpus in `adversarial-tests/` covers known attack patterns but cannot explore the combinatorial input space.

3. **Round-trip fidelity is hard to test exhaustively.** The property "serialize then parse produces the same value" requires testing across all `Value` variants, nesting depths, and string content — a space too large for manual enumeration.

### Alternatives Considered

| Approach | Pros | Cons |
|----------|------|------|
| **Property-based testing (proptest/quickcheck)** | Integrated into `cargo test`, structure-aware | Limited mutation depth, no coverage feedback, deterministic |
| **AFL++** | Mature, multiple mutation strategies | Requires instrumentation harness, harder CI integration on GitHub Actions |
| **cargo-fuzz (libFuzzer)** | Native Rust support, coverage-guided, dictionary support, easy CI | Requires nightly toolchain, Linux-only |
| **Honggfuzz** | Hardware-assisted coverage | Less Rust ecosystem integration, complex setup |

## Decision

Use **cargo-fuzz (libFuzzer)** with a three-layer fuzzing strategy:

### Layer 1: Byte-level fuzzing (6 targets)

Coverage-guided mutation of raw bytes, testing each attack surface independently:

| Target | Input | Tests |
|--------|-------|-------|
| `fuzz_parse` | Raw bytes as TL text | Parser robustness against arbitrary byte sequences |
| `fuzz_serialize` | Raw bytes as TL text | Parse then re-serialize roundtrip fidelity |
| `fuzz_roundtrip` | Raw bytes as TL text | Full text → parse → serialize → re-parse → value equality |
| `fuzz_reader` | Raw bytes as `.tlbx` binary | Binary reader robustness against malformed files |
| `fuzz_json` | Raw bytes as JSON string | JSON import → TL export → re-import roundtrip |
| `fuzz_json_schemas` | Raw bytes as JSON string | JSON import with schema inference → roundtrip |

### Layer 2: Dictionary-guided fuzzing

libFuzzer dictionaries provide grammar-aware tokens that seed the mutation engine, dramatically improving coverage for structured formats where random bytes rarely produce valid syntax:

| Dictionary | Used by | Key tokens |
|------------|---------|------------|
| `tl.dict` | `fuzz_parse`, `fuzz_serialize`, `fuzz_roundtrip` | Keywords (`true`, `false`, `null`, `NaN`, `inf`), directives (`@struct`, `@table`, `@union`), type names, escape sequences, boundary numbers, timestamp patterns |
| `json.dict` | `fuzz_json`, `fuzz_json_schemas` | JSON delimiters, escape sequences, surrogate pair markers, serde_json magic strings, boundary numbers |

Measured coverage impact (30-second fresh corpus):

| Target | Without dict | With dict | Improvement |
|--------|-------------|-----------|-------------|
| `fuzz_parse` | 1790 edges | 1922 edges | **+7.4%** |
| `fuzz_json` | 1339 edges | 1533 edges | **+14.5%** |

### Layer 3: Structure-aware fuzzing (1 target)

The `fuzz_structured` target bypasses the parser entirely, generating valid `Value` trees directly from fuzzer bytes using the `arbitrary` crate. This tests serialization and binary compilation paths with guaranteed-valid inputs that would take byte-level fuzzers much longer to discover:

- **Bounded recursion** (max depth 3) prevents stack overflow
- **13 Value variants** including `JsonNumber`, `Tagged`, `Ref`, `Map`, `Bytes`
- **Three roundtrip tests per invocation**: text serialize/parse, binary compile/read, JSON no-panic
- Reaches 2464 coverage edges in just 733 runs (vs thousands of runs for byte-level targets)

### Fuzz infrastructure layout

```
tealeaf-core/fuzz/
  Cargo.toml              # Fuzz workspace with libfuzzer-sys + arbitrary
  fuzz_targets/
    fuzz_parse.rs         # Layer 1: text parser robustness
    fuzz_serialize.rs     # Layer 1: text roundtrip
    fuzz_roundtrip.rs     # Layer 1: full text roundtrip with value equality
    fuzz_reader.rs        # Layer 1: binary reader robustness
    fuzz_json.rs          # Layer 1: JSON import roundtrip
    fuzz_json_schemas.rs  # Layer 1: JSON with schema inference roundtrip
    fuzz_structured.rs    # Layer 3: structure-aware value generation
  dictionaries/
    tl.dict               # Layer 2: TL text format tokens
    json.dict             # Layer 2: JSON format tokens
  corpus/                 # Persistent corpus (per-target subdirectories)
  artifacts/              # Crash artifacts (per-target subdirectories)
```

### CI integration

Fuzz targets run on GitHub Actions `ubuntu-latest` (2-core, 7 GB RAM) with the following constraints:

- **120 seconds per target** (coverage saturates within ~30 seconds; 120s provides buffer for deeper exploration)
- **Serial execution** — targets run one at a time to avoid memory pressure (each can use up to 512 MB RSS)
- **RSS limit: 512 MB** per target
- Dictionary-guided runs for text and JSON targets
- Nightly Rust toolchain required (libFuzzer instrumentation)
- **Total wall time: ~15 minutes** (7 targets × 120s + build overhead)

### Value equality semantics

All roundtrip targets use a custom `values_equal()` function rather than `PartialEq` to handle expected coercions:

- `Int(n)` == `UInt(n)` when `n >= 0` (sign-agnostic integer comparison)
- `JsonNumber(s)` == `Int(i)` when `s` parses to `i` (precision-preserving numbers may roundtrip as integers if they fit)
- `Float` comparison uses `to_bits()` for exact bit-level equality (distinguishes `+0.0` from `-0.0`, handles NaN)

## Consequences

### Positive

- **Discovered real bugs.** Fuzzing found a NaN quoting bug (`NaN` roundtripped as `Float(NaN)` instead of being preserved through text format) and the precision loss that motivated `Value::JsonNumber`.
- **Continuous regression detection.** CI runs catch regressions in parser/serializer correctness automatically on every push.
- **Coverage-guided exploration.** libFuzzer's coverage feedback explores code paths that hand-written tests miss, particularly in error handling and edge case branches.
- **Dictionary tokens accelerate exploration.** Measured 7-14% coverage improvement with dictionaries, at zero runtime cost (dictionaries only seed the mutation engine).
- **Structure-aware fuzzing tests serializer independently.** By generating valid `Value` trees directly, `fuzz_structured` achieves deep serializer coverage without depending on parser correctness.

### Negative

- **Nightly Rust toolchain required.** `cargo-fuzz` requires nightly for `-Z` flags and sanitizer instrumentation. This is isolated to the fuzz workspace and does not affect the main build.
- **Linux-only.** libFuzzer doesn't support Windows natively. Local fuzzing requires WSL on Windows; CI uses Ubuntu runners.
- **CI time cost.** ~15 minutes per run. Acceptable for a post-push check; not suitable for pre-commit.
- **Corpus growth.** The persistent corpus grows over time as new coverage-increasing inputs are discovered. Periodic corpus minimization (`cargo fuzz cmin`) is recommended.

### Not covered

- **Protocol-level fuzzing.** The FFI boundary (`tealeaf-ffi`) is not fuzzed directly. FFI functions are thin wrappers around the core library, which is fuzzed.
- **.NET binding fuzzing.** The .NET layer is tested through its own test suite and the adversarial harness, but not through libFuzzer.
- **Concurrency testing.** All fuzz targets are single-threaded. Thread-safety of `Reader` (which uses `mmap`) is tested separately.

## References

- [cargo-fuzz documentation](https://rust-fuzz.github.io/book/cargo-fuzz.html)
- [libFuzzer documentation](https://llvm.org/docs/LibFuzzer.html)
- [libFuzzer dictionary format](https://llvm.org/docs/LibFuzzer.html#dictionaries)
- [arbitrary crate](https://docs.rs/arbitrary/latest/arbitrary/) for structure-aware fuzzing
