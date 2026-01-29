# Canonical test: Mixed schema-bound and schemaless data
# Tests: @struct definitions alongside untyped objects, @table with plain arrays

# =============================================================================
# Schema Definitions
# =============================================================================

@struct Point (x: float, y: float)

@struct Person (
    name: string,
    age: int,
    email: string?,
)

@struct Address (
    street: string,
    city: string,
    zip: string,
    country: string,
)

@struct Employee (
    id: int,
    person: Person,
    department: string,
    salary: float?,
    address: Address?,
)

@union Status {
    active (),
    inactive (reason: string),
    pending (since: timestamp),
}

# =============================================================================
# Schema-Bound Data (using @table)
# =============================================================================

# Points using schema
points: @table Point [
    (0.0, 0.0),
    (10.5, 20.3),
    (-5.0, 15.0),
]

# People using schema
people: @table Person [
    ("Alice", 30, "alice@example.com"),
    ("Bob", 25, ~),
    ("Charlie", 35, "charlie@example.com"),
]

# Employees using nested schema
employees: @table Employee [
    (1, ("Alice Smith", 30, "alice@corp.com"), "Engineering", 85000.0, ("123 Tech Blvd", "San Jose", "95101", "USA")),
    (2, ("Bob Jones", 25, ~), "Marketing", 65000.0, ~),
    (3, ("Carol White", 40, "carol@corp.com"), "Engineering", 95000.0, ("456 Code Ave", "Austin", "78701", "USA")),
]

# =============================================================================
# Schemaless Data (plain objects/arrays)
# =============================================================================

# Plain object (no schema)
metadata: {
    version: "1.0.0",
    generated_at: 2024-01-15T10:30:00Z,
    author: "System",
    tags: ["auto-generated", "test-data"],
}

# Plain array of objects (no schema)
logs: [
    {timestamp: 2024-01-15T09:00:00Z, level: "INFO", message: "Application started"},
    {timestamp: 2024-01-15T09:01:00Z, level: "DEBUG", message: "Loading configuration"},
    {timestamp: 2024-01-15T09:02:00Z, level: "WARN", message: "Cache miss", details: {key: "user_123"}},
]

# Heterogeneous array (different shapes)
mixed_items: [
    {type: "text", content: "Hello"},
    {type: "number", value: 42},
    {type: "point", x: 10, y: 20},
    {type: "nested", data: {a: 1, b: 2, c: 3}},
]

# =============================================================================
# Mixed in Same Context
# =============================================================================

# Object with both schema-bound and schemaless fields
dashboard: {
    # Schema-bound data
    featured_points: @table Point [
        (0.0, 0.0),
        (100.0, 100.0),
    ],

    # Schemaless data
    settings: {
        theme: "dark",
        sidebar_width: 250,
        show_grid: true,
    },

    # Another schema-bound table
    team: @table Person [
        ("Dana", 28, "dana@team.com"),
        ("Eve", 32, ~),
    ],

    # Schemaless nested object
    filters: {
        date_range: {
            start: 2024-01-01,
            end: 2024-12-31,
        },
        categories: ["A", "B", "C"],
        include_archived: false,
    },
}

# =============================================================================
# Edge Cases
# =============================================================================

# Single row table
single_point: @table Point [
    (5.0, 5.0),
]

# Empty table
empty_points: @table Point []

# Schemaless data with same field names as schema (but not using schema)
point_like_objects: [
    {x: 1.0, y: 2.0, label: "A"},
    {x: 3.0, y: 4.0, label: "B", color: "red"},
]

# Complex nested mix
report: {
    title: "Quarterly Report",

    # Schema-bound section
    key_employees: @table Employee [
        (100, ("John Doe", 45, "john@exec.com"), "Executive", 150000.0, ("1 Corporate Way", "New York", "10001", "USA")),
    ],

    # Schemaless section
    summary: {
        revenue: 1000000.50,
        expenses: 750000.25,
        headcount: 50,
        departments: ["Engineering", "Sales", "Marketing", "HR"],
    },

    # Tagged value (schemaless)
    status: :approved {
        by: "CFO",
        date: 2024-01-15,
    },

    # Map (schemaless)
    regional_data: @map {
        "NA": {revenue: 500000, growth: 0.15},
        "EU": {revenue: 300000, growth: 0.10},
        "APAC": {revenue: 200000, growth: 0.25},
    },
}

# =============================================================================
# Union with Schema
# =============================================================================

# Using the Status union defined above
employee_statuses: [
    {employee_id: 1, status: :active ()},
    {employee_id: 2, status: :inactive ("On leave")},
    {employee_id: 3, status: :pending (2024-02-01T00:00:00Z)},
]
