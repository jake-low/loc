# inside a triple-quoted string should be code, not a line comment
lines=3 code=3 comments=0 blank=0
---
x = """
# not a comment
"""
