// single-quoted string containing /* should not start a block comment (TypeScript)
lines=3 code=3 comments=0 blank=0
---
const OPEN = '/*';
const CLOSE = '*/';
doSomething(OPEN, CLOSE);
