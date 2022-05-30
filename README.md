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
| binary_trees.lox    |          9.848 s |  4.624 s |                  2.13 |
| equality.lox        |          9.782 s |  2.849 s |                  3.43 |
| fib.lox             |          8.805 s |  2.884 s |                  3.05 |
| instantiation.lox   |          3.125 s |  1.126 s |                  2.78 |
| invocation.lox      |          2.739 s |  1.034 s |                  2.65 |
| method_call.lox     |          1.713 s |  1.258 s |                  1.36 |
| properties.lox      |          4.331 s |  3.201 s |                  1.35 |
| string_equality.lox |          4.741 s |  2.273 s |                  2.09 |
| trees.lox           |         19.602 s | 22.050 s |                  0.89 |
| zoo.lox             |          3.083 s |  3.208 s |                  0.96 |