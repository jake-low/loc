Backslash before backtick does not escape it in Go; the backtick closes the string
lines=3 code=2 comments=1 blank=0
---
s := `start
\`
// real comment
