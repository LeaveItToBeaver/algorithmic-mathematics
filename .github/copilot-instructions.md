# amlang - Algorithmic Mathematics Language

## What This Project Is

**amlang** is an interpreter for Algorithmic Mathematics (AM), a language where **algorithms, conditionals, and recursion are first-class mathematical objects**. Instead of forcing closed-form solutions, AM embraces explicit case analysis using the `[ condition -> result ; ... ]` syntax.

The language uses **ASCII-first notation** that is directly typeable on any keyboard, with optional Unicode rendering for display/export. This ensures zero translation loss between handwritten mathematics and executable code.

This is a Rust-based lexer/parser/evaluator with module system support, REPL, and file execution.

## Architecture Overview

### Pipeline: Source → AST → Evaluation

1. **Lexer** (`src/lexer.rs`): Converts source (ASCII with optional Unicode) to tokens
2. **Parser** (`src/parser.rs`): Builds AST from tokens; uses recursive descent with `Tokens` wrapper
3. **Resolver** (`src/resolver.rs`): Handles multi-file modules, imports/exports, validates symbol visibility
4. **Evaluator** (`src/eval.rs`): Interprets AST with `World` (algorithm registry) and `Env` (variable bindings)
5. **Renderer** (`src/render.rs`): Converts ASCII source to Unicode for display (optional)

### Key Data Structures

**AST** (`src/ast.rs`):
- `Expr`: Core expression enum (Number, Bool, Ident, Call, Unary, Bin, Case, Pipe)
- `AlgorithmDef`: Named function with params (optionally typed), optional return type, optional spec, and body expression
- `Type`: Type annotations (Real, Natural, Integer, etc., with generics and vector spaces)
- `Specification`: Preconditions and postconditions for formal verification (future)
- `Module`: Top-level container with imports/exports/declarations
- `BinOp`/`UnOp`: Operator enums (Add, Mul, Pow, Eq, And, Or, etc.)

**Evaluation** (`src/eval.rs`):
- `Value`: Runtime values (Number/Bool only - f64 and bool)
- `Env`: Variable bindings map (includes built-in constants: `inf`, `NaN`, `pi`, `e`, `tau`)
- `World`: Algorithm registry + symbol table for qualified name resolution

**Rendering** (`src/render.rs`):
- `render_unicode()`: Converts ASCII source to Unicode for display/PDF export
- Handles type identifiers (R → ℝ), operators (:= → ≝, -> → →), superscripts (R^3 → ℝ³)

## Language Syntax (v2.0 - ASCII-First)

### Algorithm Definitions
```am
// Basic definition
@Add(a, b) := a + b

// With type annotations
@SafeDiv(a: R, b: R) -> R := [
  b != 0 -> a/b;
  b = 0 && a > 0 -> inf;
  b = 0 && a < 0 -> -inf;
  else -> NaN
]

// With specification (future)
@GCD(a: N, b: N) -> N
  requires: a > 0 || b > 0
  ensures: result | a && result | b
:= [
  b = 0 -> a;
  else -> @GCD(b, a mod b)
]
```

### Case Expressions (The Core Pattern)
```am
[ condition1 -> result1 ; condition2 -> result2 ; else -> default ]
```
- Square brackets `[ ]` denote case analysis (NOT arrays)
- Semicolons `;` separate arms
- `else` is the default/catch-all pattern (preferred over `_`)
- This is how ALL conditionals work in AM

### Type Annotations
```am
// Basic types (ASCII notation)
R     // Real numbers (ℝ when rendered)
N     // Natural numbers (ℕ when rendered)
Z     // Integers (ℤ when rendered)
Q     // Rationals (ℚ when rendered)
C     // Complex (ℂ when rendered)
Bool  // Boolean values

// Vector spaces
R^3       // 3D real space (ℝ³ when rendered)
R^n       // n-dimensional space (ℝⁿ when rendered)

// Generic types
Set<R>           // Set of reals
Vec<R, 3>        // 3-element vector
List<N>          // List of naturals
Option<R>        // Optional real
```

### Operators (ASCII → Rendered)
```am
:=    →  ≝   // "is defined as"
->    →  →   // arrow (types, case arms)
!=    →  ≠   // not equal
<=    →  ≤   // less than or equal
>=    →  ≥   // greater than or equal
&&    →  ∧   // logical AND
||    →  ∨   // logical OR
!     →  ¬   // logical NOT
```

### Module System
```am
module Nat
export S, P, Add
import Arith as A
use Arith.IsZero

@Add(a,b) := [ b = 0 -> a ; else -> S(@Add(a, P(b))) ]
```
- Modules resolve across files in search path (entry file's directory + configured paths)
- Exports control visibility; resolver validates all exported symbols exist
- Use statements import specific functions; imports bring whole modules with optional alias

### Calling Algorithms
- Algorithms can be called WITH `@` prefix: `@GCD(a,b)` 
- OR without if imported/built-in: `Add(x,y)`
- The `is_alg` flag in `Expr::Call` tracks this; evaluator handles both

### Recursion
Recursive calls MUST use `@` prefix when calling the algorithm from within itself:
```am
@Fact(n) := [ n = 0 -> 1 ; else -> n * @Fact(n - 1) ]
```

## CLI Usage Patterns

### Running Files
```bash
# Basic execution (evaluates module, prints nothing unless there's a call)
cargo run examples/Nat.am

# Execute with a specific call
cargo run examples/Nat.am --call "Add(2,3)"

# Print AST for debugging
cargo run examples/Gcd.am --ast --call "GCD(48,18)"
```

### REPL Commands
```bash
cargo run  # starts REPL
```
- `:help` - show commands
- `:list` - list defined algorithms  
- `:reset` - clear definitions
- `@Add(x,y) := x+y` - define algorithm
- `Add(2,3)` - evaluate expression

## Development Workflows

### Building and Testing
```bash
cargo build               # debug build
cargo build --release     # optimized
cargo test                # run tests (unit tests + smoke tests)
```

Tests use `CARGO_BIN_EXE_amlang` to invoke the compiled binary with example files.

### Adding Language Features
1. Update `Token` enum in `src/token.rs` for new keywords/operators
2. Modify lexer (`src/lexer.rs`) to recognize tokens
3. Extend `Expr` or add new AST node in `src/ast.rs`
4. Update parser (`src/parser.rs`) - usually in `parse_expr` or `parse_primary`
5. Implement evaluation logic in `src/eval.rs` (match on new `Expr` variant)
6. Add rendering logic in `src/render.rs` if Unicode representation differs
7. Add examples and tests

### Error Handling Pattern
- Lexer/parser use `panic!` with `caret_message()` from `src/token.rs` for nice error display
- Evaluator returns `Result<Value, String>`
- File processor wraps everything and prints user-friendly errors

## ASCII-First Philosophy

**Key Principle**: The canonical form is ASCII. Unicode rendering is purely presentational.

- Source files on disk are ASCII
- Parser reads ASCII tokens
- `render_unicode()` converts to Unicode for display/export
- Version control diffs show ASCII (no Unicode conflicts)
- Zero translation loss between typing and execution

**Example**:
```am
// What you TYPE (canonical):
@SafeDiv(a: R, b: R) -> R := [ b != 0 -> a/b ; else -> inf ]

// What you SEE (when rendered):
@SafeDiv(a: ℝ, b: ℝ) → ℝ ≝ [ b ≠ 0 → a/b ; else → ∞ ]
```

Both representations are semantically identical - the ASCII form is the source of truth.

## Common Pitfalls

### Case Expression Syntax
- Current parser expects: `[ cond -> val ; else -> default ]`
- NOT `case x of ... end` (old syntax, removed)
- NOT `[ cond ? val ; _ ? default ]` (old syntax, updated to use `->` and `else`)

### Algorithm Definitions
- Use `:=` for definitions, not `=` (though `=` still works for backwards compatibility)
- Use `->` for arrows, not `?`
- Use `else` for default case, not `_` (though `_` still works)

### Module Resolution
- Entry file directory is auto-added to search paths
- Imports must match file names (case-sensitive)
- Files without `module` declarations will be treated as part of the default module
- `include "path"` is for embedding raw code; not same as `import`

### Type Annotations
- Types are optional - parser accepts both `@Add(a,b)` and `@Add(a: R, b: R)`
- Type checking is NOT yet implemented - types are parsed but not enforced
- Future: types will enable formal verification and proof checking

## File Organization

```
src/
  main.rs           - CLI entry: file mode vs REPL
  lexer.rs          - Tokenization with ASCII/Unicode support
  token.rs          - Token definitions + error formatting
  parser.rs         - Recursive descent parser
  ast.rs            - All AST node types (Expr, Type, AlgorithmDef, Module)
  eval.rs           - Interpreter (Value, Env, World)
  resolver.rs       - Multi-file module loader + validator
  normalize.rs      - Unicode → ASCII conversion (input normalization)
  render.rs         - ASCII → Unicode conversion (output rendering)
  file_processor.rs - Orchestrates lex→parse→eval for files
  repl.rs           - Interactive mode with rustyline
  error_handling.rs - Pretty error wrappers

examples/          - Working AM programs (ground truth for syntax)
tests/smoke.rs     - Integration tests via CLI invocation
```

## What to Check First

When debugging:
1. **Syntax errors**: Check `src/parser.rs` - parser expects exact token sequences
2. **Evaluation errors**: Look at `src/eval.rs` - check `Env` for var lookups, `World` for algorithm defs
3. **Module not found**: Verify search paths in `src/resolver.rs` and file naming
4. **Unexpected case behavior**: Remember `else` is the default; arms are evaluated top-to-bottom
5. **Rendering issues**: Check `src/render.rs` for ASCII→Unicode mappings

## Current Limitations

- Only `f64` numbers and `bool` values (no strings, lists, or custom types yet - though AST supports them)
- Type checking is parsed but not enforced (all errors are runtime)
- Specifications (requires/ensures) are parsed but not verified
- Single-threaded evaluation (no parallelism)
- Built-in constants (`pi`, `e`, `inf`, `NaN`, `tau`) are hardcoded in `Env::base()`
- No standard library beyond what examples define

## Future Roadmap

1. **Type System**: Enforce type annotations at runtime/compile-time
2. **Proof Checking**: Implement specification verification
3. **Standard Library**: Build foundation modules (Nat, Bool, algebra, calculus, etc.)
4. **Self-Hosting**: Write compiler in AM itself
5. **IDE Support**: Syntax highlighting, auto-complete, real-time Unicode rendering

## When Adding Tests

Use the pattern from `tests/smoke.rs`:
```rust
fn run(file: &str, call: &str) -> String {
    Command::new(env!("CARGO_BIN_EXE_amlang"))
        .args([file, "--call", call])
        .output()
        .expect("run failed")
    // ... parse stdout
}
```

This tests the full pipeline including CLI arg parsing.
