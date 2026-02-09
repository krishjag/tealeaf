# =============================================================================
# @include Example — Modular LLM Context
# =============================================================================
#
# This example shows how @include breaks a large context file into reusable
# pieces. Schemas are defined once and shared across data files.
#
# File layout:
#   main.tl                 <- you are here (entry point)
#   schemas/types.tl        <- shared schema definitions
#   data/team.tl            <- team member data
#   data/endpoints.tl       <- API endpoint data
#
# Usage:
#   tealeaf to-json examples/includes/main.tl
#   tealeaf compile examples/includes/main.tl -o context.tlbx
#

# Include shared schemas first — order matters, schemas must be
# defined before any @table that references them.
@include "schemas/types.tl"

# Include domain data files. Each file adds its own top-level keys.
# The schemas from types.tl are available to all included files.
@include "data/team.tl"
@include "data/endpoints.tl"

# Local data alongside included data — everything merges into one document.
project: {
  name: "Acme API"
  version: "2.1.0"
  description: "User management service"
}
