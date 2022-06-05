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
| binary_trees.lox    |          9.894 s |  4.624 s |                  2.14 |
| equality.lox        |          9.453 s |  2.849 s |                  3.32 |
| fib.lox             |          8.720 s |  2.884 s |                  3.02 |
| instantiation.lox   |          3.042 s |  1.126 s |                  2.70 |
| invocation.lox      |          2.703 s |  1.034 s |                  2.61 |
| method_call.lox     |          1.723 s |  1.258 s |                  1.37 |
| properties.lox      |          4.253 s |  3.201 s |                  1.33 |
| string_equality.lox |          4.699 s |  2.273 s |                  2.07 |
| trees.lox           |         20.150 s | 22.050 s |                  0.91 |
| zoo.lox             |          3.099 s |  3.208 s |                  0.97 |

You can find the benchmark scripts [here](resources/benchmark).