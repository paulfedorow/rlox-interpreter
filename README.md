# rlox-interpreter

rlox-interpreter is a Rust implementation of Bob Nystrom's toy language, [Lox](https://craftinginterpreters.com/).

Disclaimer: This is my first Rust project, do not expect idiomatic or performant code. :wink:

## Build

Execute the following commands to build rlox-interpreter:

    cargo build --release

## Usage

Start a repl:

    target/release/rlox-interpreter

Execute a Lox script:

    target/release/rlox-interpreter examples/class.lox

## Benchmarks

rlox-interpreter is implemented as an AST-walking interpreter. Here are some execution times from running each of the
benchmarks on a AMD Ryzen 7 Pro 5850U. For comparison there are also the execution times for Bob Nystrom's AST-walking
interpreter (which is programmed in Java).

|                     | rlox-interpreter | jlox     | rlox-interpreter/jlox |
|---------------------|------------------|----------|-----------------------|
| binary_trees.lox    |         84.363 s |  4.624 s |                 18.24 |
| equality.lox        |         17.440 s |  2.849 s |                  6.12 |
| fib.lox             |         44.646 s |  2.884 s |                 15.48 |
| instantiation.lox   |          6.761 s |  1.126 s |                  6.00 |
| invocation.lox      |          5.978 s |  1.034 s |                  5.78 |
| method_call.lox     |          8.279 s |  1.258 s |                  6.58 |
| properties.lox      |         14.384 s |  3.201 s |                  4.49 |
| string_equality.lox |         10.793 s |  2.273 s |                  4.75 |
| trees.lox           |        337.839 s | 22.050 s |                 15.32 |
| zoo.lox             |         10.098 s |  3.208 s |                  3.15 |