# rlox-interpreter

rlox-interpreter is an AST-walking implementation of Bob Nystrom's [Lox](https://craftinginterpreters.com/) language in Rust.

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
benchmarks on an AMD Ryzen 7 Pro 5850U. For comparison there are also the execution times for Bob Nystrom's AST-walking
interpreter (which is programmed in Java).

|                     | rlox-interpreter | jlox     | rlox-interpreter/jlox |
|---------------------|------------------|----------|-----------------------|
| binary_trees.lox    |          6.129 s |  4.624 s |                  1.33 |
| equality.lox        |          7.045 s |  2.849 s |                  2.47 |
| fib.lox             |          5.675 s |  2.884 s |                  1.97 |
| instantiation.lox   |          2.219 s |  1.126 s |                  1.97 |
| invocation.lox      |          2.052 s |  1.034 s |                  1.98 |
| method_call.lox     |          1.016 s |  1.258 s |                  0.81 |
| properties.lox      |          2.799 s |  3.201 s |                  0.87 |
| string_equality.lox |          2.185 s |  2.273 s |                  0.96 |
| trees.lox           |         14.073 s | 22.050 s |                  0.64 |
| zoo.lox             |          1.962 s |  3.208 s |                  0.61 |

You can find the benchmark scripts [here](resources/benchmark).
