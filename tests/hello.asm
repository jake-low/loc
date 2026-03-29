Smoke test: code, blank line, and line comment
lines=6 code=4 comments=1 blank=1
---
section .data
    msg db "hello, world!", 0

; prints a greeting
section .text
    global _start
