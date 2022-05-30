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
| binary_trees.lox    |         20.887 s |  4.624 s |                  4.52 |
| equality.lox        |          9.516 s |  2.849 s |                  3.34 |
| fib.lox             |          9.577 s |  2.884 s |                  3.32 |
| instantiation.lox   |          3.288 s |  1.126 s |                  2.92 |
| invocation.lox      |          2.909 s |  1.034 s |                  2.81 |
| method_call.lox     |          2.811 s |  1.258 s |                  2.23 |
| properties.lox      |          6.081 s |  3.201 s |                  1.90 |
| string_equality.lox |          4.403 s |  2.273 s |                  1.94 |
| trees.lox           |         68.517 s | 22.050 s |                  3.11 |
| zoo.lox             |          4.286 s |  3.208 s |                  1.34 |