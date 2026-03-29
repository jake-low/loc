@" inside a single-line string should not open a verbatim string literal
lines=3 code=2 comments=1 blank=0
---
string s = "contains @\" inside";
// this is a comment
int x = 1;
