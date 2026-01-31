# AM Language Notation (v2.0)

**Philosophy**: AM uses **ASCII-first notation** that is directly typeable on any keyboard, with optional Unicode rendering in IDEs and PDFs. This ensures zero translation loss between handwritten mathematics and executable code.

---

## Design Principles

1. **Canonical form is ASCII** — All source files use standard ASCII characters
2. **Visual rendering is Unicode** — IDEs/exporters display beautiful mathematical notation
3. **Zero ambiguity** — Every ASCII sequence has exactly one meaning
4. **Keyboard-friendly** — No hunting for Unicode characters while typing

---

## 1. Algorithm Definitions

### Syntax
```
@Name(param1, param2, ...) := expression
@Name(param1: Type1, param2: Type2) -> ReturnType := expression
```

### ASCII Canonical Form
```am
@Add(a, b) := a + b

@SafeDiv(a: R, b: R) -> R := [
  b != 0 -> a/b;
  b = 0 && a > 0 -> inf;
  b = 0 && a < 0 -> -inf;
  else -> NaN
]
```

### Rendered Display (IDE/PDF)
```am
@Add(a, b) ≝ a + b

@SafeDiv(a: ℝ, b: ℝ) → ℝ ≝ [
  b ≠ 0 → a/b;
  b = 0 ∧ a > 0 → ∞;
  b = 0 ∧ a < 0 → −∞;
  else → NaN
]
```

---

## 2. Type Annotations

### Basic Types (ASCII → Rendered)

| ASCII | Rendered | Description |
|-------|----------|-------------|
| `R` | `ℝ` | Real numbers |
| `N` | `ℕ` | Natural numbers (0,1,2,...) |
| `Z` | `ℤ` | Integers |
| `Q` | `ℚ` | Rational numbers |
| `C` | `ℂ` | Complex numbers |
| `Bool` | `𝔹` | Boolean values |

### Vector Spaces
```am
// ASCII
R^3              // 3D real vector space
R^n              // n-dimensional real space
Matrix<R, 3, 3>  // 3×3 real matrix

// Rendered
ℝ³
ℝⁿ
Mat₃ₓ₃(ℝ)
```

### Collections
```am
// ASCII
Set<R>           // Set of reals
Vec<R, 3>        // 3-element vector of reals
List<N>          // List of naturals
Option<R>        // Optional real value

// Rendered
Set⟨ℝ⟩
Vec⟨ℝ, 3⟩
List⟨ℕ⟩
Option⟨ℝ⟩
```

---

## 3. Case Analysis (Core Pattern)

### Syntax
```
[
  condition1 -> result1;
  condition2 -> result2;
  ...
  else -> default
]
```

### Example
```am
@Factorial(n: N) -> N := [
  n = 0 -> 1;
  else -> n * @Factorial(n - 1)
]
```

**Note**: The `else` keyword is preferred over `_` for clarity in mathematical contexts.

---

## 4. Operators

### ASCII → Unicode Rendering

| ASCII | Rendered | Meaning |
|-------|----------|---------|
| `:=` | `≝` | "is defined as" |
| `->` | `→` | arrow (types, cases) |
| `!=` | `≠` | not equal |
| `<=` | `≤` | less than or equal |
| `>=` | `≥` | greater than or equal |
| `&&` | `∧` | logical AND |
| `\|\|` | `∨` | logical OR |
| `!` | `¬` | logical NOT |
| `*` | `×` | multiplication (contextual) |
| `.` | `⋅` | dot product (contextual) |

### Arithmetic
```am
+ - * / ^           // Standard operators
sqrt(x)             // Square root (rendered as √x)
abs(x)              // Absolute value
```

### Equality and Comparison
```am
=                   // Equality (in conditions)
!=                  // Not equal (rendered as ≠)
< <= > >=           // Comparisons
```

### Logical
```am
&&                  // AND (rendered as ∧)
||                  // OR (rendered as ∨)
!                   // NOT (rendered as ¬)
```

---

## 5. Constants and Literals

### Numbers
```am
42                  // Integer
3.14                // Float
1/2                 // Rational (future feature)
```

### Special Constants (ASCII → Rendered)

| ASCII | Rendered | Value |
|-------|----------|-------|
| `inf` | `∞` | Positive infinity |
| `-inf` | `−∞` | Negative infinity |
| `NaN` | `NaN` | Not a Number |
| `pi` | `π` | π ≈ 3.14159... |
| `e` | `e` | Euler's number |
| `tau` | `τ` | τ = 2π |

### Booleans
```am
true
false
```

---

## 6. Specifications (Optional)

Specifications enable formal verification and proof checking.

### Syntax
```am
@Name(params) -> Type
  requires: precondition
  ensures: postcondition
:= implementation
```

### Example
```am
@GCD(a: N, b: N) -> N
  requires: a > 0 || b > 0
  ensures: result | a && result | b
:= [
  b = 0 -> a;
  else -> @GCD(b, a mod b)
]
```

### Quantifiers (ASCII → Rendered)

| ASCII | Rendered | Meaning |
|-------|----------|---------|
| `forall` | `∀` | universal quantifier |
| `exists` | `∃` | existential quantifier |
| `in` | `∈` | element of |
| `subset` | `⊆` | subset of |

---

## 7. Let Bindings

```am
let x = expression1 in
let y = expression2 in
result_expression
```

### Example
```am
@Quadratic(a: R, b: R, c: R) -> Set<R> := 
  let Delta = b^2 - 4*a*c in [
    Delta > 0 -> {(-b + sqrt(Delta))/(2*a), (-b - sqrt(Delta))/(2*a)};
    Delta = 0 -> {-b/(2*a)};
    else -> {}
  ]
```

---

## 8. Modules and Imports

### Module Declaration
```am
module ModuleName
export Func1, Func2
import OtherModule as M
use ThirdModule.SpecificFunc
```

### Example
```am
module Nat
export Zero, Succ, Add

@Zero := 0
@Succ(n: N) -> N := n + 1
@Add(a: N, b: N) -> N := [
  b = 0 -> a;
  else -> @Add(@Succ(a), b - 1)
]
```

---

## 9. Complete Example

### Safe Division with All Features

```am
module SafeMath
export SafeDiv

@SafeDiv(a: R, b: R) -> R
  requires: true  // Always callable
  ensures: (b != 0 -> result = a/b) && 
           (b = 0 && a > 0 -> result = inf) &&
           (b = 0 && a < 0 -> result = -inf) &&
           (b = 0 && a = 0 -> isNaN(result))
:= [
  b != 0 -> a/b;
  b = 0 && a > 0 -> inf;
  b = 0 && a < 0 -> -inf;
  else -> NaN
]
```

### How It Renders

When displayed in an IDE or exported to PDF:

```am
module SafeMath
export SafeDiv

@SafeDiv(a: ℝ, b: ℝ) → ℝ
  requires: true
  ensures: (b ≠ 0 → result = a/b) ∧ 
           (b = 0 ∧ a > 0 → result = ∞) ∧
           (b = 0 ∧ a < 0 → result = −∞) ∧
           (b = 0 ∧ a = 0 → isNaN(result))
≝ [
  b ≠ 0 → a/b;
  b = 0 ∧ a > 0 → ∞;
  b = 0 ∧ a < 0 → −∞;
  else → NaN
]
```

---

## 10. Formal Grammar

```
Module      ::= ModuleHeader? ImportOrExport* TopLevelDecl*
ModuleHeader::= 'module' Ident NL
ImportOrExport ::= ImportStmt | UseStmt | ExportStmt

ImportStmt  ::= 'import' ModPath ('as' Ident)? NL
UseStmt     ::= 'use' UseList NL
UseList     ::= UseItem (',' UseItem)*
UseItem     ::= ModPath ('.' '*')? | ModPath '.' Ident

ExportStmt  ::= 'export' ExportList NL
ExportList  ::= Ident (',' Ident)*

TopLevelDecl::= AlgDef | Include
AlgDef      ::= '@' Ident '(' TypedParams? ')' ReturnType? Spec? ':=' Expr NL
TypedParams ::= Param (',' Param)*
Param       ::= Ident (':' Type)?
ReturnType  ::= '->' Type
Type        ::= 'R' | 'N' | 'Z' | 'Q' | 'C' | 'Bool'
              | Ident ('^' Number)?
              | Ident '<' Type (',' Type)* '>'

Spec        ::= 'requires' ':' Expr NL 'ensures' ':' Expr NL

Expr        ::= Number | Bool | Ident | String
              | '(' Expr ')'
              | Expr BinOp Expr
              | UnOp Expr
              | Expr '^' Expr
              | Ident '(' Args? ')'
              | '@' Ident '(' Args? ')'
              | '[' CaseArms ']'
              | 'let' Ident '=' Expr 'in' Expr
              | '{' SetElems? '}'
              | Expr '>>' Expr

CaseArms    ::= CaseArm (';' CaseArm)* (';')?
CaseArm     ::= Expr '->' Expr | 'else' '->' Expr

BinOp       ::= '+' | '-' | '*' | '/' | '=' | '!=' | '<' | '<=' | '>' | '>=' 
              | '&&' | '||' | 'mod' | '|'

UnOp        ::= '-' | '!'

Include     ::= 'include' StringLiteral NL
```

---

## 11. Rendering Implementation

For IDE/tooling developers:

### ASCII → Unicode Mapping Table

```typescript
const renderMap: Record<string, string> = {
  // Definitions and arrows
  ':=': '≝',
  '->': '→',
  
  // Comparisons
  '!=': '≠',
  '<=': '≤',
  '>=': '≥',
  
  // Logic
  '&&': '∧',
  '||': '∨',
  
  // Types (in type context)
  'R': 'ℝ',
  'N': 'ℕ',
  'Z': 'ℤ',
  'Q': 'ℚ',
  'C': 'ℂ',
  
  // Constants
  'inf': '∞',
  'pi': 'π',
  'tau': 'τ',
  
  // Quantifiers
  'forall': '∀',
  'exists': '∃',
  'in': '∈',
  'subset': '⊆',
  
  // Functions (when rendering)
  'sqrt': '√',
};

// Superscript digits for R^3 -> ℝ³
const superscripts: Record<string, string> = {
  '0': '⁰', '1': '¹', '2': '²', '3': '³', '4': '⁴',
  '5': '⁵', '6': '⁶', '7': '⁷', '8': '⁸', '9': '⁹',
};
```

---

## 12. Migration from Old Syntax

| Old Syntax | New ASCII Syntax | Notes |
|------------|------------------|-------|
| `⟦a:ℝ⟧` | `(a: R)` | Use parentheses, not brackets |
| `≝` | `:=` | ASCII definition |
| `→` | `->` | ASCII arrow |
| `_` | `else` | More explicit |
| `∧` | `&&` | Standard logical AND |
| `∨` | `\|\|` | Standard logical OR |
| `≠` | `!=` | Standard not-equal |

---

## Summary

**Write in ASCII. See in Unicode. Execute without translation loss.**

This notation enables:
- ✅ Direct typing on any keyboard
- ✅ Beautiful rendering in IDEs and papers
- ✅ Version control friendly (plain ASCII diffs)
- ✅ Zero semantic ambiguity
- ✅ Seamless paper ↔ code workflow

ModPath     ::= Ident ('.' Ident)*
Ident       ::= [A-Za-z_…Unicode…][A-Za-z0-9_…]*

NL          ::= newline or semicolon (you already support line-end)
