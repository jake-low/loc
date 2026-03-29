/* inside a double-quoted string should not start a block comment
lines=2 code=2 comments=0 blank=0
---
char *s = "/* not a comment";
int x = 1;
