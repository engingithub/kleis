# Kleis

**Kleis** is a **structure-oriented mathematical formalization language** with Z3 verification and LAPACK numerics.

> *Structures — the foundation of everything.*

| Metric | Value |
|--------|-------|
| **Grammar** | Fully implemented |
| **Tests** | 2,490 Rust unit tests |
| **Examples** | 223 Kleis files across 38 domains |
| **Built-in Functions** | 100+ (including LAPACK numerical operations) |
| **Tooling** | REPL, Jupyter kernel, VS Code extension, DAP debugger |
| **Turing Complete** | Yes — proven via LISP interpreter in Kleis |

---

## 🎯 What Makes Kleis Different

| Traditional | Kleis |
|-------------|-------|
| "What **type** is this?" | "What **structure** does this inhabit?" |
| Types define data | Structures define **axioms** |
| Prove every step (Lean/Coq) | State structure, **verify consistency** |
| Symbolic OR numerical | Symbolic (Z3) **AND** numerical (LAPACK) |

```kleis
structure Group(G) extends Monoid(G) {
    operation inv : G → G
    
    axiom left_inverse : ∀ x : G . inv(x) * x = e
    axiom right_inverse : ∀ x : G . x * inv(x) = e
}
```

---

## 🚀 Quick Start

### Install & Build

```bash
# Clone
git clone https://github.com/eatikrh/kleis.git
cd kleis

# Build with auto-detected Z3
./scripts/build-kleis.sh

# Build with numerical features (LAPACK for eigenvalues, SVD, matrix exp)
./scripts/build-kleis.sh --numerical
```

### Run the REPL

```bash
./scripts/kleis repl
```

```
🧮 Kleis REPL v0.1.0
λ> :load examples/control/eigenvalues.kleis
✅ Loaded

λ> :verify ∀(x : ℝ, y : ℝ). x + y = y + x
✅ Valid
```

### Run in Jupyter

```bash
cd kleis-notebook
pip install -e .
python -m kleis_kernel.install
jupyter notebook
```

```kleis
example "eigenvalues" {
    let A = Matrix([[1, 2], [3, 4]]) in
    out(eigenvalues(A))
}
```

Output:
```
┌         ┐
│ 5.37228 │
│-0.37228 │
└         ┘
```

### Run Tests

```bash
cargo test           # All 2,490 tests
cargo test --lib     # Library tests only
```

---

## 📊 Domain Coverage

Kleis has been used to formalize:

| Domain | Examples |
|--------|----------|
| **Mathematics** | Differential forms, tensor algebra, complex analysis, number theory |
| **Physics** | Dimensional analysis, quantum entanglement, orbital mechanics |
| **Control Systems** | LQG controllers, eigenvalue analysis, state-space models |
| **Ontology** | Projected Ontology Theory, spacetime types, kernel projections |
| **Protocols** | IPv4 packets, IP routing, stop-and-wait ARQ |
| **Authorization** | OAuth2 scopes, Google Zanzibar |
| **Formal Methods** | Petri nets, mutex verification, bounded model checking |
| **Games** | Chess, Contract Bridge, Sudoku |
| **Business** | Order-to-cash, inventory constraints |
| **Security** | SQL injection detection |
| **Meta-programming** | Kleis-in-Kleis, Lisp-in-Kleis |

See `examples/` for all 71 example files.

---

## 📄 Research

Machine-verified papers produced entirely in Kleis — axioms, proofs, numerical computation, and typesetting in a single reproducible source file.

| Paper | Description |
|-------|-------------|
| [Flat Galactic Rotation Curves from Projected Ontology](docs/papers/pot_flat_rotation_curves.pdf) | Derives flat rotation curves and the baryonic Tully-Fisher relation (M ∝ v⁴) from first principles, without dark matter. All 14 axioms and 4 theorems verified by Z3. ([source](docs/papers/pot_flat_rotation_curves.kleis)) |

---

## 🔬 Core Features

### Z3 Axiom Verification

```kleis
structure MetricSpace(X, d) {
    axiom symmetry : ∀ x y : X . d(x, y) = d(y, x)
    axiom triangle : ∀ x y z : X . d(x, z) ≤ d(x, y) + d(y, z)
    axiom identity : ∀ x y : X . d(x, y) = 0 ↔ x = y
}
```

```
λ> :verify ∀(a : ℝ, b : ℝ). (a + b)² = a² + 2*a*b + b²
✅ Valid
```

### Numerical Computation (LAPACK)

| Function | Description |
|----------|-------------|
| `eigenvalues(A)` | Eigenvalues of a matrix |
| `eigenvectors(A)` | Eigenvectors |
| `svd(A)` | Singular value decomposition |
| `solve(A, b)` | Solve Ax = b |
| `inv(A)` | Matrix inverse |
| `expm(A)` | Matrix exponential |
| `schur(A)` | Schur decomposition |

Complex variants: `eigenvalues_complex`, `expm_complex`, etc.

### Interactive Output

```kleis
example "matrix operations" {
    let A = Matrix([[1, 2], [3, 4]]) in
    out(A)
    out(multiply(A, A))
    out(eigenvalues(A))
}
```

Pretty-printed with Unicode box drawing:
```
┌     ┐
│ 1 2 │
│ 3 4 │
└     ┘
```

### Hindley-Milner Type Inference

```kleis
define compose = λ f . λ g . λ x . f(g(x))
// Inferred: (β → γ) → (α → β) → α → γ
```

---

## 🛠️ Tooling

| Tool | Command | Description |
|------|---------|-------------|
| **REPL** | `./scripts/kleis repl` | Interactive exploration |
| **Jupyter** | Kleis kernel | Notebook integration with `out()` |
| **VS Code** | Extension | Full IDE support (see below) |
| **DAP Debugger** | `./scripts/kleis server` | Step-through debugging |
| **CLI** | `./scripts/kleis eval` | Evaluate expressions |
| **Checker** | `./scripts/kleis check` | Validate .kleis files |

### VS Code Extension

Install from `vscode-kleis/` or search "Kleis" in VS Code Marketplace.

**Features:**
- **Syntax highlighting** for `.kleis` files
- **Keyword recognition**: `structure`, `implements`, `operation`, `axiom`, `data`, `match`, `verify`
- **Type highlighting**: `ℝ`, `ℂ`, `ℤ`, `ℕ`, `Matrix`, `Vector`, `Scalar`
- **Mathematical operators**: `∀`, `∃`, `λ`, `→`, `⇒`, `∈`, `∇`, `∂`, `∫`, `√`
- **Greek letters**: Full alphabet (α, β, γ, δ, ...)
- **Bracket matching** and **comment toggling**
- **LSP integration** for real-time diagnostics

### DAP Debugger

The unified server provides Debug Adapter Protocol support:

```bash
./scripts/kleis server   # Starts LSP + DAP on stdio
```

**Debugging features:**
- **Breakpoints** in `.kleis` files
- **Step-through execution** (step in, step over, step out)
- **Variable inspection** at each step
- **Call stack** visualization
- **Expression evaluation** during pause

Configure in VS Code's `launch.json` to debug Kleis files interactively.

---

## 📁 Project Structure

```
kleis/
├── src/                    # Rust source (parser, evaluator, type system)
│   ├── kleis_parser.rs     # Full grammar parser
│   ├── evaluator.rs        # Expression evaluator (100+ built-ins)
│   ├── type_inference.rs   # Hindley-Milner type inference
│   ├── z3_integration.rs   # Z3 axiom verification
│   └── bin/                # Binaries (repl, server, kleis)
├── stdlib/                 # Standard library
│   ├── prelude.kleis       # Core definitions
│   ├── tensors.kleis       # Tensor algebra axioms
│   ├── tensors_functional.kleis  # Tensor operations
│   ├── differential_forms.kleis  # Exterior calculus
│   └── combinatorics.kleis # Combinatorial structures
├── examples/               # 71 example files
│   ├── ontology/           # Projected Ontology Theory
│   ├── control/            # Control systems
│   ├── number-theory/      # FLT exploration
│   └── ...                 # 38 domains
├── kleis-notebook/         # Jupyter kernel
├── docs/                   # Documentation
│   ├── manual/             # The Kleis Manual (mdBook)
│   └── adr/                # Architecture Decision Records
└── tests/                  # 2,490 tests
```

---

## 📚 Documentation

| Resource | Description |
|----------|-------------|
| [The Kleis Manual](docs/manual/) | Comprehensive language guide |
| [Rust API Docs](https://kleis.io/api/kleis/) | Rustdoc for the codebase |
| [docs/adr/](docs/adr/) | 23 Architecture Decision Records |
| [docs/grammar/](docs/grammar/) | Formal grammar specification |
| [OPERATOR_INVENTORY.md](docs/OPERATOR_INVENTORY.md) | Complete operator reference |

---

## 🎓 Philosophy

Kleis is built on the principle that **structures**, not types, are the foundation of mathematical reasoning.

A type tells you *what* something is. A structure tells you *what laws it obeys*.

```kleis
// A type: "x is a real number"
x : ℝ

// A structure: "ℝ is a field with these axioms"
structure Field(F) {
    axiom additive_inverse : ∀ x : F . ∃ y : F . x + y = 0
    axiom multiplicative_inverse : ∀ x : F . x ≠ 0 → ∃ y : F . x * y = 1
}
```

This approach enables:
- **Cross-domain formalization** — Same framework for physics, protocols, games
- **Axiom-driven verification** — Z3 checks consistency, not just types
- **Extensibility** — Add new structures without changing the core
- **Security & Compliance** — Machine-checkable proofs for audit across sectors
- **Complex systems** — Verify rules across IoT, enterprise, and distributed systems
- **AI-ready** — Designed to verify AI-generated specifications for logical consistency

---

## 🏗️ Development

### Quality Gates

```bash
cargo fmt --all                                    # Format code
cargo clippy --all-targets --all-features          # Lint
cargo test                                         # Run all 2,490 tests
```

### Add a New Built-in

1. Add to `src/evaluator.rs` in `apply_builtin()`
2. Add tests
3. Document in `docs/OPERATOR_INVENTORY.md`

---

## 📝 License

See `LICENSE` file.

---

**Kleis** — Where structures meet verification. 🦀
