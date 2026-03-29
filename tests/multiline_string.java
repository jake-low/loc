Java text block contents that look like comments are still code
lines=4 code=4 comments=0 blank=0
---
String s = """
    // not a comment
    /* also not a comment */
    """;
