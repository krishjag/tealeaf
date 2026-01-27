# Canonical test: Large data stress tests
# Tests: huge arrays, deep nesting, long strings, large maps

# =============================================================================
# Huge Arrays
# =============================================================================

# Array with 100 integers
hundred_ints: [
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9,
    10, 11, 12, 13, 14, 15, 16, 17, 18, 19,
    20, 21, 22, 23, 24, 25, 26, 27, 28, 29,
    30, 31, 32, 33, 34, 35, 36, 37, 38, 39,
    40, 41, 42, 43, 44, 45, 46, 47, 48, 49,
    50, 51, 52, 53, 54, 55, 56, 57, 58, 59,
    60, 61, 62, 63, 64, 65, 66, 67, 68, 69,
    70, 71, 72, 73, 74, 75, 76, 77, 78, 79,
    80, 81, 82, 83, 84, 85, 86, 87, 88, 89,
    90, 91, 92, 93, 94, 95, 96, 97, 98, 99,
]

# Array with 50 strings
fifty_strings: [
    "str_00", "str_01", "str_02", "str_03", "str_04",
    "str_05", "str_06", "str_07", "str_08", "str_09",
    "str_10", "str_11", "str_12", "str_13", "str_14",
    "str_15", "str_16", "str_17", "str_18", "str_19",
    "str_20", "str_21", "str_22", "str_23", "str_24",
    "str_25", "str_26", "str_27", "str_28", "str_29",
    "str_30", "str_31", "str_32", "str_33", "str_34",
    "str_35", "str_36", "str_37", "str_38", "str_39",
    "str_40", "str_41", "str_42", "str_43", "str_44",
    "str_45", "str_46", "str_47", "str_48", "str_49",
]

# Array with 30 objects
thirty_objects: [
    {id: 0, name: "item_00", value: 0.0},
    {id: 1, name: "item_01", value: 1.1},
    {id: 2, name: "item_02", value: 2.2},
    {id: 3, name: "item_03", value: 3.3},
    {id: 4, name: "item_04", value: 4.4},
    {id: 5, name: "item_05", value: 5.5},
    {id: 6, name: "item_06", value: 6.6},
    {id: 7, name: "item_07", value: 7.7},
    {id: 8, name: "item_08", value: 8.8},
    {id: 9, name: "item_09", value: 9.9},
    {id: 10, name: "item_10", value: 10.0},
    {id: 11, name: "item_11", value: 11.1},
    {id: 12, name: "item_12", value: 12.2},
    {id: 13, name: "item_13", value: 13.3},
    {id: 14, name: "item_14", value: 14.4},
    {id: 15, name: "item_15", value: 15.5},
    {id: 16, name: "item_16", value: 16.6},
    {id: 17, name: "item_17", value: 17.7},
    {id: 18, name: "item_18", value: 18.8},
    {id: 19, name: "item_19", value: 19.9},
    {id: 20, name: "item_20", value: 20.0},
    {id: 21, name: "item_21", value: 21.1},
    {id: 22, name: "item_22", value: 22.2},
    {id: 23, name: "item_23", value: 23.3},
    {id: 24, name: "item_24", value: 24.4},
    {id: 25, name: "item_25", value: 25.5},
    {id: 26, name: "item_26", value: 26.6},
    {id: 27, name: "item_27", value: 27.7},
    {id: 28, name: "item_28", value: 28.8},
    {id: 29, name: "item_29", value: 29.9},
]

# =============================================================================
# Deep Nesting
# =============================================================================

# 10 levels of nested objects
deep_object: {
    level1: {
        level2: {
            level3: {
                level4: {
                    level5: {
                        level6: {
                            level7: {
                                level8: {
                                    level9: {
                                        level10: {
                                            value: "deepest",
                                            depth: 10,
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

# 10 levels of nested arrays
deep_array: [[[[[[[[[[
    "innermost"
]]]]]]]]]]

# Mixed deep nesting (alternating objects and arrays)
deep_mixed: {
    arr1: [{
        arr2: [{
            arr3: [{
                arr4: [{
                    arr5: [{
                        value: "deep_mixed_value",
                    }]
                }]
            }]
        }]
    }]
}

# Wide and deep: object with many keys, each containing nested data
wide_deep: {
    branch_a: {sub1: {leaf: "a1"}, sub2: {leaf: "a2"}, sub3: {leaf: "a3"}},
    branch_b: {sub1: {leaf: "b1"}, sub2: {leaf: "b2"}, sub3: {leaf: "b3"}},
    branch_c: {sub1: {leaf: "c1"}, sub2: {leaf: "c2"}, sub3: {leaf: "c3"}},
    branch_d: {sub1: {leaf: "d1"}, sub2: {leaf: "d2"}, sub3: {leaf: "d3"}},
    branch_e: {sub1: {leaf: "e1"}, sub2: {leaf: "e2"}, sub3: {leaf: "e3"}},
}

# =============================================================================
# Long Strings
# =============================================================================

# 500 character string
long_string_500: "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur. Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum. Curabitur pretium."

# 1000 character string
long_string_1000: "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur. Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum. Curabitur pretium tincidunt lacus. Nulla gravida orci a odio. Nullam varius, turpis et commodo pharetra, est eros bibendum elit, nec luctus magna felis sollicitudin mauris. Integer in mauris eu nibh euismod gravida. Duis ac tellus et risus vulputate vehicula. Donec lobortis risus a elit. Etiam tempor. Ut ullamcorper, ligula eu tempor congue, eros est euismod turpis, id tincidunt sapien risus a quam. Maecenas fermentum consequat mi. Donec fermentum. Pellentesque malesuada nulla a mi dui."

# Long multiline string
long_multiline: """
    This is a very long multiline string that spans many lines.
    Line 2: Lorem ipsum dolor sit amet, consectetur adipiscing elit.
    Line 3: Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.
    Line 4: Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris.
    Line 5: Nisi ut aliquip ex ea commodo consequat.
    Line 6: Duis aute irure dolor in reprehenderit in voluptate velit esse.
    Line 7: Cillum dolore eu fugiat nulla pariatur.
    Line 8: Excepteur sint occaecat cupidatat non proident.
    Line 9: Sunt in culpa qui officia deserunt mollit anim id est laborum.
    Line 10: Curabitur pretium tincidunt lacus.
    Line 11: Nulla gravida orci a odio.
    Line 12: Nullam varius, turpis et commodo pharetra.
    Line 13: Est eros bibendum elit, nec luctus magna felis sollicitudin mauris.
    Line 14: Integer in mauris eu nibh euismod gravida.
    Line 15: Duis ac tellus et risus vulputate vehicula.
    Line 16: Donec lobortis risus a elit.
    Line 17: Etiam tempor ullamcorper ligula eu tempor congue.
    Line 18: Eros est euismod turpis, id tincidunt sapien risus a quam.
    Line 19: Maecenas fermentum consequat mi.
    Line 20: Donec fermentum pellentesque malesuada nulla a mi.
"""

# String with many escape sequences
many_escapes: "tab:\ttab:\ttab:\ttab:\tnewline:\nnewline:\nnewline:\nquote:\"quote:\"quote:\"\nbackslash:\\backslash:\\backslash:\\"

# Long Unicode string
long_unicode: "日本語テキスト繰り返し日本語テキスト繰り返し日本語テキスト繰り返し日本語テキスト繰り返し日本語テキスト繰り返し中文文本重复中文文本重复中文文本重复中文文本重复中文文本重复한국어텍스트반복한국어텍스트반복한국어텍스트반복한국어텍스트반복한국어텍스트반복"

# =============================================================================
# Large Maps
# =============================================================================

# Map with 25 string keys
large_string_map: @map {
    "key_00": "value_00",
    "key_01": "value_01",
    "key_02": "value_02",
    "key_03": "value_03",
    "key_04": "value_04",
    "key_05": "value_05",
    "key_06": "value_06",
    "key_07": "value_07",
    "key_08": "value_08",
    "key_09": "value_09",
    "key_10": "value_10",
    "key_11": "value_11",
    "key_12": "value_12",
    "key_13": "value_13",
    "key_14": "value_14",
    "key_15": "value_15",
    "key_16": "value_16",
    "key_17": "value_17",
    "key_18": "value_18",
    "key_19": "value_19",
    "key_20": "value_20",
    "key_21": "value_21",
    "key_22": "value_22",
    "key_23": "value_23",
    "key_24": "value_24",
}

# Map with 25 integer keys
large_int_map: @map {
    0: "zero",
    1: "one",
    2: "two",
    3: "three",
    4: "four",
    5: "five",
    6: "six",
    7: "seven",
    8: "eight",
    9: "nine",
    10: "ten",
    11: "eleven",
    12: "twelve",
    13: "thirteen",
    14: "fourteen",
    15: "fifteen",
    16: "sixteen",
    17: "seventeen",
    18: "eighteen",
    19: "nineteen",
    20: "twenty",
    21: "twenty-one",
    22: "twenty-two",
    23: "twenty-three",
    24: "twenty-four",
}

# Map with complex values
large_complex_map: @map {
    "user_1": {id: 1, name: "Alice", scores: [95, 87, 92]},
    "user_2": {id: 2, name: "Bob", scores: [78, 85, 90]},
    "user_3": {id: 3, name: "Charlie", scores: [88, 91, 84]},
    "user_4": {id: 4, name: "Diana", scores: [92, 89, 96]},
    "user_5": {id: 5, name: "Eve", scores: [85, 88, 91]},
    "user_6": {id: 6, name: "Frank", scores: [79, 82, 87]},
    "user_7": {id: 7, name: "Grace", scores: [94, 97, 93]},
    "user_8": {id: 8, name: "Henry", scores: [81, 84, 86]},
    "user_9": {id: 9, name: "Ivy", scores: [90, 93, 95]},
    "user_10": {id: 10, name: "Jack", scores: [76, 79, 83]},
}

# =============================================================================
# Large Schema-Based Tables
# =============================================================================

@struct Record (
    id: int,
    name: string,
    value: float,
    active: bool,
    tags: []string,
)

# Table with 20 rows
large_table: @table Record [
    (1, "record_01", 1.1, true, ["tag1", "tag2"]),
    (2, "record_02", 2.2, false, ["tag3"]),
    (3, "record_03", 3.3, true, ["tag1", "tag4", "tag5"]),
    (4, "record_04", 4.4, true, ["tag2"]),
    (5, "record_05", 5.5, false, ["tag1", "tag3"]),
    (6, "record_06", 6.6, true, ["tag4", "tag5"]),
    (7, "record_07", 7.7, false, ["tag1"]),
    (8, "record_08", 8.8, true, ["tag2", "tag3", "tag4"]),
    (9, "record_09", 9.9, true, ["tag5"]),
    (10, "record_10", 10.0, false, ["tag1", "tag2"]),
    (11, "record_11", 11.1, true, ["tag3", "tag4"]),
    (12, "record_12", 12.2, false, ["tag5", "tag1"]),
    (13, "record_13", 13.3, true, ["tag2"]),
    (14, "record_14", 14.4, true, ["tag3", "tag4", "tag5"]),
    (15, "record_15", 15.5, false, ["tag1"]),
    (16, "record_16", 16.6, true, ["tag2", "tag3"]),
    (17, "record_17", 17.7, false, ["tag4"]),
    (18, "record_18", 18.8, true, ["tag5", "tag1", "tag2"]),
    (19, "record_19", 19.9, true, ["tag3"]),
    (20, "record_20", 20.0, false, ["tag4", "tag5"]),
]

# =============================================================================
# Combined Stress Test
# =============================================================================

# Large object with many different types of data
mega_object: {
    # Nested arrays of different sizes
    small_array: [1, 2, 3],
    medium_array: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10],

    # Nested objects
    config: {
        database: {
            host: "localhost",
            port: 5432,
            name: "test_db",
            credentials: {
                user: "admin",
                encrypted: true,
            },
        },
        cache: {
            enabled: true,
            ttl: 3600,
            max_size: 1000,
        },
    },

    # Array of arrays
    matrix: [
        [1, 2, 3, 4, 5],
        [6, 7, 8, 9, 10],
        [11, 12, 13, 14, 15],
        [16, 17, 18, 19, 20],
    ],

    # Mixed types array
    mixed: [
        1,
        "two",
        3.0,
        true,
        ~,
        [1, 2, 3],
        {nested: "object"},
    ],

    # Long key names
    this_is_a_very_long_key_name_that_tests_string_handling: "value1",
    another_extremely_long_key_name_for_testing_purposes: "value2",
    yet_another_long_key_to_test_the_limits_of_key_handling: "value3",
}

# =============================================================================
# Edge Cases for Size Limits
# =============================================================================

# Empty containers at various nesting levels
empty_nested: {
    empty_obj: {},
    empty_arr: [],
    nested_empty: {
        level1: {
            level2: {
                empty_here: {},
                arr_here: [],
            }
        }
    },
}

# Single element containers
single_elements: {
    single_arr: [42],
    single_obj: {only_key: "only_value"},
    single_nested: [[["single"]]],
}

# Repeated patterns (tests deduplication)
repeated_data: {
    same_string: ["duplicate", "duplicate", "duplicate", "duplicate", "duplicate"],
    same_object: [
        {x: 1, y: 2},
        {x: 1, y: 2},
        {x: 1, y: 2},
    ],
}
