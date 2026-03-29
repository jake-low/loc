Smoke test: code, blank line, and line comment
lines=6 code=4 comments=1 blank=1
---
object Hello {
  // greets the user

  def greet(name: String): Unit =
    println(s"hello, $name!")
}
