// first /* is inside a string; second /* later on the same line is a real block comment
lines=3 code=1 comments=2 blank=0
---
char *s = "/*"; /* open comment
this line is a comment
*/
