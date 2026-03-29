Smoke test: code, blank line, and block comment
lines=5 code=3 comments=1 blank=1
---
(* greets the user *)
let greet name =
  Printf.printf "hello, %s!\n" name

let () = greet "world"
