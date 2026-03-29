Triple quote inside a single-line string should not open a text block
lines=3 code=2 comments=1 blank=0
---
String s = "not a \"\"\" text block";
// this is a comment
int x = 1;
