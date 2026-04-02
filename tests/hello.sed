Smoke test: code, blank line, and line comment
lines=5 code=3 comments=1 blank=1
---
# strip HTML tags and collapse whitespace

s/<[^>]*>//g
s/[[:space:]]\{2,\}/ /g
s/^[[:space:]]*//
