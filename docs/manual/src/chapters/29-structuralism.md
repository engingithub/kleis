# Structuralism

> *"Structure is the universal abstraction."*

This chapter is not about Kleis the language. It is about the idea that Kleis embodies — and the architecture that follows from that idea.

## The Thesis

Any domain amenable to formalization follows the same pattern:

| Component        | What it does                        |
|------------------|-------------------------------------|
| **Notation**     | Represents objects and relations    |
| **Rules**        | Governs what is valid               |
| **Verification** | Checks that rules hold              |
| **Output**       | Produces a result for humans        |

Mathematics has notation (symbols), rules (axioms), verification (proof), and output (papers). Physics has notation (tensor indices), rules (field equations), verification (experiment or proof), and output (predictions). A code review has notation (source code), rules (coding standards), verification (pass/fail checks), and output (a review report).

Law follows the same pattern. International law has notation (treaty articles, doctrine names), rules (Article 2(4), Article 51), verification (does the act satisfy the qualifying conditions?), and output (a judgment: Permitted or Prohibited). This is not a metaphor. The UN Charter Article 51 formalization in `examples/authorization/` axiomatizes two competing self-defense doctrines, encodes case facts as a structure, and lets Z3 deliver the verdict. The same substrate that proves tensor symmetry also proves that an unprovoked military strike is illegal under both doctrines.

This follows an old tradition. The Roman jurists extracted general rules (*regulae iuris*), reduced disputes to structured categories, and separated fact from legal qualification. They were not writing novels — they were building a system. When law is axiomatized, courts become evaluators of constraint satisfaction, lawyers become modelers of fact patterns, and no one can hide behind vague doctrine. They must say: "I am invoking *this* doctrine. I am asserting *this* predicate. I have *this* evidence." The Kleis formalization enacts this aspiration with a theorem prover.

Kleis provides the substrate for this universal pattern. Not a tool for one domain — a substrate for all of them.

## Presentation Facilitates Cognition

The fourth component — output — is not an afterthought. Presentation is how knowledge becomes accessible.

This is why Kleis renders equations in proper mathematical typography rather than displaying raw syntax trees. It is why the Kleis grammar uses `∀`, `∃`, `→`, `×` instead of ASCII approximations — the notation on screen should be the notation in the textbook. It is why Kleis generates Typst PDFs suitable for papers and theses, not just terminal dumps.

The principle runs through the entire system. The Equation Editor exists because when you see `Γᵏₘₙ` rendered with proper indices, you immediately understand what it means — the visual form *is* the meaning. Stare at `christoffel(k, -m, -n)` and you have to reconstruct the mathematics in your head. The five render targets exist because the same expression needs to look right in a terminal, a web page, a LaTeX document, a PDF, and a computation engine — and "looking right" is not vanity, it is legibility. Legibility is comprehension. Comprehension is where new ideas come from.

A system that can verify theorems but cannot present them readably fails at half of its purpose. Structure without presentation is a tree falling in a forest.

## Two ASTs

Kleis has two distinct abstract syntax trees. Understanding why is the key to the architecture.

**The Editor AST** lives in the Equation Editor (JavaScript, `static/index.html`). It carries *semantic and visual* information: operation names like `gamma`, `riemann`, `index_mixed`, plus typesetting hints — superscripts, subscripts, matrix delimiters, bracket styles. It knows what something *is* and how it should *look*.

**The Kleis AST** lives in the parser and evaluator (Rust, `src/kleis_parser.rs`). It carries *semantic and computational* information: expressions with source spans, types, operations that can be evaluated, differentiated, and sent to Z3. It knows what something *means* and how to *compute* with it.

The Editor AST is richer visually but cannot compute. The Kleis AST can compute but does not know about rendering. The renderers bridge them.

## Five Projections

The Editor AST renders to five targets:

```
Editor AST (semantic + visual)
    │
    ├── → Unicode    Γᵏₘₙ              (terminal display)
    ├── → LaTeX      \Gamma^{\lambda}_{\mu\nu}   (papers)
    ├── → HTML       <sup>λ</sup>...    (web)
    ├── → Typst      Gamma^λ_{μν}       (PDF generation)
    └── → Kleis      Γ(λ, -μ, -ν)      (computation)
```

The first four targets produce representations for human consumption. The fifth — Kleis notation — produces text that the Kleis parser can ingest, creating a Kleis AST. That AST can then be type-checked, evaluated, differentiated symbolically, or verified by Z3.

This means the Equation Editor is not just a renderer. It is a *compiler* — from visual mathematics to executable specification.

## The Equation Editor Knows Mathematics

The Equation Editor is not a drawing tool. Beneath the visual rendering, it displays the algebraic data type of the expression and has buttons for formal verification:

- **Type display** — shows the inferred type: `Matrix(2, 2, Scalar)`, `Tensor(Metric, [down, down])`, etc.
- **Verify** — sends the Kleis-rendered expression to Z3 for verification against loaded axioms
- **Sat?** — checks satisfiability of the expression as a constraint

The editor knows that covariant indices go down and contravariant indices go up. It knows tensor symmetry axioms. It knows dimension constraints on matrix multiplication. None of this is hardcoded — the knowledge comes from axioms written in Kleis structures (`stdlib/` and user-defined `.kleis` files). Kleis does not just parse mathematical notation; it understands the axiomatic semantics of what it parses. The grammar tells it *how to read*. The axioms tell it *what things mean*.

## Self-Hosting

The universal formula applies to Kleis itself:

| Component        | Kleis-on-Kleis                                    |
|------------------|---------------------------------------------------|
| **Notation**     | `.kleis` source files                             |
| **Rules**        | `axiom` declarations in structures                |
| **Verification** | Pluggable solver backend, type inference, evaluator |
| **Output**       | Typst PDFs, review reports, REPL results          |

The verification slot is not welded to one tool. Z3 is the default SMT solver, but the backend is pluggable — an experimental branch has integrated Isabelle, whose tactic-based proof strategies are a different shape than Z3's satisfiability checks. The slot has a contract (accept a proposition, return a judgment), but the strategies behind that contract vary: SMT solving, tactic-based proof search, and model checking are structurally different ways to inhabit the same slot. The architecture accommodates this, but does not pretend the differences are invisible.

But self-hosting goes deeper than using Kleis to write Kleis programs. The structural scanner for Rust code review — `rust_parser.kleis` — is written *in Kleis*. It defines a `Crate` structure with operations like `scan()`, `crate_functions()`, `non_test_fns_containing()`. The review policy — `rust_review_policy.kleis` — imports this scanner and uses it to analyze Rust source code.

The result is three layers of AST, all evaluated by the same tree walker:

```
Layer 1: rust_review_policy.kleis     (the rules)
    ↓ calls into
Layer 2: rust_parser.kleis            (the scanner)
    ↓ produces
Layer 3: Crate structure              (the scanned Rust code)
    ↓ queried by
Layer 1: rust_review_policy.kleis     (back to the rules)
    ↓ produces
"pass" or "fail: reason"
```

One evaluator. One tree walker. Three languages deep.

## The Seed

The architecture was not designed top-down. It grew from a seed: the template.

The Equation Editor uses templates — patterns with placeholders for sub-expressions. A fraction template has slots for numerator and denominator. A matrix template has slots for entries. A summation template has slots for variable, bounds, and body.

The template system is not a convenience layer — it is an extension point. Templates are designed to be extendable in the same way that Kleis structures and axioms are extendable. Because the platform is axiomatic, a new domain only needs new templates and new structures. Molecular diagrams backed by chemistry structures and reaction axioms written in Kleis. Musical notation backed by counterpoint rules. Circuit schematics backed by Kirchhoff's laws. The template provides the visual entry; the structure provides the semantics; the axioms provide the verification. And because every `.kleist` template carries rendering strings for all five targets — Unicode, LaTeX, HTML, Typst, and Kleis — the new domain immediately has terminal display, papers, web pages, PDFs, and computation. The template author writes five rendering strings; the platform provides everything else. Same pattern, new domain.

Templates with placeholders for other templates is a recursive structure. It is also, in embryonic form, the Bourbaki program: mathematics built from structures composed of other structures, all the way down.

From templates came the Editor AST. From the Editor AST came the multi-target renderer. From the Kleis render target came the connection to the parser. From the parser came the evaluator. From the evaluator came Z3 verification. From verification came the review engine. Each layer emerged because the substrate — structure with slots for more structure — was the right shape for the next thing.

## The Substrate

Kleis is 7.4 MB. Inside that binary:

- A language parser and evaluator
- Hindley-Milner type inference
- Z3 theorem prover integration
- A WYSIWYG equation editor with five render targets
- LSP server and DAP debugger
- Typst PDF renderer
- Jupyter kernel and REPL
- Three MCP servers (policy, theory, review)
- A structural Rust parser written in itself
- ODE solver, symbolic differentiation, LAPACK numerics
- Tensor calculus, category theory structures, Bell inequality proofs

It does not announce itself as any one of these things. It shows up as whatever you need today — a code reviewer, an equation editor, a proof assistant, a document generator — and the rest is there when you are ready for it.

> *"Any domain that has notation, rules, and outputs fits the same pattern. Kleis provides the substrate."*

Kleis is not a language. It is a structure transformer — accepting structure as input, applying structural rules, and projecting the result into forms that humans and machines can use.

## Types with Axioms

The deepest structural decision in Kleis is the definition of "type."

In most programming languages, a type defines *shape* — fields, methods,
memory layout, interface guarantees. In object-oriented programming, a class
defines elements and operations. In algebraic type theory, a type defines
constructors and eliminators. These are useful, but they stop short. They
tell you what you *can* do with a value; they do not tell you what *must
hold* about it.

Kleis adds the missing third leg:

| Component      | OOP (class)        | Algebraic (ADT)    | Kleis (structure)          |
|----------------|--------------------|--------------------|----------------------------|
| **Carriers**   | Fields             | Constructors       | Elements, data types       |
| **Operations** | Methods            | Functions          | Operations                 |
| **Axioms**     | —                  | —                  | Axioms, verified by solver |

A Kleis structure is not a container. It is a *theory*. When you write:

```kleis
structure Group(T) {
    operation mul : T → T → T
    element e : T
    axiom assoc : ∀(a b c : T). mul(mul(a, b), c) = mul(a, mul(b, c))
    axiom left_id : ∀(a : T). mul(e, a) = a
}
```

you are not declaring a data layout. You are defining a logical universe —
a set of constraints that any inhabitant must satisfy. The solver enforces
these constraints globally. That is why the same mechanism works for tensor
symmetry, coding standards, and international law: all are constraint
systems over a domain. The domain changes; the mechanism does not.

## Theory as a First-Order Object

When Kleis sends a structure to Z3, it passes the carriers, operations,
and axioms as data — arguments to a function call. The solver receives
a theory, evaluates a query against it, and returns a result. This is
not a metaphor; at the implementation level, Z3 is a function:

```
solve(context) → sat | unsat | unknown
```

The consequence is that *theory becomes a first-order object*. It can be
versioned, parameterized, compared, and composed. Two competing doctrines
become two values of a `Doctrine` type. A case file becomes a structure
that instantiates fact predicates. The query `status(Strict, case_act)`
and `status(Anticipatory, case_act)` evaluates two theories against the
same facts in the same call.

This is reification of theory — turning meta-level concepts into
manipulable data. Once theory is data, you get *executable pluralism*:
comparative jurisprudence, comparative physics, comparative doctrine,
all become structural comparisons over parameterized theories.

## The Deliberate Boundary

This design is stable because of what it does *not* do.

Kleis does not internalize proof objects. It does not let axioms quantify
over axioms. It does not let structures inspect their own consistency.
It does not identify equivalent structures as equal inside the system.
The solver is an external oracle, not an internal reasoning engine.

This is a deliberate boundary. The moment a system allows types to talk
about types, axioms to quantify over axioms, or structures to reason
about their own provability, it enters Gödel territory — where stability
becomes conditional and complexity explodes. Homotopy Type Theory (HoTT)
takes that step: it internalizes structural equivalence as identity,
making equivalence between types a first-class concept inside the system.
That is more expressive, but dramatically heavier.

Kleis sits between Bourbaki and HoTT:

| System      | Structures are...                           | Identity is...        |
|-------------|---------------------------------------------|-----------------------|
| **Bourbaki**    | Formal exposition                           | External              |
| **Kleis**       | Executable first-order theories             | External, solver-checked |
| **HoTT**        | Higher types with internalized equivalence  | Internal (univalence) |

Bourbaki described structures but could not execute them. HoTT internalizes
everything but at the cost of enormous complexity. Kleis occupies the
middle: structures are executable, axioms are enforced, but the system does
not reason about itself. Theories are data; the solver is an oracle. That
boundary is what keeps a 7.4 MB binary stable across domains from tensor
calculus to international law.

## The Oracle

The word "oracle" in computer science usually means "black box" — a
function you call without knowing how it works. Hand it a question, get
back an answer, move on.

In Kleis, the oracle relationship is richer. Z3 does not merely return
`sat` or `unsat`. It returns *models* — concrete witnesses that satisfy
or violate the theory. And Kleis *understands* those answers, because
the question was formulated in Kleis's own vocabulary: the carriers,
operations, and axioms that Kleis defined and passed in.

The interaction follows a pattern:

1. **Kleis formulates the question** — in its own language, using its
   own types and axioms.
2. **The oracle answers from outside** — using decision procedures,
   model construction, and satisfiability algorithms that Kleis knows
   nothing about.
3. **Kleis interprets the answer** — because the answer is expressed
   in terms Kleis defined.

The oracle exists because Kleis *cannot* answer these questions itself —
not without becoming self-reflective. Determining satisfiability,
constructing models, checking entailment — these require reasoning
*about* the theory, not *within* it. If Kleis internalized that
capability, it would need to represent its own axioms as data, quantify
over its own propositions, and evaluate its own consistency. That is
the self-referential loop the architecture refuses. The oracle is not
an optimization; it is the boundary that keeps the system first-order.
Kleis asks in its own language and interprets the answer in its own
terms, but the act of judgment happens outside.

This is why the boundary is stable. Kleis does not need to internalize
what Z3 knows. It needs to formulate questions well and interpret results
correctly. The knowledge lives outside; the understanding lives inside.
Neither side needs to become the other.

The ancient meaning of "oracle" was the same — you go to Delphi, you
ask in your own language, the oracle answers from a source you cannot
access, and you return home to interpret the answer in your own context.
The oracle never becomes part of your city. Your city never becomes part
of the oracle. But the exchange produces knowledge that neither side had
alone.

## Calculemus

Leibniz imagined two things. The first was a *characteristica universalis* —
a universal symbolic language capable of expressing all reasoning. The second
was a *calculus ratiocinator* — a mechanical engine that would operate on
that language and determine whether statements were valid. Together they
formed a vision: when disagreements arise, we should say *"Calculemus"* —
let us calculate.

The Kleis architecture is a direct realization of that vision.

| Leibniz                       | Kleis                              |
|-------------------------------|------------------------------------|
| Characteristica universalis   | Kleis language                     |
| Calculus ratiocinator         | Z3 solver                          |
| Symbolic knowledge            | Kleis expressions                  |
| Mechanical reasoning          | Automated verification             |
| *Calculemus*                  | `assert(∀(x : ℝ). x + 0 = x)`    |

The reasoning loop is the same one Leibniz described three centuries ago:

```
proposition → formal expression → calculation → result
```

Leibniz lacked the machinery. The pieces arrived over centuries: formal
logic from Frege, the logicist program and type theory from Russell,
the axiomatic method from Hilbert, the completeness theorem and its
limits from Gödel, automated theorem proving from the 20th century
logic tradition, and SMT solvers from the last two decades. Bourbaki
operationalized the structural half — mathematics as structures defined
by sets, operations, and axioms — but could not execute them. Kleis
closes the loop: Bourbaki structures that a solver can verify.

```
Leibniz (1679) — dream of symbolic reasoning
    ↓
Frege (1879) — Begriffsschrift: formal predicate logic
    ↓
Russell (1903) — Principia Mathematica, type theory
    ↓
Hilbert (1899) — formal axiomatic method
    ↓
Gödel (1931) — completeness and limits of formal systems
    ↓
Bourbaki (1939) — structural formalism for mathematics
    ↓
Automated reasoning (1960s–2000s) — decision procedures
    ↓
Kleis — executable Bourbaki-style structures
```

The connection runs deeper than analogy. Leibniz believed that concepts
could be decomposed into primitive components, somewhat like numbers factor
into primes. Bourbaki turned that intuition into a program: mathematics
built from structures composed of other structures, all the way down. Kleis
makes those structures executable — and adds the ratiocinator that Leibniz
imagined but could not build.

## Everything Is an Expression

Kleis does not privilege any foundational ontology. It does not assert that
mathematics is fundamentally about sets (ZFC), categories (category theory),
or types (type theory). Instead, the system adopts a uniform primitive:

> *Everything is an Expression.*

Structures, axioms, theorems, operations, values — all are expressions
inside the same system. The language does not require a meta-language.

| System              | Foundational object        |
|---------------------|----------------------------|
| ZFC                 | Sets                       |
| Category theory     | Objects and morphisms      |
| Type theory         | Types                      |
| **Kleis**           | **Expressions**            |

This is not a metaphysical claim about the nature of reality. It is a
syntactic commitment: formal reasoning artifacts are represented as
expressions. Whether the universe itself is symbolic is a different question
entirely.

The consequence is that any of those foundational frameworks can be
*constructed* inside Kleis without the language taking sides. Set theory,
category theory, and type theory all become structures defined by axioms —
inhabitants of the expression system, not assumptions baked into the
grammar.

There is a type-oriented organization: in Kleis, `structure ≈ type`. The
Hindley-Milner engine infers types, unifies them, and ensures consistency.
Axioms attach to structures as semantic constraints; the HM unification
mechanism itself is unchanged — it still generates constraints, unifies,
and infers principal types without knowing about axioms.

But the two layers are not fully independent. When a structure extends
another, polymorphism honors the inherited axioms. A `Field` that extends
`Ring` carries the ring axioms forward. The type hierarchy is the channel
through which axioms propagate — HM determines *what type something is*,
and the type determines *which axioms apply*. The mechanism is unchanged;
what flows through it is richer.

```
Layer 1 — Type System (HM):  type correctness, polymorphism, unification
Layer 2 — Logical Constraints: axioms, theorems, solver verification
        ↕ connected through structure inheritance
```

That design gives Kleis the engineering advantages of a type system
(inference, polymorphism, error detection) without the philosophical
commitment that everything *must be* a type. Types remain structural
scaffolding, not philosophical doctrine — but the scaffolding carries
weight.

This design has a practical consequence for AI agents. Because Kleis reads
like mathematical notation, LLMs adapt to it immediately — their training
data already contains vast amounts of LaTeX, symbolic logic, and functional
languages. An AI agent writing `∀(x : ℝ). x + 0 = x` is producing a
short, unambiguous token sequence that a solver can verify. No translation
layer. No hedging language. No hallucination pathway. The proposition is
either satisfiable or it is not.

Forcing humans to communicate in formal notation would be impractical. But
for machine agents, Kleis is no harder than English — and considerably less
ambiguous. That asymmetry is why the architecture works: AI agents propose
in mathematics, and Kleis decides what is admissible.

## The Constraint Layer

The three Kleis MCP servers — policy, review, and theory — follow a single
architectural pattern:

| MCP server       | What the AI sends      | What Kleis checks        |
|------------------|------------------------|--------------------------|
| **Kleis-policy** | Action proposal        | Policy axioms            |
| **Kleis-review** | Source code            | Engineering standards    |
| **Kleis-theory** | Logical proposition    | Mathematical axioms      |

The mechanism is identical in all three cases:

```
AI proposes → Kleis formalizes → solver verifies → allow or deny
```

This makes Kleis a *constraint enforcement layer* between AI cognition and
system execution. The AI agent is a proposal generator, not an authority.
The authority is the axiom system.

The pattern is not novel. The F-16 fly-by-wire system interposes a flight
control computer between pilot input and control surfaces. The pilot
requests a maneuver; the computer enforces stability constraints, g-force
limits, and structural boundaries. The pilot is creative but bounded.
Nuclear reactor protection systems, operating system kernels, and
transaction engines all follow the same principle: the proposer does not
touch the actuators directly.

| System                     | Proposer     | Constraint layer                |
|----------------------------|--------------|---------------------------------|
| F-16 fly-by-wire           | Pilot        | Flight control computer         |
| Operating system            | User process | Kernel                          |
| UNIVAC EXEC 8              | Job program  | Executive (ER dispatcher)       |
| **Kleis AI architecture**  | **AI agent** | **Kleis + solver**              |

What is unusual is not the architecture but what the constraints are written
in. Instead of control laws, rule engines, or heuristics, Kleis uses formal
axioms verified by an SMT solver. The constraint is not "this looks
acceptable" but "this is logically consistent with the policy system."

The result is that AI agents become unprivileged processes. They submit
executive requests; Kleis decides whether the system may execute them.
And because the responses — witnesses, counterexamples, verdicts — are
expressed in the same mathematical notation as the requests, the
interaction forms a closed formal loop. No natural language required
anywhere in the reasoning cycle. No hallucination surface. No hedging.

> *Instead of trying to make AI smarter, the architecture makes AI bounded.*

That is an engineering approach to AI safety: not eliminating instability in
the controller, but adding a constraint layer that keeps the system inside
the admissible region. Classic robust control thinking, applied to
cognition.

## Why Not a DSL

A natural question: why not build a domain-specific language for verified
reasoning in an existing host language?

Because a DSL for verified reasoning is just a programming language in
denial. You end up re-implementing a parser, an AST, a type system, error
reporting, modules, imports, and tooling. You need a logic backend —
either embedding an SMT-LIB generator or writing a custom reasoner. Each
"nice" feature (records, pattern matching, doctrine packs, case files)
forces new design decisions. Error messages become a nightmare: domain
users need "why unsatisfiable?" explanations, not stack traces. The
boundary between data and logic blurs. You keep adding "just one more"
construct. You re-discover the need for axiom versioning, doctrine
isolation, fact profiles, import scoping — all as first-class
requirements.

Most DSL projects fail not because the domain is hard, but because the
*meta-domain* — semantics, verification, tooling — is harder than the
domain. Kleis built the meta-domain first: stable grammar, logic
interface, structuring mechanisms, social discipline (you must declare
axioms, you must separate fact profiles, you must accept solver
consequences). That is why formalizing the UN Charter took a session,
not a quarter.

The architecture is the philosophy. The philosophy is the architecture. Structure all the way down.
