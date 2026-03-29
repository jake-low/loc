// escaped single-quote in char literal should not confuse the block comment scanner
lines=3 code=1 comments=2 blank=0
---
char c = '\''; /*
this line is a comment
*/
