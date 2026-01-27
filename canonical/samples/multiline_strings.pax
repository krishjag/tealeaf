# Canonical test: Multiline strings (triple-quoted)
# Tests: auto-dedenting, preserved newlines, in various contexts

# Basic multiline string
description: """
    This is a multiline string.
    Leading whitespace is trimmed.
    Useful for documentation.
"""

# Code block
code_sample: """
    fn main() {
        println!("Hello, world!");
    }
"""

# SQL query
sql_query: """
    SELECT id, name, email
    FROM users
    WHERE active = true
    ORDER BY created_at DESC
    LIMIT 100;
"""

# JSON template
json_template: """
    {
        "name": "{{name}}",
        "version": "{{version}}",
        "dependencies": {}
    }
"""

# Markdown content
markdown: """
    # Heading

    This is a paragraph with **bold** and *italic* text.

    - Item 1
    - Item 2
    - Item 3

    ```rust
    let x = 42;
    ```
"""

# No leading/trailing blank lines
compact: """
    Line one
    Line two
    Line three
"""

# In objects
document: {
    title: "My Document",
    content: """
        This is the document content.
        It spans multiple lines.
        And preserves formatting.
    """,
    footer: "Copyright 2024",
}

# In arrays
templates: [
    """
        Template One
        With multiple lines
    """,
    """
        Template Two
        Also with lines
    """,
]

# Edge case: single line in triple quotes
single_line: """
    Just one line
"""

# Edge case: empty-ish (just whitespace lines get trimmed)
nearly_empty: """
    x
"""
