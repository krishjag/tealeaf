# API endpoints — uses the `endpoint` and `param` schemas from schemas/types.tl.
# This file is included by main.tl, which includes the schema first.

endpoints: @table endpoint [
  (GET,    "/users",      "List all users",       [(limit, int, false), (offset, int, false)], "[]user"),
  (GET,    "/users/{id}", "Get user by ID",       [(id, string, true)],                        user),
  (POST,   "/users",      "Create a new user",    ~,                                           user),
  (DELETE, "/users/{id}", "Delete user by ID",    [(id, string, true)],                        "204 No Content"),
]
