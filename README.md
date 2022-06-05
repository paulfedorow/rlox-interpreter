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

    target/release/rlox-interpreter resources/benchmark/fib.lox

## Benchmarks

rlox-interpreter is implemented as an AST-walking interpreter. Here are some execution times from running each of the
benchmarks on an AMD Ryzen 7 Pro 5850U. For comparison there are also the execution times for Bob Nystroms AST-walking
interpreter (which is programmed in Java).

|                     | rlox-interpreter | jlox     | rlox-interpreter/jlox |
|---------------------|------------------|----------|-----------------------|
| binary_trees.lox    |          6.495 s |  4.624 s |                  1.40 |
| equality.lox        |          8.962 s |  2.849 s |                  3.15 |
| fib.lox             |          6.147 s |  2.884 s |                  2.13 |
| instantiation.lox   |          2.352 s |  1.126 s |                  2.09 |
| invocation.lox      |          2.084 s |  1.034 s |                  2.02 |
| method_call.lox     |          1.134 s |  1.258 s |                  0.90 |
| properties.lox      |          2.912 s |  3.201 s |                  0.91 |
| string_equality.lox |          3.216 s |  2.273 s |                  1.41 |
| trees.lox           |         15.090 s | 22.050 s |                  0.68 |
| zoo.lox             |          2.108 s |  3.208 s |                  0.66 |

You can find the benchmark scripts [here](resources/benchmark).