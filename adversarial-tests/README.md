# Adversarial Test Workspace

This folder contains adversarial test inputs and harnesses for the TeaLeaf Rust core, CLI, and .NET bindings.
All tests are isolated here to avoid touching core project files.

Folders:
- inputs/: crafted malformed and edge-case inputs
- core-harness/: Rust harness tests (depends on tealeaf-core via path)
- dotnet-harness/: C# harness using TeaLeaf bindings
- scripts/: runners for CLI/core/.NET adversarial tests
- results/: test logs and outputs
