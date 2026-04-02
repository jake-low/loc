CoffeeScript block comment spanning multiple lines
lines=6 code=3 comments=3 blank=0
---
### This is a
    multiline
    comment ###
greet = (name) ->
    console.log "Hello, #{name}!"
greet "World"
