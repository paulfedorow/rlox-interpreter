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
| binary_trees.lox    |          6.115 s |  4.624 s |                  1.32 |
| equality.lox        |          7.230 s |  2.849 s |                  2.54 |
| fib.lox             |          5.743 s |  2.884 s |                  1.99 |
| instantiation.lox   |          2.220 s |  1.126 s |                  1.97 |
| invocation.lox      |          2.056 s |  1.034 s |                  1.99 |
| method_call.lox     |          1.025 s |  1.258 s |                  0.81 |
| properties.lox      |          2.813 s |  3.201 s |                  0.88 |
| string_equality.lox |          2.208 s |  2.273 s |                  0.97 |
| trees.lox           |         14.571 s | 22.050 s |                  0.66 |
| zoo.lox             |          2.004 s |  3.208 s |                  0.62 |

You can find the benchmark scripts [here](resources/benchmark).