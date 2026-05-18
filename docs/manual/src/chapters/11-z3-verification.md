# Z3 Verification

## What is Z3?

[Z3](https://github.com/Z3Prover/z3) is a theorem prover from Microsoft Research. Kleis uses Z3 to:

- **Verify** mathematical statements
- **Find counterexamples** when statements are false
- **Check** that implementations satisfy axioms

## Basic Verification

Use `verify` to check a statement:

```kleis example
axiom commutativity : ∀(x : ℝ)(y : ℝ). x + y = y + x
// Z3 verifies: ✓ Valid

axiom zero_annihilates : ∀(x : ℝ). x * 0 = 0
// Z3 verifies: ✓ Valid

axiom all_positive : ∀(x : ℝ). x > 0
// Z3 finds counterexample: x = -1
```

## Verifying Quantified Statements

Z3 handles universal and existential quantifiers:

```kleis example
axiom additive_identity : ∀(x : ℝ). x + 0 = x
// Z3 verifies: ✓ Valid

axiom squares_nonnegative : ∀(x : ℝ). x * x ≥ 0
// Z3 verifies: ✓ Valid (squares are non-negative)

axiom no_real_sqrt_neg1 : ∃(x : ℝ). x * x = -1
// Z3: ✗ Invalid (no real square root of -1)

axiom complex_sqrt_neg1 : ∃(x : ℂ). x * x = -1
// Z3 verifies: ✓ Valid (x = i works)
```

## Checking Axioms

Verify that definitions satisfy axioms:

```kleis
structure Group(G) {
    e : G
    operation mul : G × G → G
    operation inv : G → G
    
    axiom identity : ∀(x : G). mul(e, x) = x
    axiom inverse : ∀(x : G). mul(x, inv(x)) = e
    axiom associative : ∀(x : G)(y : G)(z : G).
        mul(mul(x, y), z) = mul(x, mul(y, z))
}

// Define integers with addition
implements Group(ℤ) {
    element e = 0
    operation mul = builtin_add
    operation inv = builtin_negate
}

// Kleis verifies each axiom automatically!
```

## Implication Verification

Prove that premises imply conclusions:

```kleis example
// If x > 0 and y > 0, then x + y > 0
axiom sum_positive : ∀(x : ℝ)(y : ℝ). (x > 0 ∧ y > 0) → x + y > 0
// Z3 verifies: ✓ Valid

// Triangle inequality
axiom triangle_ineq : ∀(x : ℝ)(y : ℝ)(a : ℝ)(b : ℝ).
    (abs(x) ≤ a ∧ abs(y) ≤ b) → abs(x + y) ≤ a + b
// Z3 verifies: ✓ Valid
```

## Counterexamples

When verification fails, Z3 provides counterexamples:

```kleis example
axiom square_equals_self : ∀(x : ℝ). x^2 = x
// Z3: ✗ Invalid, Counterexample: x = 2 (since 4 ≠ 2)

axiom positive_greater_than_one : ∀(n : ℕ). n > 0 → n > 1
// Z3: ✗ Invalid, Counterexample: n = 1
```

## Solver Configuration

Kleis provides fine-grained control over the Z3 solver through environment
variables. These are runtime controls — they affect `kleis test` and
`kleis eval`, not the build.

### Environment Variables

| Variable | Default | Purpose |
|----------|---------|---------|
| `KLEIS_Z3_DEBUG=1` | off | Full diagnostics: per-axiom timing, quantifier profiling, solver stats |
| `KLEIS_Z3_TIMEOUT_MS=N` | 0 (none) | Z3 internal timeout in milliseconds |
| `KLEIS_Z3_RLIMIT=N` | 0 (unlimited) | Deterministic resource limit in work units |
| `KLEIS_Z3_MEMORY_MB=N` | 2048 | Memory ceiling in megabytes (0 = unlimited) |

**`KLEIS_Z3_DEBUG=1`** enables verbose output: which axioms are being loaded,
how long each `solver.check()` call takes, quantifier instantiation counts,
and the reason when Z3 returns Unknown. This is the first tool to reach for
when a test is slow or returns an unexpected result.

**`KLEIS_Z3_RLIMIT`** is the preferred diagnostic knob for performance issues.
Unlike wall-clock timeout, the resource limit is *deterministic* — it produces
the same result on any hardware, under any system load. A value of 5,000,000
is a reasonable cap for debugging. Z3 returns Unknown when the limit is hit.

**`KLEIS_Z3_TIMEOUT_MS`** sets Z3's internal timeout. Use with caution: Z3 can
crash with an internal assertion violation when the timeout fires mid-processing
of complex quantifier reasoning. The watchdog (see below) is the safe timeout
mechanism. Only set this for diagnosing specific divergence scenarios.

**`KLEIS_Z3_MEMORY_MB`** caps Z3's memory allocation. Z3 legitimately uses
several GB for theories with many quantified axioms. The default of 2GB
is sufficient for all examples in this repository. Set to 0 to disable.

### The Two-Layer Safety Architecture

Z3's internal timeout can fail to trigger during complex quantifier
instantiation loops, causing `solver.check()` to hang indefinitely. Kleis
protects against this with two independent safety layers:

**Layer 1 — External Watchdog (primary):**
Every `solver.check()` call is wrapped in a scoped watchdog thread that:

- Polls on a wall-clock timer (timeout + 2 seconds headroom)
- Monitors `Z3_get_estimated_alloc_size()` against the memory limit
- Calls `ContextHandle::interrupt()` if either limit is exceeded
- Z3 returns Unknown with reason "canceled" — no crash, no abort

The watchdog is always active. It is the safe timeout mechanism.

**Layer 2 — Z3 Internal `memory_max_size` (backstop):**
Set to 125% of the external memory limit. If the watchdog's polling interval
misses a sudden allocation spike, Z3's internal allocator returns null instead
of allocating. The vendored z3 crate handles null returns with a clean exit.

### Configuration File

Z3 settings can also be set in a configuration file:

```toml
# ~/.config/kleis/config.toml  (or config/kleis.toml in project root)
[z3]
timeout_ms = 30000
```

Environment variables override the config file, which overrides defaults.

### Diagnostic Workflow

When a Z3 check is slow or returns Unknown, follow these steps:

```bash
# Step 1: See what's happening inside the solver
KLEIS_Z3_DEBUG=1 kleis test file.kleis

# Step 2: Cap work units for reproducible diagnosis
KLEIS_Z3_RLIMIT=5000000 kleis test file.kleis

# Step 3: If memory is the bottleneck, lower the ceiling
KLEIS_Z3_MEMORY_MB=512 kleis test file.kleis

# Step 4: Hard wall-clock cap (diagnostic only — Z3 may crash internally)
KLEIS_Z3_TIMEOUT_MS=2000 kleis test file.kleis

# Combined: full diagnostics with resource cap
KLEIS_Z3_DEBUG=1 KLEIS_Z3_RLIMIT=5000000 kleis test file.kleis
```

The debug output identifies which axiom or quantifier is causing divergence,
so you can refactor the problematic expression rather than increasing limits.

### Timeout and Limits

Complex statements may time out:

```kleis
// Very complex statement — may exceed solver capacity
verify ∀ M : Matrix(100, 100) . det(M * M') ≥ 0
// Result: ⏱ Timeout (statement too complex)
```

When Z3 returns Unknown, it does *not* mean the statement is false. It means
the solver could not determine the answer within its resource budget. Options:

1. **Increase resources**: raise `KLEIS_Z3_RLIMIT` or `KLEIS_Z3_MEMORY_MB`
2. **Simplify the statement**: break it into smaller lemmas
3. **Add hints**: provide intermediate assertions that guide the solver
4. **Use Skolemization**: replace existential quantifiers with concrete witnesses

## Verifying Nested Quantifiers (Grammar v0.9)

Grammar v0.9 enables nested quantifiers in logical expressions:

```kleis
structure Analysis {
    // Quantifier inside conjunction - Z3 handles this
    axiom bounded: (x > 0) ∧ (∀(y : ℝ). y = y)
    
    // Epsilon-delta limit definition
    axiom limit_def: ∀(L a : ℝ, ε : ℝ). ε > 0 → 
        (∃(δ : ℝ). δ > 0 ∧ (∀(x : ℝ). abs(x - a) < δ → abs(f(x) - L) < ε))
}
```

### Function Types in Verification

Quantify over functions and verify their properties:

```kleis
structure Continuity {
    // Z3 treats f as an uninterpreted function ℝ → ℝ
    axiom continuous_at: ∀(f : ℝ → ℝ, a : ℝ, ε : ℝ). ε > 0 →
        (∃(δ : ℝ). δ > 0 ∧ (∀(x : ℝ). abs(x - a) < δ → abs(f(x) - f(a)) < ε))
}
```

**Note:** Z3 treats function-typed variables as uninterpreted functions, allowing reasoning about their properties without knowing their implementation.

## What Z3 Can and Cannot Do

### Z3 Excels At:
- Linear arithmetic
- Boolean logic
- Array reasoning
- Simple quantifiers
- Algebraic identities
- Nested quantifiers (Grammar v0.9)
- Function-typed variables

### Z3 Struggles With:
- Non-linear real arithmetic (undecidable in general)
- Very deep quantifier nesting (may timeout)
- Transcendental functions (sin, cos, exp)
- Infinite structures
- Inductive proofs over recursive data types

## Practical Workflow

1. **Write structure with axioms**
2. **Implement operations**
3. **Kleis auto-verifies** axioms are satisfied
4. **Use `verify`** for additional properties
5. **Examine counterexamples** when verification fails

```kleis
// Step 1: Define structure
structure Ring(R) {
    zero : R
    one : R
    operation add : R × R → R
    operation mul : R × R → R
    operation neg : R → R
    
    axiom add_assoc : ∀(a : R)(b : R)(c : R).
        add(add(a, b), c) = add(a, add(b, c))
}

// Step 2: Implement for integers
implements Ring(ℤ) {
    element zero = 0
    element one = 1
    operation add = builtin_add
    operation mul = builtin_mul
    operation neg = builtin_negate
}

// Step 3: Auto-verification happens!

// Step 4: Check additional properties
axiom mul_zero : ∀(x : ℤ). mul(x, zero) = zero
// Z3 verifies: ✓ Valid
```

## Solver Abstraction Layer

While this chapter focuses on Z3, Kleis is designed with a **solver abstraction layer** that can interface with multiple proof backends.

### Architecture

```
User Code (Kleis Expression)
         │
    SolverBackend Trait
         │
   ┌─────┴──────┬───────────┬──────────────┐
   │            │           │              │
Z3Backend  CVC5Backend  IsabelleBackend  CustomBackend
   │            │           │              │
   └─────┬──────┴───────────┴──────────────┘
         │
  OperationTranslators
         │
   ResultConverter
         │
User Code (Kleis Expression)
```

### The SolverBackend Trait

The core abstraction is defined in `src/solvers/backend.rs`:

```rust
// Simplified — the full trait (src/solvers/backend.rs) has 16 methods
// including scope management, structure loading, and function definition.
pub trait SolverBackend {
    /// Get solver name (e.g., "Z3", "CVC5")
    fn name(&self) -> &str;

    /// Get solver capabilities (declared upfront, MCP-style)
    fn capabilities(&self) -> &SolverCapabilities;

    /// Verify an axiom (validity check)
    fn verify_axiom(&mut self, axiom: &Expression) 
        -> Result<VerificationResult, String>;

    /// Check if an expression is satisfiable
    fn check_satisfiability(&mut self, expr: &Expression) 
        -> Result<SatisfiabilityResult, String>;

    /// Evaluate an expression to a concrete value
    fn evaluate(&mut self, expr: &Expression) 
        -> Result<Expression, String>;

    /// Simplify an expression
    fn simplify(&mut self, expr: &Expression) 
        -> Result<Expression, String>;

    /// Check if two expressions are equivalent
    fn are_equivalent(&mut self, e1: &Expression, e2: &Expression) 
        -> Result<bool, String>;

    // ... plus: load_structure_axioms, check_consistency, push, pop,
    //     reset, load_identity_element, assert_expression, define_function
}
```

**Key design principle:** All public methods work with Kleis `Expression`, not solver-specific types. Solver internals never escape the abstraction.

### MCP-Style Capability Declaration

Solvers declare their capabilities upfront (inspired by Model Context Protocol):

```rust
pub struct SolverCapabilities {
    pub solver: SolverMetadata,      // name, version, type
    pub capabilities: Capabilities,   // operations, theories, features
}

pub struct Capabilities {
    pub theories: HashSet<String>,              // "arithmetic", "boolean", etc.
    pub operations: HashMap<String, OperationSpec>,
    pub features: FeatureFlags,                 // quantifiers, evaluation, etc.
    pub performance: PerformanceHints,          // timeout, max axioms
}
```

This enables:
- **Coverage analysis** - Know what operations are natively supported
- **Multi-solver comparison** - Choose the best solver for a program
- **User extensibility** - Add translators for missing operations

### Verification Results

```rust
pub enum VerificationResult {
    Valid,                              // Axiom holds for all inputs
    Invalid { counterexample: String }, // Found a violation
    Unknown,                            // Timeout or too complex
}

pub enum SatisfiabilityResult {
    Satisfiable { example: String },    // Found satisfying assignment
    Unsatisfiable,                      // No solution exists
    Unknown,
}
```

### Why Multiple Backends?

Different proof systems have different strengths:

| Backend | Strength | Best For |
|---------|----------|----------|
| **Z3** | Fast SMT solving, decidable theories | Arithmetic, quick checks, counterexamples |
| **CVC5** | Finite model finding, strings | Bounded verification, string operations |
| **Isabelle** | Structured proofs, automation | Complex inductive proofs, formalization |
| **Coq/Lean** | Dependent types, program extraction | Certified programs, mathematical libraries |

### Current Implementation

Currently implemented in `src/solvers/`:

| Component | Status | Description |
|-----------|--------|-------------|
| `SolverBackend` trait | ✅ Complete | Core abstraction |
| `SolverCapabilities` | ✅ Complete | MCP-style capability declaration |
| `Z3Backend` | ✅ Complete | Full Z3 integration |
| `ResultConverter` | ✅ Complete | Convert solver results to Kleis expressions |
| `discovery` module | ✅ Complete | List available solvers |
| CVC5Backend | 🔮 Future | Alternative SMT solver |
| IsabelleBackend | 🔮 Future | HOL theorem prover |

### Solver Discovery

```rust
use kleis::solvers::discovery;

// List all available backends
let solvers = discovery::list_solvers();  // ["Z3"]

// Check if a specific solver is available
if discovery::is_available("Z3") {
    let backend = Z3Backend::new()?;
}
```

### Benefits of Abstraction

1. **Solver independence** - Swap solvers without code changes
2. **Unified API** - Same methods regardless of backend
3. **Capability-aware** - Know what each solver supports before using it
4. **Extensible** - Add custom backends by implementing the trait
5. **Future-proof** - New provers can be integrated without changing Kleis code

This architecture makes Kleis a **proof orchestration layer** with beautiful notation, not just another proof assistant.

## What's Next?

Try the interactive REPL!

→ [Next: The REPL](./12-repl.md)
