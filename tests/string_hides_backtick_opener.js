Backtick inside a single-quoted string should not open a template literal
lines=3 code=2 comments=1 blank=0
---
const ch = '`';
// this is a comment, not inside a template literal
const x = 1;
