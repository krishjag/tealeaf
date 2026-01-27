# Canonical test: Schemas and structured data
# Tests: struct definitions, tables, nested structs, nullable fields

@struct point (x: float, y: float)
@struct user (id: int, name: string, email: string?)
@struct address (city: string, country: string, zip: string?)
@struct employee (id: int, name: string, department: string, salary: float, address: address)

# Simple struct table
points: @table point [
    (0.0, 0.0),
    (1.0, 2.0),
    (3.5, -4.2),
]

# Table with nullable fields
users: @table user [
    (1, alice, "alice@example.com"),
    (2, bob, ~),
    (3, charlie, "charlie@test.org"),
]

# Nested struct table
employees: @table employee [
    (101, "Alice Johnson", Engineering, 95000.0, (Seattle, USA, "98101")),
    (102, "Bob Smith", Marketing, 75000.0, (Portland, USA, ~)),
    (103, "Charlie Brown", Engineering, 88000.0, (Vancouver, Canada, "V6B 1A1")),
]

# Plain value alongside tables
company_name: "Acme Corporation"
employee_count: 3
