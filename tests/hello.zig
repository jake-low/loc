Smoke test: code, blank line, and line comment
lines=6 code=4 comments=1 blank=1
---
const std = @import("std");

// greets the user
pub fn main() void {
    std.debug.print("hello, world!\n", .{});
}
