# Change Log

All notable changes to the "PAX Language Support" extension will be documented in this file.

## [0.1.0] - 2024-01-25

### Added
- Initial release
- Syntax highlighting for PAX format
  - Directives: `@struct`, `@union`, `@table`, `@map`, `@include`
  - Primitive types: `string`, `int`, `uint`, `float`, `bool`, `bytes`, `timestamp`
  - Sized types: `int8`, `int16`, `int32`, `int64`, `uint8`, etc.
  - Array types: `[]type`
  - Nullable types: `type?`
  - Schema and type definitions
  - String literals with escape sequences
  - Numeric literals (decimal, hex, float, scientific)
  - Boolean literals: `true`, `false`
  - Null literal: `~`
  - Comments: `#`
  - References: `!name`
  - Tagged values: `:tag`
- Language configuration
  - Comment toggling
  - Bracket matching and auto-closing
  - Indentation rules

## [Unreleased]

### Planned
- Semantic highlighting
- Error diagnostics via Language Server
- Auto-completion for types and fields
- Hover information
- Go to definition
- Format document
