# Architecture Decision Records

This section documents significant architecture decisions made in the TeaLeaf project. Each record captures the context, decision, and consequences of a choice that affects the project's design or implementation.

## ADR Index

| ADR | Title | Status | Date |
|-----|-------|--------|------|
| [ADR-0001](./adr-0001-indexmap-insertion-order.md) | Use IndexMap for Insertion Order Preservation | Accepted | 2026-02-05 |
| [ADR-0002](./adr-0002-fuzzing-architecture.md) | Fuzzing Architecture and Strategy | Accepted | 2026-02-06 |
| [ADR-0003](./adr-0003-nesting-depth-limit.md) | Maximum Nesting Depth Limit (256) | Accepted | 2026-02-06 |
| [ADR-0004](./adr-0004-zlib-compression.md) | ZLIB Compression for Binary Format | Accepted | 2026-02-06 |
| [ADR-0005](./adr-0005-direct-value-representation.md) | Direct Value Representation Without Intermediate AST | Accepted | 2026-02-14 |

## What is an ADR?

An Architecture Decision Record (ADR) is a short document that captures an important architectural decision along with its context and consequences. ADRs help future contributors understand **why** certain design choices were made, not just **what** was built.

## ADR Lifecycle

Each ADR has one of the following statuses:

- **Proposed** — Under discussion, not yet implemented
- **Accepted** — Approved and implemented (or in progress)
- **Superseded** — Replaced by a newer ADR (linked in the record)
- **Deprecated** — No longer applicable due to project changes
