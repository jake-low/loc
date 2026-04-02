Smoke test: code, blank lines, and line comment
lines=8 code=6 comments=1 blank=1
---
# schema for an address book entry
@0xcadda024352070d0;

struct Person {
  name  @0 :Text;
  email @1 :Text;
  age   @2 :UInt32;
}
