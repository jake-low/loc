Common Lisp block comment spanning multiple lines
lines=6 code=3 comments=3 blank=0
---
#| This is a
   multiline
   comment |#
(defun greet (name)
    (format t "Hello, ~a!~%" name))
(greet "World")
