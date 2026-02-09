# Canonical test: References, Tagged values, and Maps
# Tests: !ref definitions, ref usage, :tag syntax, @map directive

# =============================================================================
# References (!ref)
# =============================================================================

# Basic reference definition and usage
!default_config: {
    theme: dark,
    language: en,
    debug: false,
}

# Using the reference
app_config: !default_config
backup_config: !default_config

# Reference with nested data
!shared_address: {
    street: "123 Main St",
    city: "Springfield",
    country: "USA",
}

# References in different contexts
shipping_address: !shared_address
billing_address: !shared_address

# Reference to simple values
!pi: 3.14159
!greeting: "Hello, World!"
!max_retries: 3

circle_area_constant: !pi
welcome_message: !greeting
retry_count: !max_retries

# Reference to array
!common_tags: ["important", "review", "2024"]

document_tags: !common_tags
task_tags: !common_tags

# Multiple references in one object
settings: {
    config: !default_config,
    address: !shared_address,
    tags: !common_tags,
}

# =============================================================================
# Tagged Values (:tag)
# =============================================================================

# Simple tagged values
result_success: :ok 200
result_failure: :error "Something went wrong"

# Tagged with different value types
tagged_null: :none ~
tagged_bool: :active true
tagged_int: :count 42
tagged_float: :ratio 0.75
tagged_string: :message "Hello"

# Tagged with complex values
tagged_object: :user {
    id: 1,
    name: "Alice",
}

tagged_array: :items [1, 2, 3, 4, 5]

# Nested tags
nested_tag: :outer :inner "value"

# Tags in arrays
status_list: [
    :pending "task1",
    :completed "task2",
    :failed "task3",
]

# Tags in objects
responses: {
    api1: :success {data: "result1"},
    api2: :error {code: 404, message: "Not found"},
    api3: :timeout ~,
}

# Result pattern (common use case)
operation_results: [
    :ok {value: 100},
    :err {code: "E001", message: "Invalid input"},
    :ok {value: 200},
]

# Option pattern (common use case)
optional_values: {
    present: :some "Hello",
    absent: :none ~,
    also_present: :some 42,
}

# =============================================================================
# Maps (@map)
# =============================================================================

# Basic string-keyed map
headers: @map {
    "Content-Type": "application/json",
    "Accept": "*/*",
    "Authorization": "Bearer token123",
}

# Integer-keyed map
http_codes: @map {
    200: "OK",
    201: "Created",
    400: "Bad Request",
    404: "Not Found",
    500: "Internal Server Error",
}

# Mixed key types
mixed_keys: @map {
    1: "one",
    "two": 2,
    3: "three",
}

# Map with complex values
user_profiles: @map {
    "alice": {id: 1, role: "admin"},
    "bob": {id: 2, role: "user"},
    "charlie": {id: 3, role: "guest"},
}

# Empty map
empty_map: @map {}

# Map with array values
category_items: @map {
    "fruits": ["apple", "banana", "cherry"],
    "vegetables": ["carrot", "broccoli", "spinach"],
}

# Nested maps
nested_maps: @map {
    "level1": @map {
        "level2a": "value_a",
        "level2b": "value_b",
    },
}

# Map with tagged values
tagged_map: @map {
    "key1": :some "value1",
    "key2": :none ~,
}

# =============================================================================
# Combined Usage
# =============================================================================

# Reference used as map value
!default_permissions: ["read", "write"]

role_permissions: @map {
    "admin": ["read", "write", "delete", "admin"],
    "user": !default_permissions,
    "guest": ["read"],
}

# Tagged reference
tagged_ref: :config !default_config

# Complex combination
api_response: {
    status: :ok 200,
    headers: @map {
        "Content-Type": "application/json",
    },
    body: {
        config: !default_config,
        tags: !common_tags,
    },
}
