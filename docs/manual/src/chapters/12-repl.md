# The REPL

## What is the REPL?

The REPL (Read-Eval-Print Loop) is an interactive environment for experimenting with Kleis:

```bash
$ cargo run --bin repl

🧮 Kleis REPL v1.0.0
   Type :help for commands, :quit to exit

λ>
```

## Basic Usage

Enter expressions to evaluate them symbolically:

```
λ> 2 + 2
2 + 2

λ> let x = 5 in x * x
times(5, 5)

λ> sin(π / 2)
sin(divide(π, 2))
```

> **Note:** The REPL performs **symbolic evaluation**, not numeric computation. Expressions are simplified symbolically, not calculated to numbers.

## Loading Files

The REPL prompt evaluates expressions. For definitions (`define`, `structure`, etc.), use `:load`:

```
λ> :load examples/protocols/stop_and_wait.kleis
✅ Loaded: 1 files, 5 functions, 0 structures, 0 data types, 0 type aliases

λ> :env
📋 Defined functions:
  next_seq (seq) = ...
  valid_ack (sent, ack) = ...
  sender_next_state (current_seq, ack_received) = ...
  receiver_accepts (expected, received) = ...
  receiver_next_state (expected, received) = ...
```

More examples to load:

```
λ> :load examples/business/order_to_cash.kleis
✅ Loaded: 1 files, 21 functions, 0 structures, 4 data types, 0 type aliases

λ> :load examples/authorization/zanzibar.kleis
✅ Loaded: 1 files, 13 functions, 0 structures, 0 data types, 0 type aliases
```

## The `import` Keyword

In Kleis source files, use `import` to include definitions from other files:

```kleis
import "stdlib/prelude.kleis"
import "stdlib/matrices.kleis"

// Now you can use definitions from those files
define my_matrix = identity(3)
```

**Import syntax:**
- `import "path/to/file.kleis"` — includes all definitions from that file

Imports are processed at parse time. Relative paths are resolved from the importing file's directory.

**Common imports:**

```kleis
import "stdlib/prelude.kleis"     // Basic types and operations
import "stdlib/matrices.kleis"    // Matrix operations
import "stdlib/complex.kleis"     // Complex number support
```

### Standard Library Resolution

Imports starting with `stdlib/` are handled specially:

1. **`KLEIS_ROOT` environment variable** — If set, Kleis looks for `$KLEIS_ROOT/stdlib/...` first
2. **Project directory** — Kleis walks up from the current file looking for a `stdlib/` folder
3. **Current working directory** — Falls back to `./stdlib/...`

**Setting `KLEIS_ROOT`:**

```bash
# Add to your shell profile (~/.bashrc, ~/.zshrc, etc.)
export KLEIS_ROOT=/path/to/kleis

# Now stdlib imports work from anywhere
kleis run my_project/main.kleis
```

This is useful when:
- Working on projects outside the Kleis repository
- Running Kleis from arbitrary directories
- Sharing code that uses the standard library

> **Note:** In the REPL, use `:load` instead of `import`. The `:load` command loads a file interactively, while `import` is for use inside `.kleis` source files.

## Verification with Z3

Run verifications interactively with `:verify`:

```
λ> :verify x + y = y + x
✅ Valid

λ> :verify x > 0
❌ Invalid - Counterexample: x!2 -> 0
```

## Satisfiability with Z3

Use `:sat` to find solutions (equation solving):

```
λ> :sat ∃(z : ℂ). z * z = complex(-1, 0)
✅ Satisfiable
   Witness: z_re = 0, z_im = -1

λ> :sat ∃(x : ℝ). x * x = 4
✅ Satisfiable
   Witness: x = -2

λ> :sat ∃(x : ℝ). x * x = -1
❌ Unsatisfiable (no solution exists)

λ> :sat ∃(x : ℝ)(y : ℝ). x + y = 10 ∧ x - y = 4
✅ Satisfiable
   Witness: x = 7, y = 3
```

**`:verify` vs `:sat`:**

| Command | Question | Use Case |
|---------|----------|----------|
| `:verify` | Is it always true? (∀) | Prove theorems |
| `:sat` | Does a solution exist? (∃) | Solve equations |

## Lambda Expressions

Lambda expressions work at the prompt:

```
λ> λ x . x * 2
λ x . times(x, 2)

λ> λ x y . x + y
λ x y . x + y
```

## Type Inference

Check types with `:type`:

```
λ> :type 42
📐 Type: Int

λ> :type 3.14
📐 Type: Scalar

λ> :type sin
📐 Type: α0
```

> **Note:** Integer literals (`42`) type as `Int`, real literals (`3.14`) type as `Scalar`. This enables proper type promotion (e.g., `Int + Rational → Rational`).

## Concrete Evaluation with `:eval`

The `:eval` command performs **concrete evaluation** — it actually computes results, including recursive functions:

```
λ> :load examples/meta-programming/lisp_parser.kleis
✅ Loaded: 60 functions

λ> :eval run("(+ 2 3)")
VNum(5)

λ> :eval run("(letrec ((fact (lambda (n) (if (<= n 1) 1 (* n (fact (- n 1))))))) (fact 5))")
VNum(120)
```

**`:eval` vs `:sat` vs `:verify`:**

| Command | Execution | Handles Recursion | Use Case |
|---------|-----------|-------------------|----------|
| `:eval` | **Concrete** (Rust) | ✅ Yes | Compute actual values |
| `:sat` | Symbolic (Z3) | ❌ No (may timeout) | Find solutions |
| `:verify` | Symbolic (Z3) | ❌ No (may timeout) | Prove theorems |

> **Key insight:** Z3 cannot symbolically unroll recursive functions over unbounded data types. Use `:eval` for concrete computation, `:sat`/`:verify` for symbolic reasoning.

This is what makes Kleis **Turing complete** — the combination of ADTs, pattern matching, recursion, and concrete evaluation enables arbitrary computation. See [Appendix: LISP Interpreter](../appendix/lisp-interpreter.md) for a complete example.

## The Verification Gap (Important!)

**Users must understand this fundamental limitation.**

The three REPL modes operate on **different systems**:

| Command | Executes On | Axiom Checking |
|---------|-------------|----------------|
| `:eval` | Rust builtins / pattern matching | ❌ None |
| `:verify` | Z3's mathematical model | ✅ Symbolic |
| `:sat` | Z3's mathematical model | ✅ Symbolic |

**The gap:**

When you run `:verify ∀(a b : ℕ). a + b = b + a`, Z3 proves this using its built-in integer arithmetic theory.

When you run `:eval 2 + 3`, Rust's `+` operator computes `5`.

**We never verify that Rust's `+` matches Z3's `+`.**

**The Trusted Computing Base:**

These components are assumed correct, never verified:
- Rust compiler
- Builtin implementations (`builtin_add`, `builtin_mul`, etc.)
- LAPACK (for matrix operations)
- IEEE 754 floating point

**What Kleis provides:**

| Capability | Provided? |
|------------|-----------|
| Verify mathematical properties symbolically | ✅ Yes |
| Compute concrete results efficiently | ✅ Yes |
| Prove computation matches specification | ❌ No |

**Example:**

```kleis
structure AdditiveMonoid(M) {
    operation add : M → M → M
    axiom add_comm: ∀(a b : M). add(a, b) = add(b, a)
}

implements AdditiveMonoid(ℕ) {
    operation add = builtin_add  // Rust's + operator
}
```

- `:verify add_comm` → Z3 checks its integer model ✅
- `:eval 2 + 3` → Rust's `builtin_add` runs ✅
- Connection between them → **Trusted, not verified** ⚠️

This is the pragmatic trade-off Kleis makes: trust the implementation, verify the mathematics.

## Value Bindings with `:let`

Use `:let` to bind values to names that persist across REPL commands:

```
λ> :let x = 2 + 3
x = 5

λ> :eval x * 2
✅ 10

λ> :let matrix = Matrix(2, 2, [1, 2, 3, 4])
matrix = Matrix(2, 2, [1, 2, 3, 4])

λ> :eval det(matrix)
✅ -2
```

**`:let` vs `:define`:**

| Command | Creates | Persistence | Use Case |
|---------|---------|-------------|----------|
| `:let x = expr` | **Value binding** | REPL session | Store computed values |
| `:define f(x) = expr` | **Function** | REPL session | Define reusable functions |

## The `it` Magic Variable

After each `:eval`, the result is stored in `it` for quick chaining:

```
λ> :eval 2 + 3
✅ 5

λ> :eval it * 2
✅ 10

λ> :eval it + 1
✅ 11
```

This is inspired by GHCi and OCaml REPLs. Use `:env` to see all bindings including `it`:

```
λ> :env
📌 Value bindings:
  x = 5

📍 Last result (it):
  it = 11

📋 Defined functions:
  double (x) = ...
```

## REPL Commands

| Command | Description |
|---------|-------------|
| `:help` | Show all commands |
| `:load <file>` | Load a .kleis file |
| `:env` | Show defined functions and bindings |
| `:eval <expr>` | **Concrete evaluation** (computes actual values) |
| `:let x = <expr>` | Bind value to variable (persists in session) |
| `:define f(x) = <expr>` | Define a function |
| `:verify <expr>` | Verify with Z3 (is it always true?) |
| `:sat <expr>` | Check satisfiability (does a solution exist?) |
| `:type <expr>` | Show inferred type |
| `:ast <expr>` | Show parsed AST |
| `:symbols` | Unicode math symbols palette |
| `:syntax` | Complete syntax reference |
| `:examples` | Show example expressions |
| `:quit` | Exit REPL |

> **Tip:** Use `it` in any expression to refer to the last `:eval` result.

## Multi-line Input

For complex expressions, end lines with `\` or use block mode:

```
λ> :verify ∀(a : R, b : R). \
   (a + b) * (a - b) = a * a - b * b
✅ Valid
```

Or use `:{ ... :}` for blocks:

```
λ> :{
   :verify ∀(x : R, y : R, z : R).
     (x + y) + z = x + (y + z)
   :}
✅ Valid
```

## Example Session

```
λ> :load examples/authorization/zanzibar.kleis
✅ Loaded: 1 files, 13 functions, 0 structures, 0 data types, 0 type aliases

λ> :env
📋 Defined functions:
  can_share (perm) = ...
  can_edit (perm) = ...
  can_delete (perm) = ...
  effective_permission (direct, group) = ...
  inherited_permission (child_perm, parent_perm) = ...
  can_comment (perm) = ...
  is_allowed (perm, action) = ...
  doc_access (doc_perm, folder_perm, action) = ...
  has_at_least (user_perm, required_perm) = ...
  can_read (perm) = ...
  multi_group_permission (perm1, perm2, perm3) = ...
  can_grant (granter_perm, grantee_perm) = ...
  can_transfer_ownership (perm) = ...

λ> :verify ∀(x : ℝ). x * x ≥ 0
✅ Valid

λ> :quit
Goodbye! 👋
```

## Tips

1. Press **Ctrl+C** to cancel input
2. Press **Ctrl+D** or type `:quit` to exit
3. Use `:symbols` to copy-paste Unicode math symbols
4. Use `:help <topic>` for detailed help (e.g., `:help quantifiers`)

## What's Next?

For a richer interactive experience with plots and visualizations:

→ [Jupyter Notebook](21-jupyter-notebook.md)

Or explore practical applications:

→ [Applications](13-applications.md)
