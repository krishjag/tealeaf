# ADR-0004: ZLIB Compression for Binary Format

- **Status:** Accepted
- **Date:** 2026-02-06
- **Applies to:** tealeaf-core (writer, reader), spec §4.3 and §4.9

## Context

The `.tlbx` binary format compresses individual sections to reduce file size. The implementation has always used **ZLIB (deflate)** via the `flate2` crate. However, the spec contained a contradiction:

- **§4.3 (Header Flags)** described the COMPRESS flag as indicating "zstd compression" and required readers to detect compression via the zstd frame magic (`0xFD2FB528`).
- **§4.9 (Compression)** correctly stated the algorithm as "ZLIB (deflate)".

This contradiction meant a third-party implementation following §4.3 would look for zstd-compressed data that doesn't exist, while one following §4.9 would work correctly. The spec needed a single, definitive answer.

## Decision

Standardize on **ZLIB (deflate)** as the sole compression algorithm for `.tlbx` binary format v2.

### Why not zstd?

zstd is a superior algorithm in general-purpose benchmarks, but TeaLeaf's design neutralizes its advantages:

1. **String deduplication removes the most compressible data.** The string table deduplicates all strings before compression runs. What remains for the compressor is packed integers, null bitmaps, and string table indices — low-entropy binary data with little redundancy.

2. **Sections are small.** The compression threshold is 64 bytes. Most sections are a few hundred bytes to a few KB. At these sizes, zlib and zstd achieve nearly identical compression ratios without dictionaries.

3. **zstd's dictionary mode doesn't help here.** Dictionary compression — where zstd's largest advantage lies for small payloads — requires pre-training on representative data. TeaLeaf documents are schema-variable and content-diverse (the primary use case is LLM context engineering with arbitrary structured data). A static dictionary would not generalize across different schemas and data shapes.

4. **The 90% threshold filters aggressively.** Sections that don't compress to under 90% of their original size are stored uncompressed. This threshold means most small sections aren't compressed at all, making the algorithm choice irrelevant for the majority of sections.

5. **Decompression speed is irrelevant at this scale.** zstd decompresses 3-5x faster than zlib, but a few-hundred-byte section decompresses in microseconds with either algorithm. The difference is unmeasurable in practice.

### Why zlib?

1. **Universal availability.** ZLIB/deflate is implemented in every language's standard library or a widely-available package. zstd requires an additional native dependency in most ecosystems.

2. **No breaking change.** Every `.tlbx` file ever produced uses zlib. Switching would require either a format version bump (breaking all existing files) or dual-algorithm detection logic (complexity for every implementation).

3. **Simpler for third-party implementations.** One algorithm, no magic-byte detection, no conditional dependency. A conformant reader needs only zlib decompression.

4. **Compression is not the primary size reduction strategy.** TeaLeaf's token efficiency comes from the text format's conciseness and the binary format's schema-aware encoding (struct arrays, string deduplication, type-specific packing). Compression is a secondary optimization applied on top.

## Spec Changes

| Section | Before | After |
|---------|--------|-------|
| §4.3 (Header Flags) | "zstd compression", "zstd frame magic" | "ZLIB (deflate) compression", per-section flag detection |
| §4.9 (Compression) | Already correct ("ZLIB (deflate)") | No change |

## Consequences

### Positive

- **Spec is internally consistent.** §4.3 and §4.9 now agree on ZLIB.
- **Third-party interop is unambiguous.** Implementers need one algorithm, clearly documented.
- **No migration required.** All existing `.tlbx` files remain valid.

### Negative

- **Foregoes zstd's speed advantage.** In workloads with large sections (tens of KB+), zstd would decompress faster. TeaLeaf's current section sizes don't reach this threshold.

### Neutral

- **Future versions can reconsider.** If TeaLeaf v3 introduces large-section use cases (e.g., embedded binary blobs), zstd could be adopted with a format version bump. This ADR applies to binary format v2 only.
