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
| binary_trees.lox    |         39.305 s |  4.624 s |                  8.50 |
| equality.lox        |          9.928 s |  2.849 s |                  3.48 |
| fib.lox             |         10.213 s |  2.884 s |                  3.54 |
| instantiation.lox   |          3.865 s |  1.126 s |                  3.43 |
| invocation.lox      |          3.602 s |  1.034 s |                  3.48 |
| method_call.lox     |          3.875 s |  1.258 s |                  3.08 |
| properties.lox      |          7.640 s |  3.201 s |                  2.39 |
| string_equality.lox |          5.887 s |  2.273 s |                  2.59 |
| trees.lox           |        122.254 s | 22.050 s |                  5.54 |
| zoo.lox             |          5.160 s |  3.208 s |                  1.61 |