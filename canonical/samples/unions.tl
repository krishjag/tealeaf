# Canonical test: Discriminated unions
# Tests: @union directive, variants with fields, empty variants

# Simple shape union
@union Shape {
    circle (radius: float),
    rectangle (width: float, height: float),
    point (),
}

# Result union (like Rust's Result)
@union Result {
    ok (value: int),
    error (code: int, message: string),
}

# Option-like union
@union Maybe {
    some (value: string),
    none (),
}

# Using unions with tagged values
shapes: [
    :circle (5.0),
    :rectangle (10.0, 20.0),
    :point (),
    :circle (2.5),
]

results: [
    :ok (200),
    :error (404, "Not Found"),
    :ok (201),
    :error (500, "Internal Server Error"),
]

maybes: [
    :some ("hello"),
    :none (),
    :some ("world"),
    :none (),
]

# Single tagged values
current_shape: :circle (3.14)
api_result: :ok (42)
optional_name: :some ("Alice")
empty_optional: :none ()

# Nested in objects
drawing: {
    name: "My Drawing",
    elements: [
        :circle (10.0),
        :rectangle (5.0, 8.0),
        :point (),
    ],
}

# Complex union usage
@union Event {
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
