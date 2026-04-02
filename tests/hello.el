Bind a key to display a greeting in the minibuffer
lines=6 code=4 comments=1 blank=1
---
; Bind C-c h to display a greeting in the minibuffer
(defun greet ()
  (interactive)
  (message "Hello, world!"))

(global-set-key (kbd "C-c h") 'greet)
