Smoke test: code, blank line, and line comment
lines=6 code=4 comments=1 blank=1
---
# greets the user

{ pkgs ? import <nixpkgs> {} }:
pkgs.mkShell {
  name = "hello";
}
