/* inside a Go backtick string should not start a block comment
lines=3 code=3 comments=0 blank=0
---
x := `/* not a comment
*/`
code_line
