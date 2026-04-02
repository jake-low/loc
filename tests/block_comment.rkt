Racket block comment spanning multiple lines
lines=6 code=3 comments=3 blank=0
---
#| This is a
   multiline
   comment |#
(define (greet name)
    (displayln (string-append "Hello, " name "!")))
(greet "World")
