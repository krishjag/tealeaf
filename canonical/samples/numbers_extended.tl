# Canonical test: Extended number formats
# Tests: hexadecimal, binary, scientific notation, large integers

# Hexadecimal integers
hex_simple: 0xFF
hex_color: 0xFF5500
hex_mask: 0x00A1
hex_large: 0xDEADBEEF

# Binary integers
binary_simple: 0b1010
binary_byte: 0b11110000
binary_flags: 0b10101010

# Scientific notation (floats)
scientific_small: 1.5e-10
scientific_large: 6.022e23
scientific_positive: 1.0e+5
scientific_negative_exp: 2.5e-3

# Large integers (near i64 limits)
large_positive: 9007199254740991
large_negative: -9007199254740991

# Integer edge cases
int_max_i32: 2147483647
int_min_i32: -2147483648
zero: 0
negative_zero: -0

# Float edge cases
float_tiny: 0.000001
float_precision: 3.141592653589793

# Mixed in arrays
hex_array: [0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F]
binary_array: [0b0001, 0b0010, 0b0100, 0b1000]
scientific_array: [1e1, 1e2, 1e3, 1e4]

# Mixed in objects
bit_flags: {
    read: 0b0001,
    write: 0b0010,
    execute: 0b0100,
    all: 0b0111,
}

color_palette: {
    red: 0xFF0000,
    green: 0x00FF00,
    blue: 0x0000FF,
    white: 0xFFFFFF,
    black: 0x000000,
}
