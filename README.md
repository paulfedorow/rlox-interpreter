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
| binary_trees.lox    |         40.559 s |  4.624 s |                  8.77 |
| equality.lox        |          9.991 s |  2.849 s |                  3.51 |
| fib.lox             |         11.374 s |  2.884 s |                  3.94 |
| instantiation.lox   |          4.574 s |  1.126 s |                  4.06 |
| invocation.lox      |          3.872 s |  1.034 s |                  3.74 |
| method_call.lox     |          4.106 s |  1.258 s |                  3.26 |
| properties.lox      |          8.153 s |  3.201 s |                  2.55 |
| string_equality.lox |          6.129 s |  2.273 s |                  2.70 |
| trees.lox           |        129.219 s | 22.050 s |                  5.86 |
| zoo.lox             |          5.434 s |  3.208 s |                  1.69 |