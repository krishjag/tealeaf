# Grammar

The formal grammar for the TeaLeaf text format in EBNF notation.

## EBNF Grammar

```ebnf
document     = { directive | pair | ref_def } ;

directive    = struct_def | union_def | include | root_array ;
struct_def   = "@struct" name "(" fields ")" ;
union_def    = "@union" name "{" variants "}" ;
include      = "@include" string ;
root_array   = "@root-array" ;

variants     = variant { "," variant } ;
variant      = name "(" [ fields ] ")" ;

fields       = field { "," field } ;
field        = name [ ":" type ] ;  (* type defaults to string if omitted *)
type         = [ "[]" ] base_type [ "?" ] ;
base_type    = "bool" | "int" | "int8" | "int16" | "int32" | "int64"
             | "uint" | "uint8" | "uint16" | "uint32" | "uint64"
             | "float" | "float32" | "float64" | "string" | "bytes"
             | "timestamp" | name ;

pair         = key ":" value ;
key          = name | string ;
value        = primitive | object | array | tuple | table | map
             | tagged | ref | timestamp ;

primitive    = string | bytes_lit | number | bool | "~" | "null" ;
bytes_lit    = "b\"" { hexdigit hexdigit } "\"" ;
object       = "{" [ ( pair | ref_def ) { "," ( pair | ref_def ) } ] "}" ;
array        = "[" [ value { "," value } ] "]" ;
tuple        = "(" [ value { "," value } ] ")" ;
table        = "@table" name array ;
map          = "@map" "{" [ map_entry { "," map_entry } ] "}" ;
map_entry    = map_key ":" value ;
map_key      = string | name | integer ;
tagged       = ":" name value ;
ref          = "!" name ;
ref_def      = "!" name ":" value ;
timestamp    = date [ "T" time [ timezone ] ] ;

date         = digit{4} "-" digit{2} "-" digit{2} ;
time         = digit{2} ":" digit{2} [ ":" digit{2} [ "." digit{1,3} ] ] ;
timezone     = "Z" | ( "+" | "-" ) digit{2} [ ":" digit{2} | digit{2} ] ;

string       = name | '"' chars '"' | '"""' multiline '"""' ;
number       = integer | float | hex | binary ;
integer      = [ "-" ] digit+ ;
float        = [ "-" ] digit+ "." digit+ [ ("e"|"E") ["+"|"-"] digit+ ]
             | [ "-" ] digit+ ("e"|"E") ["+"|"-"] digit+
             | "NaN" | "inf" | "-inf" ;
hex          = [ "-" ] ("0x" | "0X") hexdigit+ ;
binary       = [ "-" ] ("0b" | "0B") ("0"|"1")+ ;
bool         = "true" | "false" ;
name         = (letter | "_") { letter | digit | "_" | "-" | "." } ;
comment      = "#" { any } newline ;

chars        = { any_char | escape } ;
escape       = "\\" | "\\\"" | "\\n" | "\\t" | "\\r" | "\\b" | "\\f"
             | "\\u" hexdigit hexdigit hexdigit hexdigit ;
```

## Production Notes

### Document Structure

A document is a sequence of:
- **Directives** -- `@struct`, `@union`, `@include`, `@root-array` (processed before data)
- **Pairs** -- `key: value` (the actual data)
- **Reference definitions** -- `!name: value` (reusable named values)

### Key Rules

- **Keys** can be bare identifiers (`name`) or quoted strings (`"Content-Type"`)
- **Trailing commas** are allowed in all list contexts (arrays, objects, tuples, maps, fields)
- **Comments** (`#` to end of line) can appear anywhere whitespace is valid
- **Whitespace** is insignificant except inside strings

### Type Defaults

When a field type is omitted in a `@struct`, it defaults to `string`:

```tl
@struct config (host, port: int, debug: bool)
# "host" is implicitly string
```

### Tuple Semantics

Standalone tuples are parsed as arrays. Only within a `@table` context do tuples acquire schema binding:

```tl
# This is an array [1, 2, 3]
plain: (1, 2, 3)

# These are schema-bound tuples
@struct point (x: int, y: int)
points: @table point [(0, 0), (1, 1)]
```

### Root Array Directive

The `@root-array` directive marks the document as representing a root-level JSON array rather than a JSON object. This is used for JSON round-trip fidelity -- when a JSON array is imported via `from-json`, the directive is emitted so that `to-json` produces an array at the top level instead of an object:

```tl
@root-array

0: {id: 1, name: alice}
1: {id: 2, name: bob}
```

Without `@root-array`, the JSON output would be `{"0": {...}, "1": {...}}`. With it, the output is `[{...}, {...}]`.

### Map Key Restrictions

Map keys are restricted to hashable types: strings, names, and integers. Complex values (objects, arrays) cannot be map keys.

### Reference Scoping

References can be defined at:
- **Top level** -- `!name: value` alongside pairs
- **Inside objects** -- `{!ref: value, field: !ref}`

References are resolved within the document scope.
