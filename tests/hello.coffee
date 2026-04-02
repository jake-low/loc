Smoke test: code, blank line, and line comment
lines=5 code=3 comments=1 blank=1
---
# prints a greeting

greet = (name) ->
    console.log "Hello, #{name}!"
greet "World"
