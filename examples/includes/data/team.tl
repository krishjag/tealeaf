# Team members — uses the `member` schema from schemas/types.tl.
# This file is included by main.tl, which includes the schema first.

team: @table member [
  ("Alice Chen",   lead,     "alice@example.com"),
  ("Bob Park",     backend,  "bob@example.com"),
  ("Carol Reyes",  frontend, "carol@example.com"),
]
