Smoke test: code, blank line, and line comment
lines=5 code=3 comments=1 blank=1
---
(ns hello.core)

; greets the user
(defn greet [name]
  (println (str "hello, " name "!")))
