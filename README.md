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
| binary_trees.lox    |         39.074 s |  4.624 s |                  8.45 |
| equality.lox        |          9.602 s |  2.849 s |                  3.37 |
| fib.lox             |         10.743 s |  2.884 s |                  3.73 |
| instantiation.lox   |          4.067 s |  1.126 s |                  3.61 |
| invocation.lox      |          3.942 s |  1.034 s |                  3.81 |
| method_call.lox     |          4.095 s |  1.258 s |                  3.26 |
| properties.lox      |          7.887 s |  3.201 s |                  2.46 |
| string_equality.lox |          7.143 s |  2.273 s |                  3.14 |
| trees.lox           |        128.864 s | 22.050 s |                  5.84 |
| zoo.lox             |          5.343 s |  3.208 s |                  1.67 |