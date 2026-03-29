Smoke test: code, blank line, and line comment
lines=5 code=3 comments=1 blank=1
---
FROM ubuntu:22.04

# install dependencies
RUN apt-get update
RUN apt-get install -y curl
