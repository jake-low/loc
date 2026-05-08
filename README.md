# loc

`loc` counts lines of code in files, reporting blank, comment, and code line counts broken down by language.

There are many other tools like it. What makes `loc` different is that it does not decide _which_ files to read; instead you pass it a list of files through stdin, and it returns statistics about those files. You can use tools like `find` or `git ls-files` to produce a list of files to pass to `loc`, depending on what you want to count.

## Installation

`loc` is written in Rust. Clone the repository and then run `cargo install --path .` to build and install it.

## Usage

Example invocations:

```
git ls-files | loc
find src/ -name '*.rs' | loc
fd --extension py | loc
```

Example output:

```
Language        Files      Bytes      Lines     Blanks   Comments       Code
TypeScript         13    76.7 KB      2,819        325         62      2,432
Rust                3    32.5 KB      1,019        146         39        834
CSS                 1    10.0 KB        562         86         10        466
TOML                1        801         28          2          0         26
Total              18   119.9 KB      4,428        559        111      3,758
```

## Counting algorithm

`loc` reads supported files line by line, and classifies each line as exactly one of code, blanks, or comments.

- Blank lines are those that are empty or contain only whitespace
- Comment lines are those that only contain comments (and whitespace)
- Code lines are those that contain any code (even if they also contain comments)

As it reads the file, `loc` keeps track of whether it is currently in a block comment or a string literal. While inside a block comment, non-blank lines are counted as comment lines. While in a multiline string, non-blank lines are counted as code. Block comment start markers are also ignored while inside strings, to avoid things like `glob("src/*.js")` being misidentified as the start of a block comment (which would then likely cause the remainder of the file to be miscounted as comment lines).

`loc` works by scanning for simple substrings (like `//` or `/*`); it does not parse code into a proper AST. This means there are edge cases where `loc` will misclassify lines.

## Supported languages

Run `loc --languages` (or see the [languages/](./languages/) directory) for a list of supported languages.

Language detection is based on file extension (e.g. `*.py`) or filename (for things like `Makefile`). If that fails, `loc` checks if the file has a `#!` line and tries to detect the language based on that. Files whose language can't be identified are silently skipped.

## Is it fast?

It's pretty fast. `loc` uses [`memchr`](https://docs.rs/memchr/) to scan through files, avoids most allocations, and processes files in parallel.

On my computer, `loc` counts all of VSCode's source code (~2.1M lines, mainly TypeScript) in 60 milliseconds.

## License

This code is available under the ISC license; see LICENSE file for details.
