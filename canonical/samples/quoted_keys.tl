# Canonical test: Quoted field names in struct definitions
# Tests: @-prefixed (JSON-LD), $-prefixed (JSON Schema), #-prefixed (XML-to-JSON),
#        colon keys (XML namespaces), URI keys (RDF/JSON), space keys (spreadsheet),
#        and mixed quoted/unquoted fields in a single schema.

# JSON-LD style: @type, @id
@struct jsonld_record ("@type": string, "@id": string, name: string)
jsonld: @table jsonld_record [
    (Dataset, "urn:dataset:1", "Climate Data"),
    (Distribution, "urn:dist:1", "CSV Download"),
]

# JSON Schema / OpenAPI style: $ref, $id
@struct schema_def ("$ref": string, "$id": string, description: string)
definitions: @table schema_def [
    ("/components/User", def-001, "User schema reference"),
    ("/components/Order", def-002, "Order schema reference"),
]

# XML-to-JSON style: #text, #comment
@struct xml_node ("#text": string, tag: string, "#comment": string?)
xml_nodes: @table xml_node [
    ("Hello world", p, "intro paragraph"),
    (Goodbye, span, ~),
]

# XML namespace style: xsi:type, dc:title
@struct ns_element ("xsi:type": string, "dc:title": string, id: int)
namespaced: @table ns_element [
    (string, "Document A", 1),
    (int, "Document B", 2),
]

# OData style: @odata.type, @odata.id
@struct odata_entity ("@odata.type": string, "@odata.id": string, displayName: string)
odata_results: @table odata_entity [
    ("Microsoft.Graph.User", "users/1", Alice),
    ("Microsoft.Graph.User", "users/2", Bob),
]

# URI keys (RDF/JSON)
@struct rdf_triple ("http://schema.org/name": string, "http://schema.org/age": string, id: string)
triples: @table rdf_triple [
    (Alice, "30", s1),
    (Bob, "25", s2),
]

# Space in keys (spreadsheet exports)
@struct contact ("First Name": string, "Last Name": string, age: int)
contacts: @table contact [
    (Alice, Smith, 30),
    (Bob, Jones, 25),
]

# Mixed: some fields quoted, some not
@struct catalog_item ("@type": string, "$id": string, name: string, "sku:code": string)
catalog: @table catalog_item [
    (Product, p1, Widget, "W-100"),
    (Product, p2, Gadget, "G-200"),
]
