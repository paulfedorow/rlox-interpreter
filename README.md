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
| binary_trees.lox    |         20.351 s |  4.624 s |                  4.40 |
| equality.lox        |          9.636 s |  2.849 s |                  3.38 |
| fib.lox             |          8.850 s |  2.884 s |                  3.07 |
| instantiation.lox   |          3.226 s |  1.126 s |                  2.87 |
| invocation.lox      |          2.938 s |  1.034 s |                  2.84 |
| method_call.lox     |          2.634 s |  1.258 s |                  2.09 |
| properties.lox      |          5.312 s |  3.201 s |                  1.66 |
| string_equality.lox |          4.637 s |  2.273 s |                  2.04 |
| trees.lox           |         65.593 s | 22.050 s |                  2.97 |
| zoo.lox             |          3.850 s |  3.208 s |                  1.20 |