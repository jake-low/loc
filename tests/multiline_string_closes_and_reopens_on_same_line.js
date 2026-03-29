Closing and re-opening a backtick string on one line
lines=3 code=3 comments=0 blank=0
---
let a = `closed`; let b = `
// should be code (inside string b), not a comment
`;
