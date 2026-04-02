Smoke test: code, blank lines, and line comment
lines=6 code=3 comments=1 blank=2
---
# compute the sum of the third column in a CSV
BEGIN { FS = ","; total = 0 }

NR > 1 { total += $3 }

END { print total }
