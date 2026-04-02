Bind a key to display a greeting
lines=6 code=4 comments=1 blank=1
---
" Bind <leader>h to display a greeting
function! Greet()
  echo "Hello, world!"
endfunction

nnoremap <leader>h :call Greet()<CR>
