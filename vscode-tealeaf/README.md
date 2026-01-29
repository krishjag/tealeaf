# TeaLeaf Language Support for Visual Studio Code

Syntax highlighting and language support for the TeaLeaf format (`.tl` files).

## Features

- **Syntax Highlighting** for TeaLeaf text format
  - Directives (`@struct`, `@union`, `@table`, `@map`)
  - Primitive types (`string`, `int`, `float`, `bool`, `bytes`, `timestamp`)
  - Schema definitions and type references
  - Strings with escape sequences
  - Numbers (integers, floats, hex)
  - Constants (`true`, `false`, `~` for null)
  - Comments (`#`)
  - References (`!name`)
  - Tagged values (`:tag`)

- **Language Configuration**
  - Auto-closing brackets and quotes
  - Comment toggling with `Ctrl+/`
  - Bracket matching
  - Indentation rules

## Installation

### From VSIX (Local)

1. Package the extension:
   ```bash
   cd vscode-tealeaf
   npx vsce package
   ```

2. Install the `.vsix` file:
   - Open VS Code
   - Press `Ctrl+Shift+P` → "Extensions: Install from VSIX..."
   - Select the generated `.vsix` file

### For Development

1. Copy or symlink this folder to your VS Code extensions directory:
   - **Windows**: `%USERPROFILE%\.vscode\extensions\tealeaf-lang`
   - **macOS/Linux**: `~/.vscode/extensions/tealeaf-lang`

2. Reload VS Code

## Example

```tealeaf
# Schema definitions
@struct Location (city: string, country: string)
@struct Person (
  id: int,
  name: string,
  email: string?,
  location: Location,
  tags: []string,
)

# Data using schema
people: @table Person [
  (1, "Alice", "alice@example.com", ("Seattle", "USA"), [admin, user])
  (2, "Bob", ~, ("Austin", "USA"), [user])
]

# Simple values
config: {
  debug: true,
  timeout: 5000,
  api_url: "https://api.example.com",
}

# References and tagged values
base: {host: localhost, port: 8080}
dev: !base
status: :ok 200
```

## TeaLeaf Format

TeaLeaf is a schema-aware document format with:
- Human-readable text representation (`.tl`)
- Compact binary representation (`.tlbx`)
- Schema definitions with types
- String deduplication
- JSON interoperability

Learn more: [TeaLeaf Specification](../spec/TEALEAF_SPEC.md)

## License

MIT OR Apache-2.0
