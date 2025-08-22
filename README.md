# amlang — Algorithmic Mathematics

Algorithmic Mathematics (AM) treats **algorithms and conditional logic as first‑class mathematical objects**. Instead of hunting for a single closed‑form formula, AM lets you write an *adaptive* specification: explicit case analysis, recursion, and composition are part of the mathematics, not afterthoughts.

This repository contains a working **parser** and **execution engine** plus a small **examples** folder and an initial **tests** suite.

---

## Project Layout

```
src/        # Core crate: lexer/parser, AST, and execution engine
examples/   # AM programs demonstrating the language
tests/      # Initial smoke test(s) for the engine
Cargo.toml  # Package manifest
```

> Exact module names may evolve as the language stabilizes, but the public entrypoint and engine are usable today.

---

## Getting Started

### Prerequisites
- Rust (stable) with Cargo — install via https://rustup.rs
- macOS, Linux, or Windows

### Build
```bash
git clone https://github.com/LeaveItToBeaver/algorithmic-mathematics.git
cd algorithmic-mathematics
cargo build
# or for an optimized binary
cargo build --release
```

### Run
The CLI runs AM source files from the `examples/` directory or any path you provide.
```bash
# Print CLI usage
cargo run -- --help

# Run an example (replace with an actual file from examples/)
cargo run -- examples/<example-file>

# Using the release build
./target/release/algorithmic-mathematics examples/<example-file>
```

> If your CLI accepts flags (e.g., `--trace`, `--pretty`, `--json`), they can be passed after `--`. Use `--help` to see the authoritative options supported by the current build.

### Test
There is currently **one** test in `tests/`:
```bash
cargo test
```

---

## What Works Today

- **Parsing** of the core AM syntax used in `examples/`
- **Execution** of parsed programs (conditionals, recursion, and composition patterns present in the examples)
- **CLI** that evaluates a file and prints results (formatting depends on flags/help)

> The examples directory is the ground truth for supported syntax and features. If it runs there, it’s supported by the current engine.

---

## Roadmap

Short‑term
- Expand the standard library of built‑in algorithmic combinators
- Better diagnostics (error spans, pretty printing, traces)
- More tests covering recursion, branching, and failure modes
- Clearer result types (numeric, set‑like, complex) and conversions

Medium‑term
- Imports/modules for multi‑file AM programs
- Deterministic evaluation traces for debugging and pedagogy
- Benchmarks and profiling to guide engine optimizations
- Optional JSON output mode for tooling integration

Long‑term
- Typed algorithmic objects and contracts (pre/postconditions)
- Interop with conventional math libraries and external solvers
- REPL and web playground

---

## Design Philosophy (brief)

- **Explicit over implicit** — decisions and branches should be visible in the math.
- **Composable** — small pieces combine into larger methods (sequential, parallel, conditional).
- **Paper‑first** — everything should be writable and reasoned about on paper, then executable by the engine.

---

## Contributing

Issues and PRs are welcome. Useful contributions include:
- Additional examples in `examples/`
- Tests in `tests/` that capture edge‑cases and semantics
- Improvements to parser/engine ergonomics and error messages

---

## License

MIT
