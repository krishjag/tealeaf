# LLM Context Engineering

TeaLeaf's primary use case is context engineering for Large Language Model applications. This guide explains why and how.

## The Problem

LLM context windows are limited and expensive. Typical structured data (tool definitions, conversation history, user profiles) consumes tokens proportional to format verbosity:

```json
{
  "messages": [
    {"role": "user", "content": "Hello", "tokens": 2},
    {"role": "assistant", "content": "Hi there!", "tokens": 3},
    {"role": "user", "content": "What's the weather?", "tokens": 5},
    {"role": "assistant", "content": "Let me check...", "tokens": 4}
  ]
}
```

Every message repeats `"role"`, `"content"`, `"tokens"`. With 50+ messages, this overhead adds up.

## The TeaLeaf Approach

```tl
@struct Message (role: string, content: string, tokens: int?)

messages: @table Message [
  (user, Hello, 2),
  (assistant, "Hi there!", 3),
  (user, "What's the weather?", 5),
  (assistant, "Let me check...", 4),
]
```

Field names defined once. Data is positional. For 50 messages, this saves ~40% in text size and ~80% in binary.

## Context Assembly Pattern

### Define Schemas for Your Context

```tl
@struct Tool (name: string, description: string, params: []string)
@struct Message (role: string, content: string, tokens: int?)
@struct UserProfile (id: int, name: string, preferences: []string)

system_prompt: """
  You are a helpful assistant with access to the user's profile
  and conversation history. Use the tools when appropriate.
"""

user: @table UserProfile [
  (42, "Alice", ["concise_responses", "code_examples"]),
]

tools: @table Tool [
  (search, "Search the web for information", ["query"]),
  (calculate, "Evaluate a mathematical expression", ["expression"]),
  (weather, "Get current weather for a location", ["city", "country"]),
]

history: @table Message [
  (user, Hello, 2),
  (assistant, "Hi there! How can I help?", 7),
]
```

### Binary Caching

Compiled `.tlbx` files make excellent context caches:

```rust
use tealeaf_core::{TeaLeafBuilder, ToTeaLeaf};

// Build context document
let doc = TeaLeafBuilder::new()
    .add_value("system_prompt", Value::String(system_prompt))
    .add_vec("tools", &tools)
    .add_vec("history", &messages)
    .add("user", &user_profile)
    .build();

// Cache as binary (fast to read back)
doc.compile("context_cache.tlbx", true)?;

// Later: load instantly
let cached = TeaLeaf::load("context_cache.tlbx")?;
```

### Sending to LLM

Convert to text for LLM consumption:

```rust
let doc = TeaLeaf::load("context.tl")?;
let context_text = doc.to_tl_with_schemas();
// Send context_text as part of the prompt
```

Or convert specific sections:

```rust
let doc = TeaLeaf::load("context.tl")?;
let json = doc.to_json()?;
// Use JSON for APIs that expect it
```

## Size Comparison: Real-World Context

For a typical LLM context with 50 messages, 10 tools, and a user profile:

| Format | Approximate Size |
|--------|-----------------|
| JSON | ~15 KB |
| TeaLeaf Text | ~8 KB |
| TeaLeaf Binary | ~4 KB |
| TeaLeaf Binary (compressed) | ~3 KB |

The token savings are proportional to size savings in text mode.

## Structured Outputs

LLMs can also produce TeaLeaf-formatted responses:

```tl
@struct Insight (category: string, finding: string, confidence: float)

analysis: @table Insight [
  (revenue, "Q4 revenue grew 15% YoY", 0.92),
  (churn, "Customer churn decreased by 3%", 0.87),
  (forecast, "Projected 20% growth in Q1", 0.73),
]
```

This can then be parsed and processed programmatically:

```rust
let response = TeaLeaf::parse(&llm_output)?;
if let Some(Value::Array(insights)) = response.get("analysis") {
    for insight in insights {
        // Process each structured insight
    }
}
```

## Best Practices

1. **Define schemas for all structured context** -- tool definitions, messages, profiles
2. **Use `@table` for arrays of uniform objects** -- conversation history, search results
3. **Cache compiled binary** for frequently-used context segments
4. **Use text format for LLM input** -- models understand the schema notation
5. **String deduplication** helps when context has repetitive strings (roles, tool names)
6. **Separate static and dynamic context** -- compile static context once, merge at runtime
