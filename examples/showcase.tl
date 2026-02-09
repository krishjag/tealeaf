# =============================================================================
# TeaLeaf Format Showcase
# =============================================================================
#
# This file demonstrates every feature of the TeaLeaf text format (.tl) with
# detailed comments explaining syntax, semantics, and nuances.
#
# TeaLeaf is a dual-format data language: human-readable text (.tl) for
# authoring and version control, and compact binary (.tlbx) for storage and
# transmission. This file covers the text format exhaustively.
#
# Compile this file:   tealeaf compile showcase.tl -o showcase.tlbx
# Decompile it back:   tealeaf decompile showcase.tlbx -o showcase_rt.tl
# Export to JSON:       tealeaf to-json showcase.tl -o showcase.json
#
# =============================================================================


# =============================================================================
# 1. COMMENTS
# =============================================================================
#
# Comments begin with # and extend to end of line.
# They can appear on their own line or after a value (inline).
# Comments are stripped during compilation to binary.


# =============================================================================
# 2. STRINGS
# =============================================================================

# --- Simple (unquoted) strings ---
# Bare identifiers with no whitespace or special characters.
# Valid characters: letters, digits, underscore, hyphen, dot.
simple_string: hello
dotted_string: config.production
hyphenated: my-app-name
underscored: user_name

# --- Quoted strings ---
# Double-quoted strings support escape sequences.
# Escapes: \\  \"  \n  \t  \r  \b  \f  \uXXXX
quoted: "hello world"
with_escapes: "line1\nline2\ttabbed"
with_path: "/home/alice/data"
with_backslash: "C:\\Users\\alice\\data"
with_email: "alice@example.com"
with_special_chars: "key=value&foo=bar"
with_quotes: "she said \"hello\""

# --- Multiline (triple-quoted) strings ---
# Triple-quoted strings auto-dedent: the common leading whitespace of all
# non-empty lines is removed, and leading/trailing blank lines are stripped.
description: """
    This is a multiline string.
    Leading whitespace is removed based on the
    minimum indentation of content lines.

    Blank lines are preserved. Relative indentation is kept:
        - This line is indented 4 extra spaces.
        - So is this one.
    Back to base indentation.
"""

# Multiline strings work anywhere a value is expected.
code_block: """
    fn main() {
        println!("Hello, world!");
    }
"""


# =============================================================================
# 3. NUMBERS
# =============================================================================

# --- Integers ---
positive_int: 42
negative_int: -17
zero: 0
large_int: 9007199254740991      # Near i64 max

# --- Floating-point ---
pi: 3.14159
negative_float: -273.15

# --- Scientific notation ---
# Always parsed as float, regardless of whether the result is a whole number.
avogadro: 6.022e23
large_sci: 1.23e15
small: 1.5e-10
with_plus: 1.0e+5

# --- Special float values ---
# IEEE 754 special values. In JSON export, NaN and infinities become null.
not_a_number: NaN
positive_infinity: inf
negative_infinity: -inf

# --- Hexadecimal ---
# Prefix: 0x or 0X (case insensitive for prefix and digits)
hex_color: 0xFF5500
hex_mask: 0x00A1
hex_large: 0xCAFEF00D

# --- Binary ---
# Prefix: 0b or 0B
binary_flags: 0b1010
binary_byte: 0b11110000

# --- Negative hex/binary ---
# A minus sign can precede hex or binary literals.
neg_hex: -0xFF
neg_binary: -0b1010


# =============================================================================
# 4. BOOLEAN AND NULL
# =============================================================================

enabled: true
disabled: false

# The tilde ~ is the null literal.
missing_value: ~


# =============================================================================
# 5. TIMESTAMPS
# =============================================================================
#
# ISO 8601 format. Stored internally as Unix milliseconds (i64) plus
# timezone offset in minutes (i16). Offsets are preserved through round-trips.

# Date only (midnight UTC)
date_only: 2024-01-15

# Date and time (UTC, indicated by Z)
date_time: 2024-01-15T10:30:00Z

# Seconds are optional (default to :00)
no_seconds: 2024-01-15T10:30Z

# With milliseconds
precise: 2024-01-15T10:30:00.123Z

# With timezone offsets
eastern: 2024-01-15T10:30:00-05:00
india: 2024-01-15T10:30:00+05:30


# =============================================================================
# 6. OBJECTS
# =============================================================================
#
# Curly-brace delimited key-value collections.
# Keys are unquoted identifiers or quoted strings.
# Trailing commas are allowed.

# Inline object
point: {x: 10, y: 20}

# Multi-line object
server_config: {
    host: localhost,
    port: 8080,
    debug: false,
    max_connections: 100,
}

# Nested objects
database: {
    primary: {
        host: "db-primary.internal",
        port: 5432,
        pool_size: 20,
    },
    replica: {
        host: "db-replica.internal",
        port: 5432,
        pool_size: 10,
    },
}

# Empty object
empty_object: {}

# Quoted keys (for names with special characters)
http_headers: {
    "Content-Type": "application/json",
    "X-Request-ID": "abc-123",
    "Accept-Language": "en-US",
}


# =============================================================================
# 7. ARRAYS
# =============================================================================
#
# Square-bracket delimited ordered collections.
# Can hold any mix of value types. Trailing commas are allowed.

numbers: [1, 2, 3, 4, 5]
strings: [alice, bob, carol]
booleans: [true, false, true]

# Mixed types are allowed
mixed: [1, "hello", true, ~, 3.14]

# Nested arrays
matrix: [
    [1, 2, 3],
    [4, 5, 6],
    [7, 8, 9],
]

# Array of objects (no schema required)
team: [
    {name: alice, role: lead},
    {name: bob, role: engineer},
    {name: carol, role: designer},
]

# Empty array
empty_array: []


# =============================================================================
# 8. TUPLES
# =============================================================================
#
# Parenthesized value lists. IMPORTANT: standalone tuples are parsed as
# plain arrays, NOT as schema-bound objects.
#
# Tuples only become schema-bound inside a @table context (see section 10).

# This is an array [1, 2, 3], NOT a struct -- there is no schema binding.
coordinates: (1, 2, 3)

# Nested tuples are also just nested arrays.
pair_of_pairs: ((1, 2), (3, 4))


# =============================================================================
# 9. SCHEMAS WITH @struct
# =============================================================================
#
# @struct defines a named schema with typed fields.
# Field types can be omitted (defaults to string).
# These schemas enable @table for compact positional data.

# Basic struct definition
@struct color (name: string, r: int, g: int, b: int)

# Fields default to string when type is omitted
@struct tag (key, value)

# Nullable fields use the ? suffix
@struct user (
    id: int,
    name: string,
    email: string?,
    active: bool,
)

# Array fields use the [] prefix
@struct playlist (
    name: string,
    tags: []string,
    track_ids: []int,
)


# =============================================================================
# 10. TABLES WITH @table
# =============================================================================
#
# @table binds tuples to a struct schema. Each tuple's positional values
# map to the struct's fields in order. This is the primary way to write
# compact, schema-bound data.

colors: @table color [
    (red, 255, 0, 0),
    (green, 0, 255, 0),
    (blue, 0, 0, 255),
    (white, 255, 255, 255),
    (black, 0, 0, 0),
]

users: @table user [
    (1, alice, "alice@example.com", true),
    (2, bob, ~, true),                       # bob's email is null
    (3, carol, "carol@example.com", false),
]

# Tables with array fields
playlists: @table playlist [
    (favorites, [rock, jazz, blues], [101, 205, 307]),
    (workout, [edm, hiphop], [401, 402, 403, 404]),
]


# =============================================================================
# 11. NESTED STRUCTS
# =============================================================================
#
# Structs can reference other structs as field types.
# In @table tuples, nested struct fields are written as nested tuples,
# and they automatically bind to the referenced schema.

@struct address (
    street: string,
    city: string,
    state: string,
    zip: string,
)

@struct employee (
    id: int,
    name: string,
    title: string,
    office: address,
    home: address?,
)

employees: @table employee [
    (
        1,
        "Alice Smith",
        "Engineering Lead",
        ("100 Tech Blvd", "San Francisco", "CA", "94105"),
        ("42 Oak St", "Palo Alto", "CA", "94301"),
    ),
    (
        2,
        "Bob Johnson",
        "Product Manager",
        ("200 Market St", "San Francisco", "CA", "94102"),
        ~,    # no home address on file
    ),
]

# Deep nesting: structs referencing structs referencing structs
@struct payment_method (type: string, last_four: string)
@struct payment (amount: float, method: payment_method)
@struct order (id: int, customer: string, payment: payment)

orders: @table order [
    (1001, alice, (99.99, (credit, "4242"))),
    (1002, bob, (49.50, (debit, "1234"))),
]


# =============================================================================
# 12. UNIONS WITH @union
# =============================================================================
#
# @union defines a discriminated union: a type that can be one of several
# named variants, each with its own set of fields.
#
# Union definitions are encoded in the binary schema table alongside
# structs, preserving variant names, field names, and field types
# through binary round-trips.

@union shape {
    circle (radius: float),
    rectangle (width: float, height: float),
    point (),
}

@union api_result {
    ok (value: int),
    error (code: int, message: string),
}

@union maybe_string {
    some (value: string),
    none (),
}

# Use tagged values (section 13) to instantiate union variants.
# The tag name must match a variant name.

shapes: [
    :circle (5.0),
    :rectangle (10.0, 20.0),
    :point (),
    :circle (2.5),
]

api_results: [
    :ok (200),
    :error (404, "Not Found"),
    :ok (201),
    :error (500, "Internal Server Error"),
]

optional_names: [
    :some ("Alice"),
    :none (),
    :some ("Bob"),
    :none (),
]

# Single tagged values (not in arrays)
current_shape: :circle (3.14)
last_result: :ok (42)

# A more complex union
@union event {
    click (x: int, y: int),
    keypress (key: string),
    scroll (delta: int),
    resize (width: int, height: int),
}

event_log: [
    :click (100, 200),
    :keypress ("Enter"),
    :scroll (-50),
    :resize (1920, 1080),
    :click (150, 250),
]


# =============================================================================
# 13. TAGGED VALUES
# =============================================================================
#
# A colon prefix adds a discriminator tag to any value.
# Syntax: :tag_name value
#
# Tags work independently of @union -- you can use tagged values without
# defining a union. Unions add schema metadata (variant/field names/types)
# that is preserved in the binary format.

# Tagged primitives
status: :active true
priority: :high 1

# Tagged objects
notification: :email {
    to: "alice@example.com",
    subject: "Welcome",
    body: "Hello!",
}

# Tagged arrays
batch: :urgent [
    {id: 1, task: deploy},
    {id: 2, task: notify},
]

# Tagged null (useful for Option/Maybe patterns)
empty_result: :none ~

# Tags inside arrays (common pattern for heterogeneous collections)
messages: [
    :text "Hello, world!",
    :image {url: "photo.jpg", width: 800, height: 600},
    :system "User joined the chat",
]


# =============================================================================
# 14. REFERENCES
# =============================================================================
#
# References enable data reuse and graph structures.
# Define with:  !name: value
# Use with:     !name
#
# References can appear at the top level or inside objects.

# Define reusable values
!default_config: {
    timeout: 30,
    retries: 3,
    backoff: exponential,
}

!seattle: {city: Seattle, state: WA, zip: "98101"}
!portland: {city: Portland, state: OR, zip: "97201"}

# Use references -- the same value is shared, not copied.
service_a: {
    name: auth-service,
    config: !default_config,
    location: !seattle,
}

service_b: {
    name: data-service,
    config: !default_config,
    location: !portland,
}

# References in arrays
office_locations: [!seattle, !portland]

# References inside objects (as definitions)
graph: {
    !node_a: {label: "Start", value: 1},
    !node_b: {label: "Middle", value: 2},
    !node_c: {label: "End", value: 3},
    edges: [
        {from: !node_a, to: !node_b, weight: 1.0},
        {from: !node_b, to: !node_c, weight: 2.0},
        {from: !node_c, to: !node_a, weight: 0.5},
    ],
}


# =============================================================================
# 15. MAPS WITH @map
# =============================================================================
#
# Maps are ordered key-value collections that support non-string keys.
# Unlike objects (which require string keys), maps accept strings,
# integers, and bare names as keys. Maps preserve insertion order.

# String keys
http_response_headers: @map {
    "Content-Type": "application/json",
    "Cache-Control": "no-cache",
    "X-Request-ID": "req-abc-123",
}

# Integer keys
status_codes: @map {
    200: "OK",
    301: "Moved Permanently",
    404: "Not Found",
    500: "Internal Server Error",
}

# Bare name keys (treated as strings)
feature_flags: @map {
    dark_mode: true,
    beta_ui: false,
    new_search: true,
}

# Maps with complex values
error_handlers: @map {
    400: {action: reject, log: false},
    404: {action: fallback, log: true},
    500: {action: retry, log: true},
}

# Empty map
empty_map: @map {}


# =============================================================================
# 16. TYPE SYSTEM DETAILS
# =============================================================================
#
# This section demonstrates the full type system using struct definitions.

@struct type_showcase (
    # Integer types (signed)
    tiny: int8,           # -128 to 127
    small: int16,         # -32,768 to 32,767
    medium: int,          # -2B to 2B (alias: int32)
    large: int64,         # Full 64-bit range

    # Integer types (unsigned)
    ubyte: uint8,         # 0 to 255
    ushort: uint16,       # 0 to 65,535
    uint_val: uint,       # 0 to ~4B (alias: uint32)
    ubig: uint64,         # 0 to ~18 quintillion

    # Floating-point
    single: float32,      # 32-bit IEEE 754
    double: float,        # 64-bit IEEE 754 (alias: float64)

    # Other primitives
    flag: bool,
    label: string,
    when: timestamp,

    # Nullable fields: the ? suffix means the field can be ~
    maybe_name: string?,
    maybe_count: int?,

    # Array fields: the [] prefix creates a typed array
    scores: []int,
    names: []string,
    flags: []bool,

    # Nullable arrays: the field itself can be ~
    optional_tags: []string?,

    # Struct-typed fields
    location: address,

    # Array of structs
    addresses: []address,
)


# =============================================================================
# 17. PUTTING IT ALL TOGETHER
# =============================================================================
#
# A realistic example combining multiple features.

@struct product_option (name: string, value: string, price_modifier: float)
@struct product_variant (sku: string, name: string, price: float, stock: int, options: []product_option)
@struct product (id: string, name: string, category: string, base_price: float)

products: @table product [
    (PROD-001, "Wireless Headphones", "Electronics/Audio", 299.99),
    (PROD-002, "Ergonomic Chair", "Furniture/Office", 549.00),
    (PROD-003, "Smart Watch", "Electronics/Wearables", 399.00),
]

# Variants as objects (not table-bound, to show the contrast)
product_variants: [
    {
        sku: "SKU-001-BLK",
        name: "Headphones - Black",
        price: 299.99,
        stock: 150,
        options: [
            {name: color, value: Black, price_modifier: 0.0},
            {name: warranty, value: "2-Year Standard", price_modifier: 0.0},
        ],
    },
    {
        sku: "SKU-001-WHT-EXT",
        name: "Headphones - White + Extended Warranty",
        price: 349.99,
        stock: 45,
        options: [
            {name: color, value: White, price_modifier: 0.0},
            {name: warranty, value: "5-Year Extended", price_modifier: 50.0},
        ],
    },
]

# Analytics using maps for status breakdowns
order_analytics: {
    total_orders: 250,
    total_revenue: 147059.10,
    average_order_value: 588.24,
    orders_by_status: @map {
        delivered: 180,
        shipped: 35,
        processing: 15,
        cancelled: 12,
        returned: 8,
    },
    top_categories: [
        {category: "Electronics/Audio", revenue: 45000.00},
        {category: "Furniture/Office", revenue: 38000.00},
        {category: "Electronics/Wearables", revenue: 35000.00},
    ],
}


# =============================================================================
# 18. EDGE CASES AND NUANCES
# =============================================================================

# --- Null in arrays ---
# Null values are allowed in any array, including schema-typed arrays.
# In struct arrays, nulls are tracked in a null bitmap in binary format.
sparse_data: [1, ~, 3, ~, 5, ~]

# --- Trailing commas ---
# Allowed everywhere: objects, arrays, tuples, maps, struct/union fields.
trailing: {a: 1, b: 2,}

# --- Empty containers ---
empties: {
    arr: [],
    obj: {},
    map: @map {},
    tup: (),
}

# --- Bytes literals ---
# The b"..." syntax holds raw hex-encoded byte data.
# Each pair of hex digits represents one byte.
payload: b"cafef00d"
empty_bytes: b""

# --- Unicode ---
# Full UTF-8 support in quoted strings.
unicode_samples: {
    japanese: "\u3053\u3093\u306b\u3061\u306f",
    greeting: "Hello, World!",
    emoji: "coffee: \u2615",
}

# --- Nested tagged values ---
# Tags can wrap any value type, including other tagged values.
wrapped: :outer :inner "deeply tagged"

# --- Tagged values with tuples ---
# Common pattern for union variants with multiple fields.
tagged_tuple: :point (10, 20)

# --- Timestamps in tables ---
@struct log_entry (timestamp: timestamp, level: string, message: string)
logs: @table log_entry [
    (2024-01-15T10:30:00Z, info, "Server started"),
    (2024-01-15T10:30:05.123Z, warn, "High memory usage"),
    (2024-01-15T10:31:00-05:00, error, "Connection timeout"),
]

# --- Large nested structure ---
# TeaLeaf handles arbitrary nesting depth.
deeply_nested: {
    level1: {
        level2: {
            level3: {
                level4: {
                    value: "found it!",
                },
            },
        },
    },
}


# =============================================================================
# NOTES
# =============================================================================
#
# Features NOT demonstrated here (require separate files or binary format):
#
#   @include "path/to/file.tl"
#     Imports schemas, unions, and data from another file.
#     Paths are resolved relative to the including file.
#
#   @root-array
#     Marks the document as a root-level JSON array.
#     Used for JSON round-trip fidelity when the source is a JSON array.
#     Example:
#       @root-array
#       0: {id: 1, name: alice}
#       1: {id: 2, name: bob}
#
#
# For the full specification, see spec/TEALEAF_SPEC.md.
# For the formal grammar, see the Grammar section of the spec.
