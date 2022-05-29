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
| binary_trees.lox    |         39.161 s |  4.624 s |                  8.47 |
| equality.lox        |         10.129 s |  2.849 s |                  3.56 |
| fib.lox             |         11.309 s |  2.884 s |                  3.92 |
| instantiation.lox   |          4.689 s |  1.126 s |                  4.16 |
| invocation.lox      |          3.860 s |  1.034 s |                  3.73 |
| method_call.lox     |          4.238 s |  1.258 s |                  3.37 |
| properties.lox      |          8.133 s |  3.201 s |                  2.54 |
| string_equality.lox |          7.149 s |  2.273 s |                  3.15 |
| trees.lox           |        129.540 s | 22.050 s |                  5.87 |
| zoo.lox             |          5.551 s |  3.208 s |                  1.73 |