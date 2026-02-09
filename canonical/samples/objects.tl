# Canonical test: Objects
# Tests: empty objects, simple objects, nested objects, mixed content

empty_object: {}

simple_object: {
    name: alice
    age: 30
    active: true
}

nested_object: {
    user: {
        name: bob
        email: "bob@example.com"
    }
    settings: {
        theme: dark
        notifications: true
    }
}

deeply_nested: {
    level1: {
        level2: {
            level3: {
                level4: {
                    value: "deep value"
                }
            }
        }
    }
}

mixed_content: {
    id: 1
    name: "Test Record"
    tags: [alpha, beta, gamma]
    metadata: {
        created: "2024-01-15"
        version: 2
    }
    scores: [95.5, 87.3, 92.1]
    enabled: true
    notes: ~
}
