# Canonical test: Reference cycles
# Tests: cyclic references, self-references, forward references

# =============================================================================
# Cyclic References (A -> B -> A)
# =============================================================================

# Define node_a which references node_b
!node_a: {
    name: "Node A",
    next: !node_b,
}

# Define node_b which references node_a (creating a cycle)
!node_b: {
    name: "Node B",
    next: !node_a,
}

# Use both nodes
cycle_start: !node_a
cycle_other: !node_b

# =============================================================================
# Self-Reference (A -> A)
# =============================================================================

# A node that references itself
!self_ref: {
    name: "Self-referential",
    self: !self_ref,
}

self_referential: !self_ref

# =============================================================================
# Forward References
# =============================================================================

# Use a reference before it's defined
forward_usage: !forward_target

# Define the reference target after use
!forward_target: {
    name: "Defined later",
    value: 42,
}

# =============================================================================
# Three-Node Cycle (A -> B -> C -> A)
# =============================================================================

!cycle3_a: {name: "A", next: !cycle3_b}
!cycle3_b: {name: "B", next: !cycle3_c}
!cycle3_c: {name: "C", next: !cycle3_a}

three_cycle: [!cycle3_a, !cycle3_b, !cycle3_c]

# =============================================================================
# Reference in Different Contexts
# =============================================================================

# Reference inside array
refs_in_array: [!node_a, !node_b, !node_a]

# Reference inside nested object
refs_in_object: {
    primary: !node_a,
    secondary: !node_b,
    primary_again: !node_a,
}

# Reference as tagged value
tagged_ref: :active !node_a

# Reference in map value
ref_map: @map {
    "first": !node_a,
    "second": !node_b,
}

# =============================================================================
# Mixed Cyclic and Non-Cyclic
# =============================================================================

# Non-cyclic reference for comparison
!standalone: {
    name: "Standalone",
    value: 100,
}

mixed: {
    cyclic: !node_a,
    non_cyclic: !standalone,
}

# =============================================================================
# Deeply Nested Cycle
# =============================================================================

!deep_a: {
    level1: {
        level2: {
            ref: !deep_b,
        }
    }
}

!deep_b: {
    level1: {
        level2: {
            ref: !deep_a,
        }
    }
}

deep_cycle: !deep_a
