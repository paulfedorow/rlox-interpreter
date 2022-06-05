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
| binary_trees.lox    |          6.679 s |  4.624 s |                  1.44 |
| equality.lox        |          7.357 s |  2.849 s |                  2.58 |
| fib.lox             |          6.133 s |  2.884 s |                  2.13 |
| instantiation.lox   |          2.312 s |  1.126 s |                  2.05 |
| invocation.lox      |          2.146 s |  1.034 s |                  2.08 |
| method_call.lox     |          1.150 s |  1.258 s |                  0.91 |
| properties.lox      |          3.011 s |  3.201 s |                  0.94 |
| string_equality.lox |          2.656 s |  2.273 s |                  1.17 |
| trees.lox           |         15.053 s | 22.050 s |                  0.68 |
| zoo.lox             |          2.138 s |  3.208 s |                  0.67 |

You can find the benchmark scripts [here](resources/benchmark).