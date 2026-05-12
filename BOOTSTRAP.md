# Bootstrap

The Suru compiler is self-hosted: `compiler/build.suru` is the compiler source,
and `bin/suru-build` is the compiled bootstrap binary used to rebuild it.

## Origin

`bin/suru-build` was compiled from the C# bootstrap compiler and released as
`v0.1.0` in the frozen bootstrap repository:

  https://github.com/paul-ciorogar/suru-lang-bootstrap/releases/tag/v0.1.0

The C# source is archived there and will not receive further changes.

## Rebuilding the compiler

Use `bin/suru-build` to compile `compiler/build.suru` into a new binary:

```sh
./bin/suru-build compiler/build.suru /tmp/build.ll
clang-18 /tmp/build.ll runtime/*.ll -o bin/suru-build-new
```

`scripts/bootstrap.sh` automates this process.
