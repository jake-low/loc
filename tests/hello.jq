Smoke test: code, blank line, and line comment
lines=6 code=4 comments=1 blank=1
---
# filter results to find good scores

.results
| map(select(.score > 80))
| sort_by(.score)
| reverse
