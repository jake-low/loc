Smoke test: code, blank line, and line comment
lines=7 code=5 comments=1 blank=1
---
defmodule Hello do
  # greets the user

  def greet(name) do
    IO.puts("hello, #{name}!")
  end
end
