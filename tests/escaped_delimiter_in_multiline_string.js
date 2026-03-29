Escaped delimiter inside a backtick string should not close the string
lines=3 code=3 comments=0 blank=0
---
var q = `UPDATE
SET \`name\` = "foo"`
code_line
