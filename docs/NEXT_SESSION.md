# Next Session Notes

**Last Updated:** May 16, 2026

---

## Active Research

### Middle Egyptian Grammar — WRITE THE KLEIS THEORY

**Status:** Reading phase complete through Lesson 8 (page ~140 of 600).
Ready to pivot from reading to writing the Kleis theory.

**What we did:**
- Read James P. Allen's "Middle Egyptian" Lessons 1-8 page by page
- Extracted **125 formalizable axioms** covering:
  - Writing system (signs, phonograms, determinatives, quadrats) — axioms 1-12
  - Nouns (gender, number, genitives, honorific transposition) — axioms 13-30
  - Pronouns (suffix, dependent, independent paradigms) — axioms 31-45
  - Demonstratives (4 series, agreement, evolution to articles) — axioms 46-55
  - Adjectives (primary, secondary, nisbe, agreement, position) — axioms 56-69
  - Non-verbal sentences (adjectival, nominal, A pw, A pw B) — axioms 70-100
  - Prepositions (17 primary, compounds, nisbes, reverse nisbes) — axioms 101-125
- All notes in `docs/notes/middle_egyptian_axiomatization.md`

**Where we left off:** End of Lesson 8, Exercise 8 (32 sentences from Sinuhe).
The remaining ~460 pages cover the verbal system (Phase 2, future work).

**What to do next — Phase 1: Write the Kleis Theory**

Follow the Moonlight Sonata pattern (4-layer architecture):

1. **Theory layer** — `stdlib/theories/middle_egyptian_grammar.kleis`
   - `HieroglyphicSpelling` structure (axioms 1-12)
   - `MiddleEgyptianNominalGrammar` structure (axioms 13-98)
   - `MiddleEgyptianPrepositions` structure (axioms 99-125)

2. **Data layer** — `examples/linguistics/sinuhe_text.kleis`
   - Encode sentences from Exercises 1-8 as test data
   - Tale of Sinuhe as primary specimen (like Moonlight Sonata)

3. **Analysis layer** — `examples/linguistics/sinuhe_analysis.kleis`
   - Apply grammar axioms to parse/verify real sentences
   - Disambiguation via Z3 Sat queries
   - Type inference as translation

4. **Paper layer** — `examples/linguistics/middle_egyptian_paper.kleis`
   - Research paper following the Moonlight Sonata format

**Key references:**
- `stdlib/theories/tonal_harmony.kleis` — model for theory structure
- `examples/music/moonlight_analysis.kleis` — model for analysis
- `docs/adr/ADR-034-Egyptian-Hieroglyph-Editor.md` — architectural decisions

**Key linguistic insight:** Middle Egyptian most resembles Classical Arabic
typologically (copulaless sentences, suffix pronouns, nisbe adjectives,
genitive constructions). The axioms capture Afroasiatic structural patterns.

---

### Kernel Factorization Reconciliation — IMPORTANT FUTURE WORK

There are **two different kernel decomposition architectures** in the POT papers that need to be reconciled:

**Architecture 1: Three-factor factorization** (Entanglement, Electrodynamics, Rotation Curves papers)
```
K = K_univ ∘ K_dyn ∘ K_rep
```
- K_univ: universal geometric structure
- K_dyn: dynamical sector (elliptic for gravity, hyperbolic for propagation)
- K_rep: representation/measurement sector (scalar for gravity, matrix-valued for spin)
- Each factor is admissible; composition is admissible by `compose_admissible`
- The EM paper extends this: K_em-sector = K_univ ∘ K_dyn ∘ K_em (where K_em = d|Ω¹)
- The entanglement paper uses K_rep as the spinor projection parameterized by detector angle

**Architecture 2: K-Q pipeline** (GR paper, Yang-Mills paper, K-Q Atlas, abstract framework)
```
Configuration → K (production) → intermediate → Q (projection) → observables
```
- K maps configurations to intermediate mathematical objects (curvature, field strength)
- Q extracts observables (Ricci tensor, physical cross-sections)
- Admissibility is tested on K and Q separately
- The formulation fiber analysis lives here (where in the K-Q pipeline is non-admissibility?)

**The tension**: The three-factor factorization decomposes the *production kernel* K into composable admissible pieces. The K-Q pipeline separates *production* from *observation*. These are orthogonal decompositions of the same overall process:

```
Full pipeline:  config → [K_univ ∘ K_dyn ∘ K_rep] → intermediate → Q → observables
                         ←── Architecture 1 ──→    ←── Arch 2 ──→
```

**Questions to resolve**:
1. Is K in the K-Q pipeline = K_univ ∘ K_dyn ∘ K_rep? If so, what is Q in terms of the three factors?
2. For non-admissible K (GR, Yang-Mills): which factor breaks admissibility? Is it K_dyn (the ω∧ω term lives in dynamics)?
3. The EM paper says K_em = d is admissible because U(1) is abelian. In the K-Q pipeline, this means K is admissible and Q (extracting observables from F) is also admissible. Does K_em map to K_dyn or K_rep in the three-factor scheme?
4. Can the formulation fiber analysis (Cartan vs TEGR moving non-admissibility between K and Q) be expressed as re-partitioning the three factors between K and Q?
5. The entanglement paper says the gravitational and measurement kernels are "different faces of the same underlying operator." But the GR paper says K_GR is non-admissible while K_measurement (spinor projection) is admissible. How do these compose if one factor is non-admissible?

**This is not a contradiction** — the three-factor scheme was developed for admissible sectors (gravity = logarithmic kernel, EM, measurement), while the K-Q pipeline handles non-admissible sectors (full GR, Yang-Mills). The reconciliation likely involves: the three-factor decomposition applies when K is admissible; when K is non-admissible, the factorization breaks and you need the K-Q pipeline instead. But this needs to be formalized.

### Projection Residues as Elementary Objects — NEW RESEARCH DIRECTION

**Status: Not yet started. Needs a dedicated paper.**

The original POT insight: when a projection kernel degenerates, the residues of the Laurent expansion *are* the elementary physical objects — point masses, point charges, angular momenta. Particles are not fundamental inputs to the theory; they are what remains when an ontological projection collapses.

**Core claim:** For a kernel K that degenerates at a point x₀, the Laurent expansion K(x) ~ R/(x - x₀)ⁿ + ... yields residues R that correspond to physical charges (mass, electric charge, spin).

**Connections across existing papers:**

1. **Renormalization → counterterms**: Q's poles in the regularization parameter ε have residues that are the physical coupling constants. This is where the idea originated — the Q operator annihilates infinity and the residue is the finite answer (-1/12, etc.).

2. **Gravity → mass**: Kernel degeneracy at a spatial point → Schwarzschild-like mass as a residue. The 1/r potential is literally the Green's function with a point-source residue.

3. **Electrodynamics → charge**: Kernel degeneracy → Coulomb charge as a residue. The EM kernel from the electrodynamics paper should have pole structure whose residue is e.

4. **Black hole singularities → no-hair theorem**: Complete kernel collapse at the singularity. The no-hair theorem says only mass, charge, and spin survive — these may be exactly the residues of the degenerate projection kernel. If so, no-hair is a corollary of the residue structure, not a separate theorem.

5. **Non-patchability**: The flat rotation curves and GR papers showed that kernels don't patch spatially. The residue analysis may explain *why* — the pole structure creates topological obstructions to global extension (analogous to how a meromorphic function can't be extended across its poles).

**Approach:**
- Start with the kernels already computed in the GR and electrodynamics papers
- Identify where they degenerate (poles, essential singularities)
- Compute the Laurent expansion and characterize residues
- Show that the residues correspond to known physical quantities
- Investigate whether the residue structure is invariant across K-Q factorization schemes (scheme independence)
- Use Kleis numerical tools (eigenvalues, FFT, tensor operations) for explicit computations

**Key question:** Is the residue structure invariant across all non-unique K-Q factorizations? If yes, the residues are the scheme-independent physical content — the part of the theory that doesn't depend on how you split K from Q.

### Future Research Questions

- **Kernel unification problem**: A reader of the four-kernel table would naturally ask whether a single kernel can reproduce all four properties (rotation curves, frame-dragging, waves, binary pulsar). A naive spatial patch fails because gravitational waves propagate through intergalactic space. A true unified kernel would need to reduce to logarithmic behavior for quasi-static galactic potentials and to linearized GR behavior for dynamical/radiative modes.

- **Teleparallel kernel in Kleis**: Formalize the TEGR pipeline in a full Kleis theory file. Computing the torsion scalar and showing it matches the Einstein-Hilbert Lagrangian (up to a boundary term) would be a strong computational verification.

- **The circularity question**: The Deser self-coupling argument says gravity must couple to its own energy. But non-admissibility makes gravitational energy non-localizable. In TEGR, gravitational energy IS localizable but Q is non-admissible. The circularity moves but doesn't resolve.

### DO NOT

- Do NOT edit the Typst output file directly
- Do NOT change the plan file
- Do NOT use `render_paper()` — the correct function is `compile_arxiv_paper()`
- Do NOT use `ArxivPaper(...)` — the correct constructor is `Paper(...)`
- Do NOT use `$Q circ K$` in paper source — Typst wants `$Q compose K$`

---

### Paper 8: One Field, Two Projections — READY TO IMPLEMENT

**Status: Plan written. Ready to implement.**

Plan file: `.cursor/plans/classical_quantum_kernel_reach_paper_8.plan.md`

#### The kernel inclusion theorem

For any phenomenon with both classical and quantum descriptions:

    ker(K_qu) ⊆ ker(K_cl)

The quantum kernel reaches strictly more of the source. The "classically invisible, quantum-activated" sector is:

    Δ = ker(K_cl) \ ker(K_qu)

For EM/QED: Δ = {ψ} (the electron field). Classical EM sees only A_μ through the exterior derivative. QED sees both A_μ and ψ through the Feynman kernel.

#### What the paper will contain

1. Kernel inclusion axioms
2. EM/QED instantiation — Δ = {ψ}
3. Gravity instantiation — linearized Green's fn vs graviton propagator
4. Fluid instantiation — Biot-Savart vs quantum fluid kernel
5. Minimum field content — union of Δ across domains
6. Philosophical payoff — quantization is kernel refinement, not ontological upgrade

---

### POT Volume XI — BUILT BUT NOT YET PUBLISHED

**Theory file:** `theories/pot_quantization_kernel.kleis` — 19/19 Z3-verified
**Worked file:** `theories/pot_quantization_kernel_worked.kleis` — 7/7 verified
**Paper file:** `examples/ontology/revised/pot_quantization_kernel_paper.kleis`
**Status:** PRs created, landing page link deliberately NOT added — needs further conceptual review

#### THE PROBLEM: Circularity

The paper assumes quantization happens and then finds patterns across quantization formalisms. This is taxonomic, not derivational. For POT, this is a problem.

#### THE KEY INSIGHT: Composite Kernel

K_obs = K_detector ∘ K_propagation ∘ K_source

The discreteness you observe is a property of the composite kernel, not an axiom about Hilbert spaces.

#### What to do next

**Option A:** Publish Volume XI as-is (structural survey). Begin Volume XII (composite kernel thesis).
**Option B:** Fold Volume XI into Volume XII.
**Option C:** Reframe Volume XI to include composite kernel section.

**Decision deferred.** Theory files, Z3 proofs, and worked examples are solid and will be reused.

---

### YANG-MILLS MASS GAP PROGRAM — 7 THEORY FILES, 230+ EXAMPLES

#### The conditional theorem

**Under Assumptions A-D, the IR singularity of the YM weight forces the dressed resolvent into the α = γ = 1/2 Darboux asymptotic class, yielding linear confinement and gap scaling ~ σ^{2/3} · 1.750.**

#### Theory files

| # | File | Examples | What it establishes |
|---|------|----------|---------------------|
| 1 | `pot_spectral_transfer.kleis` | 28 | Spectral mapping theorem, resolvent gap transfer |
| 2 | `pot_green_identification.kleis` | 33 | Anchor theorem, parameter matching, Born series |
| 3 | `pot_weight_families.kleis` | 66 | IR classification (β threshold), Darboux bridge |
| 4 | `pot_ym_darboux_matching.kleis` | 25 | Darboux universality family, gap scaling |
| 5 | `pot_ir_dressing_bridge.kleis` | 34 | Hankel duality, Born dressing, bridge equation |
| 6 | `pot_ym_assumptions.kleis` | 22 | Assumptions isolation, conditional theorem |
| 7 | `pot_assumption_c_proof.kleis` | 22 | Watson's lemma, IR/UV convergence |

#### The five assumptions

| Assumption | Status | What closes it |
|-----------|--------|----------------|
| **A** (γ > 0) | Level C | Derive γ from YM Lagrangian |
| **B** (kernel = resolvent) | Level B/C | Verify resolvent equation |
| **C** (Hankel regularity) | **Level A/B** | Watson's lemma verified; mild condition remains |
| **D** (inverse extraction) | Level B | Apply Gel'fand-Levitan |
| **E** (QFT construction) | Level C | Construct 4D YM (= Clay problem) |

#### Upgrade priority (next targets)

1. **Assumption D** — apply Gel'fand-Levitan to specific spectral measure
2. **Assumption B** — verify resolvent equation for ITCM kernel — **the decisive step**
3. **Assumption A** — non-perturbative QCD input
4. **Assumption E** — the hardest open problem in mathematical physics

#### Key technical lessons

- Z3 `divide`/`rat_div` requires numeric arguments — encode as multiplications
- Z3 `implies` with `element = 1` comparisons can fail — use separate structures
- `let` variable names can conflict with structure elements — use distinct names
- Self-contained theory files (no imports) avoid cross-file Z3 context issues
- **BUG: Nullary `ℝ` operations + equality → Z3 inconsistency.** Declaring `operation foo : ℝ` at file scope and constraining with `axiom : foo = 0.6602` causes Z3 UNSAT. Nullary `ℤ` with equality works fine. Nullary `ℝ` with inequalities works fine. **Workaround:** use `element foo : ℝ` inside the structure.
- **BUG: Evaluator substitution — compound expressions with repeated let-bound variables give wrong results inside `list_map` lambdas.** The expression `a*b - c*d` is evaluated as `((a*b) - c) * d` when operands are `fst`/`snd` applied to let-bound variables inside a `list_map` lambda. **Workaround:** decompose into separate `let` bindings.

---

### TWIN PRIME CONJECTURE — STATUS

#### Strategic evolution

1. **Path A** (comb → eigenvector delocalization → macroscopic S_N(2)) was **refuted** by
   block Jacobi elimination (see `jacobi_comb_operator.kleis`).
2. **Conrey-Keating** route formalized: RH + ratios conjecture → twin primes (two assumptions).
3. **Direct route** (contraction mixing → |P(x)| = o(x)) was **refuted** by the spectral
   comb paper's own numerical analysis (ε → 0, J_F → I, zeros decouple). March 2026.
4. **Reductio ad absurdum** attempted but **removed** — circular argument. March 2026.

#### The Conrey-Keating route (only surviving forward route)

```
RH (proved) + Ratios Conjecture (assumption — CFZ 2008)
  → Conrey-Keating 2016 theorem → Hardy-Littlewood → twin primes
```

This is a **two-assumption** result. The ratios conjecture remains an external assumption.

#### Files

| File | Purpose |
|------|---------|
| `examples/mathematics/twin_prime_correlation.kleis` | All routes (21 Z3 examples) |
| `examples/mathematics/jacobi_comb_operator.kleis` | Path A refutation |

#### Next steps

1. **Path B (ITCM):** The heat kernel / integral transform route bypasses the comb's internal structure entirely. Remaining independent forward route.
2. **Accept the conditional result:** The Conrey-Keating route is clean, well-formalized, and the only live forward chain.

#### Key technical lessons (twin primes)

- Block Jacobi elimination converts a hypothesis-failing scalar comb into an effective operator that DOES satisfy Yafaev's hypotheses
- Eigenvector localization is physical: each eigenvalue "lives" on its peak pair
- S_N(2) → 0 but is structured: spectral pair sum is a separate eigenvalue-only observable
- Skolem element mismatch in Kleis/Z3: axioms for generic elements don't propagate to concrete literals
- Bool sort mismatch: `is_prime(4) = false` fails — use `not(is_prime(4))`

---

### NEXT STEPS: Pressure Hessian Analysis

The next theorem-shaped target is to:

1. **Write down the strain evolution explicitly:**
   DS/Dt = -(S² + W²/4) - H_tf + ν∇²S (where H_tf = trace-free pressure Hessian)

2. **Isolate the off-diagonal component M₁₂ = e₂·(DS/Dt)e₁:**
   - S² contribution: 0 (diagonal in eigenframe)
   - W² contribution: -√α₁√α₂|ω|² (uses Wᵢⱼ = -εᵢⱼₖωₖ)
   - H_tf contribution: e₂·H_tf·e₁ (nonlocal, from Poisson equation)

3. **Determine the sign of M₁₂/(λ₁-λ₂):**
   - W² gives POSITIVE contribution (drives alignment UP)
   - Pressure Hessian H_tf must overcome this
   - Connects to "restricted Euler" vs "full Euler" distinction (Vieillefosse 1982)

4. **Handle eigenvalue-gap degeneracy:**
   - When λ₁ ≈ λ₂: |De₁/Dt| can diverge, but α₁ and α₂ become interchangeable
   - Need to show gap collapse events don't accumulate harmful sign

5. **Candidate formalization in Kleis:**
   - Encode W² contribution as Z3 axiom (exact)
   - Encode H_tf contribution as bounded unknown
   - Test whether known pressure Hessian bounds force Re < 0

---

### Next Paper Candidates (Volume V options)

#### Option B: Aharonov-Bohm as Kernel Non-Surjectivity

**Thesis:** On topologically nontrivial manifolds, the EM kernel d is not surjective onto closed 2-forms. The gap (H²_dR ≠ 0) produces observable effects even where F=0.

**Risk:** Moderate. Well-understood physics, clean kernel interpretation.

#### Option C: Admissibility Restoration via Additional Fields

**Thesis:** Can additional degrees of freedom restore effective admissibility to a non-admissible kernel? Derive the mechanism structurally, then identify whether it corresponds to spontaneous symmetry breaking.

**Risk:** High. Natural sequel to Volume IV.

#### Option D: Mass Gap as Topological Obstruction on the Nullspace Variety

**Seed idea (from Gemini review of Volume IV):** Reframe the mass gap from "lowest eigenvalue of QCD Hamiltonian?" to "spectral gap of the Laplacian on the nullspace variety?" Uses Cheeger inequality, Lichnerowicz theorem.

**Risk:** Very high. Requires differential geometry on the moduli space.

#### Option E: The Standard Model Gauge Sector — SU(3) × SU(2) × U(1)

**Risk:** Very high. Requires careful treatment of spontaneous symmetry breaking.

---

## Active Engineering

### ADR-035: Multi-Domain Equation Editor — DEEP ANALYSIS COMPLETE

**Status:** Plan finalized, ready to implement.  
**Branch:** TBD (needs new branch off main)  
**Plan file:** `.cursor/plans/domain-agnostic_editor_engine_f0f86c94.plan.md`

#### Goal

Extending the Equation Editor to new domains (Egyptian hieroglyphs, circuits,
chemistry) must require only `.kleist` template files and `.kleis` theory
files — no Rust, no JavaScript changes. Existing math code stays as-is.

#### Deep analysis findings (discrepancies between ADR-035 and actual code)

1. **`validate_quadrat` does NOT exist** in `render_editor.rs` or any Rust
   source file. ADR-035 incorrectly claimed it needed replacement. The only
   domain-specific code that was added during the Egyptian experiment has
   already been reverted.

2. **Egyptian-specific JS does NOT exist** in `index.html`.
   `showEgyptianPalette`, `insertEgyptianGlyph`, `validateQuadratPlacement`
   — all absent. ADR-035 incorrectly claimed these needed removal.

3. **`kleist_parser.rs` DOES need changes.** ADR-035 claimed "no changes
   needed" because it assumed the parser collects unknown metadata fields.
   In fact, `TemplateDefinition` (line 42-57) has NO `metadata` field, and
   the parser's catch-all arm (line ~620) ERRORS on unknown `identifier:
   "value"` pairs. This is a blocker for `mode:`, `slot_type:`, `accepts:`.

4. **`collect_editor_slots_recursive`** in `server.rs` (line ~1360)
   explicitly skips `EditorNode::Operation`. Zero-arg operations get no
   `ArgumentSlot`, no UUID, no bounding box. This is the root cause of
   "Argument Bounding Boxes (semantic): (none)" for glyphs.

5. **UUID wrapping** at 4 sites in `render_editor.rs` uses
   `#[#box[$CONTENT$]<idUUID>]` — always math mode. Content-mode Typst
   (like `#image(...)`) breaks inside `$...$`.

#### Type inference pipeline in the Equation Editor

Traced the full chain for how HM unification works in the visual editor:

```
Client: checkTypes() → POST /api/type_check (debounced 500ms)
    ↓
Server: type_check_handler (server.rs:1393)
    ↓
    Stage A: json_to_editor_node → EditorNode AST
    Stage B: is_formatting_operation → early return for subsup/tilde/hat/etc.
    Stage C: find_tensor_in_editor_node → early return for kind:"tensor"
    Stage D: editor_node_to_expression → flatten to Kleis Expression
    Stage E: TypeChecker::with_stdlib() → load matrices/tensors/quantum
    ↓
TypeChecker.check() → inference.infer_and_solve(expr, context_builder)
    ↓
infer_operation (type_inference.rs:1339):
    - ~400 lines of hardcoded match blocks (Matrix, Complex, Rational, etc.)
    - Arithmetic ops (plus/minus/times/divide): generic NatValue matching
      → catches Matrix(3,3) + Matrix(2,2) at line 1670
    - Final fallback: context_builder.infer_operation_type()
    ↓
TypeContextBuilder.infer_operation_type (type_context.rs:1031):
    - Looks up operation in structure registry
    - Delegates to SignatureInterpreter for HM unification
    - matrix_add(Matrix(3,3,ℝ), Matrix(2,2,ℝ)) fails because
      signature Matrix(m,n,T)→Matrix(m,n,T)→Matrix(m,n,T) forces m=3,m=2
```

**Key insight:** `EditorTypeTranslator` (src/editor_type_translator.rs) is
already generic — reads `kind` and `metadata` from EditorNodes without
hardcoding types. New domains work through the same pipeline.

#### Middle Egyptian paper insight: "Type inference IS translation"

From "The Scribe is the Skolem" (Section 6):

- The 125 axioms in `stdlib/theories/middle_egyptian_grammar.kleis` are
  structurally identical to `stdlib/matrices.kleis`
- `MiddleEgyptianNominalGrammar` defines operations like
  `gender : Noun → Gender`, `adjective_gender : Adjective → Gender`
- Axiom 53 (`adjective_agreement`):
  `modifies(a, n) → adjective_gender(a) = gender(n)`
  — same constraint pattern as matrix dimension matching
- The TypeChecker + SignatureInterpreter handles this through the same
  HM unification path — no Rust changes needed for type checking
- For the Equation Editor, Egyptian type checking means loading
  `middle_egyptian_grammar.kleis` into the TypeChecker alongside matrices
- Template `.kleist` files specify both rendering AND type info (`kind`,
  `metadata`) — same template that draws a glyph tells the type system
  what grammatical role it fills

#### render_editor.rs audit

Six categories of hardcoded template knowledge found:

1. Matrix dispatching: `"Matrix"|"PMatrix"|"VMatrix"|"BMatrix"` + fixed-size
2. Piecewise dispatching: `"Piecewise"|"cases2"|"cases3"|"cases"`
3. Tensor dispatching: `kind == "tensor"` → `render_tensor`
4. Variadic folding: hardcoded list `["times","scalar_multiply","multiply","plus","minus"]`
5. Name-specific substitutions: `if name == "index_mixed"`, `if name == "int_bounds"`
6. ~250 lines of fallback templates in `EditorRenderContext::new()`

**Decision:** All of this is existing math infrastructure — it stays as-is.
New domains go through the generic `render_with_template` path (line 1474),
which already handles arbitrary templates via positional placeholder
substitution.

#### Verify/Sat button pipeline

Traced how the Verify and Sat buttons work:

```
Client: verifyWithZ3() / checkSatisfiable()
  → POST /api/verify or /api/check_sat with currentAST
    ↓
Server (verify_handler / check_sat_handler):
  1. json_to_editor_node → parse AST
  2. extract_types_from_editor_node → EditorTypeTranslator reads kind/metadata
  3. editor_node_to_expression → flatten to Kleis Expression
  4. AxiomVerifier.new(registry) → Z3 backend with StructureRegistry
  5. analyze_dependencies(expr) → walks Expression, finds operation names,
     looks up owning structures via registry.get_operation_owners()
  6. ensure_structure_loaded(structure) → loads axioms into Z3
  7. verify_axiom or check_satisfiability → Z3 check
```

**Key finding:** `load_stdlib_registry()` (server.rs line 96) loads from a
hardcoded list of 3 files: `minimal_prelude.kleis`, `lists.kleis`,
`matrices.kleis`. Theory files like `middle_egyptian_grammar.kleis` are NOT
loaded. The `AxiomVerifier` and `analyze_dependencies` are fully generic —
they would work for any domain IF the structures are in the registry.

**Solution:** Templates should declare their theory imports. A `.kleist` file
should have `@import "stdlib/theories/middle_egyptian_grammar.kleis"`. When
the server loads templates for the palette, it also loads the imported
theories into the `StructureRegistry` and `TypeChecker`. No hardcoded list
of files. The template is the single source of truth for rendering, type
info, palette placement, AND the theory needed for verification.

**Note:** `ensure_structure_loaded` (line 361) skips parameterized structures
to avoid Z3 explosion. `MiddleEgyptianNominalGrammar` and
`HieroglyphicWriting` are non-parameterized, so they load correctly.

#### Five engine fixes (one-time, domain-agnostic)

1. **Parser metadata** — DONE. Added `metadata: HashMap<String, String>` to
   `TemplateDefinition`, parser catch-all collects unknown `key: "value"` pairs.
2. **Zero-arg slots** — DONE. Zero-arg Operations produce ArgumentSlots in
   `collect_editor_slots_recursive`.
3. **Mode-aware UUID wrapping** — DONE. `uuid_wrap` helper reads `mode`
   from template metadata. Egyptian templates use `mode: "content"` to skip
   `$...$` math-mode wrapping in Typst output.
4. **Data-driven palettes:**
   - **4a: `/api/palette` endpoint** — DONE. Serves palette structure with
     ASTs and metadata from `.kleist` files.
   - **4b: Client-side `buildPaletteFromAPI()`** — DEFERRED. Currently the
     Egyptian palette uses domain-specific JavaScript (~300 lines in
     `static/js/egyptian.js`). This violates the "no JS for new domains"
     goal. Needs replacement with a generic `domainPalette.js` that reads
     from `/api/palette` and constructs tabs, groups, filters, and buttons
     from data. Any domain would get its palette automatically.
5. **Theory imports from templates** — DONE. `.kleist` files declare
   `@import` for `.kleis` theories. Server loads imported theories into
   `StructureRegistry`.

#### ES Module Extraction — DONE

**Branch:** `refactor/es-module-extraction`
**PRs:** eatikrh/kleis#64, engingithub/kleis#62

Extracted ~3,100 lines of inline JavaScript from `static/index.html` into
17 ES module files under `static/js/`. The HTML file is now 1,232 lines
(markup + CSS only), loading `<script type="module" src="/static/js/main.js">`.

Modules: `state.js`, `astUtils.js`, `render.js`, `undoRedo.js`,
`inlineEdit.js`, `slotHandlers.js`, `palette.js`, `egyptian.js`,
`matrixBuilder.js`, `piecewiseBuilder.js`, `debug.js`, `verify.js`,
`modeConvert.js`, `jupyter.js`, `keyboard.js`, `gallery.js`, `main.js`.

Manually tested: all palettes, structural mode, inline editing, piecewise
builder, matrix builder, type checking, verify/sat, Egyptian glyphs. No
console errors. 97 AST templates loaded.

#### Next: Generic Domain Palette (Fix 4b)

**Goal:** Replace `egyptian.js` with a generic `domainPalette.js` that works
for ANY domain — no JavaScript changes needed to add new template domains.

**Approach:** Add a second concrete domain first (electronics), observe what
filter dropdowns it needs, then generalize.

**Electronics `.kleist` template example:**
```
@template resistor
  name: "resistor"
  category: "electronics_passive"
  svg: "static/svg/electronics/resistor.svg"
  metadata:
    component_type: "Passive"
    package: "Through-hole"
    symbol_standard: "IEC"
```

**Electronics filter dropdowns:**
- Component type: Passive, Active, IC, Connector, Source
- Package: Through-hole, SMD, Module
- Symbol standard: IEC, ANSI

**Egyptian filter dropdowns (existing):**
- Sign shape: Tall, Flat, Small
- Sign type: Uniliteral, Biliteral, Triliteral, Determinative

**Key insight:** The pattern is identical — for each unique metadata key
across templates in a domain, create a filter dropdown whose options are
the distinct values for that key. The generic `domainPalette.js` would:

1. Fetch templates by domain from `/api/templates`
2. Scan metadata keys across all templates in the domain
3. Build filter dropdowns dynamically from distinct metadata values
4. Generate glyph/component buttons with SVG/Unicode rendering
5. Handle composition templates (quadrats, series/parallel circuits)

**Steps:**
1. Create `std_template_lib/electronics.kleist` with ~20 basic components
2. Implement `static/js/electronics.js` (hardcoded, like `egyptian.js`)
3. Compare `egyptian.js` and `electronics.js` side-by-side
4. Extract the common pattern into `domainPalette.js`
5. Delete both domain-specific files, replace with data-driven generic

#### Known issue: `apply_template_substitutions` hardcoded aliases

`render_editor.rs` function `apply_template_substitutions` (line ~2109) uses
a hardcoded list of placeholder name aliases (`{left}`, `{right}`, `{body}`,
`{exponent}`, etc.). Template authors MUST use names from this list, or the
placeholders render as literal text. For example, `{top}` and `{bottom}` are
NOT recognized — the quadrat_v template had to use `{left}` and `{right}`
instead.

**Fix needed:** Parse argument names from the template `pattern` field (e.g.,
`quadrat_v(top, bottom)` → positional map `{top}→arg[0]`, `{bottom}→arg[1]`)
and apply those substitutions BEFORE the hardcoded aliases. This is a one-time
Rust change that enables template authors to use any argument names they want.

#### Template testing results (manually verified)

- Math rendering: matrix, piecewise, fraction, integral — no regressions
- Egyptian quadrat_v and quadrat_h: glyphs render with uniform 1em sizing
- Interactive overlays: working for both math and Egyptian templates
- All 2440 tests pass

#### Additional finding: EditorRenderContext needs metadata map

`EditorRenderContext` (render_editor.rs line 83) stores only per-target
template strings (unicode, latex, html, typst, kleis). It discards all other
template fields. For `uuid_wrap` to read `mode` from template metadata,
`EditorRenderContext` needs a new `metadata: HashMap<String, HashMap<String,
String>>` map keyed by template name.

#### CRITICAL FINDING: Editor AST ≠ Kleis AST for non-math domains

The Editor AST and Kleis AST are two DIFFERENT representations:
- **Editor AST** (`EditorNode`): visual composition — how things are laid out
  - Operations: `quadrat_h`, `egyptian_glyph_A1`, `plus`, `frac`
- **Kleis AST** (`Expression`): semantic operations — what things mean
  - Operations: `gender`, `modifies`, `matrix_add`, `transpose`

For **math**, this gap is bridged by HARDCODED logic in `type_inference.rs`
(lines 1637-1640). Operations `plus`, `minus`, `times`, `divide` are matched
by name and checked inline. They never go through `StructureRegistry`.

For **Egyptian**, there is NO bridge:
- Editor operations are **visual** (`quadrat_h` = side-by-side layout)
- Kleis structure operations are **grammatical** (`gender : Noun → Gender`)
- `editor_node_to_expression` maps names 1:1 — `quadrat_h` stays `quadrat_h`
- `analyze_dependencies` can't find `quadrat_h` in any structure
- `MiddleEgyptianNominalGrammar` defines `gender`, `modifies`, etc. — but
  the editor never produces expressions using those operation names

This means the five engine fixes handle **rendering** correctly, but
Verify/Sat buttons for Egyptian require an additional translation layer
from visual composition to semantic meaning (glyph → linguistic entity →
grammatical operation). This is the "reading" step from the paper.

**Translation layer exists but is naive:** `editor_node_to_expression` in
`server.rs` (line 1246) is the bridge between the two ASTs. Currently it does
a 1:1 name copy — `operation.name` passes through unchanged. For math, this
works because editor names (`plus`, `frac`) are handled by hardcoded logic in
`type_inference.rs`. For non-math domains, this layer would need to become
smarter — translating visual composition operations into semantic expressions
that the grammar structures understand.

**Implication for Fix 5 (@import):** Loading theory files into the registry
is necessary but NOT sufficient for Verify/Sat. The registry would contain
`gender`, `modifies`, etc., but `analyze_dependencies` would look for
`quadrat_h`, `egyptian_glyph_A1` — and find nothing.

**DECISION: Verify/Sat for non-math domains needs its own design phase.**
The five rendering/palette engine fixes should proceed. Making Verify/Sat
work for Egyptian and other non-math domains requires designing how
`editor_node_to_expression` maps visual composition to semantic operations,
and this is a separate effort. Fix 5 (@import) is still useful — it enables
the registry to have the theories available — but the translation layer
needs additional design work before Verify/Sat buttons produce meaningful
results for non-math domains.

#### Risk: top-level Typst wrapper

`typst_compiler.rs` wraps everything in `#box($ ... $)` — math mode. For
content-mode templates, the per-template `uuid_wrap` uses `#[#box[CONTENT]<id>]`
(no `$...$`). This relies on Typst's `#[...]` switching from math to content
mode inside `$...$`. Needs experimental verification during implementation.

---

### Graph Editor — IMPLEMENTED (Phases 1–9)

**Status:** Phases 1–9 complete. Domain-agnostic routing, parameters, Z3 verification, theory-driven simulation, and chunked continuous trajectory all working.
**ADR:** `docs/adr/ADR-037-Graph-Editor-Domain-Agnostic-Routing.md`
**Plan file:** `.cursor/plans/domain-agnostic_multi-port_routing_83985610.plan.md`

#### Architecture (settled)

A graph is a **value inside the AST**, not an alternative to the AST. The
**signed sparse port-based** incidence matrix IS the AST's way of expressing
graph topology:

```
graph(
  SparseMatrix(V, P, [net0, port0, val0, net1, port1, val1, ...]),
  [components...],                // component types
  [net_labels...],                // net names
  [port_labels...]                // "componentIdx:portName" per column
)
```

Columns represent individual ports, not components. A transistor (3 ports) gets
3 columns. Port identity is the column. The port_labels list maps each column
back to its component and port name, making the representation complete.

**Storage format:** COO (Coordinate List) — only non-zero entries stored as
`(net_index, port_index, value)` triples, flattened into a single list. Entries
are signed integers: `+1` for the first port of a component (source / positive),
`-1` for non-first ports (sink / negative). Higher magnitudes encode bond order
or weight. For undirected domains the signs are algebraic bookkeeping; for
directed domains (bond graphs, Petri nets) they encode physical flow direction.
Dense V x P matrix is materialized from COO only when needed (display, axioms).

The Graph Editor is a **standalone sibling** to the Equation Editor
(`static/graph_editor.html` + `static/js/graphEditorMain.js`). This avoids
shoehorning graph-type UI into tree-based equation UI. Both share the same
data model (EditorNode AST), domain data (`.kleist`/`.kleis`), and server APIs.

#### What's built (Phase 1)

**Canvas & interaction:**
- [x] SVG `viewBox`-based pan/zoom (scroll wheel, middle-click/Space+click drag)
- [x] Grid snapping (20px), component placement from palette
- [x] Component selection, dragging, rotation (90° increments), deletion
- [x] Port-to-port connection by click-drag
- [x] Interactive net selection and deletion

**Wire routing (domain-agnostic):**
- [x] Exit-direction-aware Manhattan routing for 2-port nets
- [x] Trunk+branch routing for multi-port nets (3+ connections)
  - Picks trunk pair by greatest distance
  - Projects branch ports onto trunk, routes perpendicular stubs
  - Junction dots at T-junction points
- [x] Persistent waypoints, draggable segments/waypoints
- [x] Waypoint add (double-click segment) / remove (double-click waypoint)
- [x] Collinear segment merging, zero-length segment collapse
- [x] Auto-rerouting connected nets when components are dragged
- [x] Live preview during connection with exit-direction awareness

**Domain configuration (`.kleist`-driven):**
- [x] `@template __domain_electronics` block in `electronics.kleist`
- [x] `domainConfig` object loaded from `__domain_` template metadata at startup
- [x] Routing mode dispatch: `orthogonal` (Manhattan) vs `direct` (straight line)
- [x] Multi-port strategy dispatch: `trunk_branch` vs `star` (centroid)
- [x] Junction style: `dot` (filled circle), `none` (no marker)
- [x] Defaults when no `__domain_` template: orthogonal, dot, trunk_branch, none, undirected

**Output:**
- [x] EditorNode AST (`graph(SparseMatrix(...), ...)`) — pure JS
- [x] Typst schematic export (`#place`, `#line`, `#image` with rotation)
- [x] Signed sparse incidence matrix display (COO stored, dense materialized for table view)

**Implementation:**
- [x] Pure JS for incidence matrix and AST generation (~55 lines)
- [x] `graph-editor-wasm/` crate kept as scaffold for future heavy computation

#### Domain config fields

| Field | Values | Default |
|-------|--------|---------|
| `routing_mode` | `"orthogonal"`, `"direct"`, `"curved"` (future) | `"orthogonal"` |
| `junction_style` | `"dot"`, `"none"`, `"bar"` | `"dot"` |
| `multi_port_strategy` | `"trunk_branch"`, `"star"`, `"bus"` (future) | `"trunk_branch"` |
| `edge_decoration` | `"none"`, `"arrow"`, `"half_arrow"`, `"inhibitor"` | `"none"` |
| `edge_direction` | `"undirected"`, `"directed"` | `"undirected"` |

#### To add a new graph domain

1. Create `std_template_lib/<domain>.kleist` with component templates
   (each needs `ports:`, `graph_width:`, `graph_height:` metadata and an SVG)
2. Add `@template __domain_<name>` block with routing preferences
3. Place SVG assets in `static/svg/<domain>/`
4. No JavaScript, no Rust, no recompilation

#### Files

| File | Role |
|------|------|
| `static/graph_editor.html` | Graph Editor HTML + CSS |
| `static/js/graphEditorMain.js` | Core editor: interaction, routing, rendering (~1644 lines) |
| `std_template_lib/electronics.kleist` | 20 electronic components + `__domain_electronics` config |
| `std_template_lib/bond_graph.kleist` | 9 bond graph elements + `__domain_bond_graph` config |
| `std_template_lib/petri_net.kleist` | Place + transition templates + `__domain_petri_net` config |
| `graph-editor-wasm/` | Rust/WASM crate (scaffold, not in active code path) |
| `static/svg/electronics/` | SVG assets for electronic components |
| `static/svg/bond_graph/` | SVG assets for bond graph elements |
| `static/svg/petri_net/` | SVG assets for Petri net elements |

#### Remaining phases

**Phase 2: Edge decorations and direction** — DONE
- SVG marker definitions (arrowhead, half-arrow, inhibitor, causal bar)
- `marker-start`/`marker-end` attributes based on `edge_decoration`
- Typst export emits `#polygon` arrowheads for directed edges
- `DECORATION_MARKER_ID` lookup table maps config names to marker IDs

**Phase 3: Bond graph templates + direct routing** — DONE
- `std_template_lib/bond_graph.kleist` with 9 elements (Se, Sf, R, C, I, TF, GY, 0-junction, 1-junction)
- `__domain_bond_graph` config: `routing_mode: "direct"`, `edge_decoration: "half_arrow"`, `edge_direction: "directed"`
- 9 SVG assets under `static/svg/bond_graph/`
- `?domain=X` URL parameter filters palette and domain config (e.g. `?domain=bond`, `?domain=electronics`)
- Direct routing fixes: segment drag, dbl-click split, waypoint drag disabled; cursor classes corrected
- Per-net causal stroke: `net.causal = 'start' | 'end' | null`, toggled with `K` key
- `causality_type` metadata on each template for future SCAP algorithm
- Typst export includes causal bar rendering

**Phase 4: Petri net / workflow net templates** — DONE
- `std_template_lib/petri_net.kleist` with 4 component types + `__domain_petri_net` config
- Source place (`◉` circle with inner dot) — workflow start, carries initial token
- Regular place (`○` empty circle) — intermediate state / condition
- Sink place (`◎` double circle) — workflow end, proper completion target
- Transition (`▮` filled bar) — activity / event / task
- `edge_decoration: "arrow"`, `edge_direction: "directed"`, `routing_mode: "orthogonal"`
- SVGs: `static/svg/petri_net/{place,source_place,sink_place,transition}.svg`
- Template header documents workflow net = BPMN equivalence and soundness property
- component_type metadata: `SourcePlace`, `Place`, `SinkPlace`, `Transition`
- **Deferred items:**
  - Token rendering inside places (needs Phase 5 component parameters: `pn_place(tokens=3)`)
  - Inhibitor arcs (need per-arc decoration support; SVG marker already exists from Phase 2)
  - Arc weight labels (need per-arc metadata; ties into signed incidence matrix weights)
  - VERIFY / SAT buttons (see Phase 6)

**Petri net ↔ BPMN mapping** (reference for VERIFY implementation):

| BPMN Element   | Workflow Net Element           | Kleis Template       |
|----------------|-------------------------------|----------------------|
| Start event    | Source place (initial token)   | `pn_source_place`    |
| End event      | Sink place                    | `pn_sink_place`      |
| Activity/Task  | Transition                    | `pn_transition`      |
| Condition      | Place                         | `pn_place`           |
| XOR split      | Place with multiple out-arcs  | (topology, not type) |
| AND split      | Transition with multiple outs | (topology, not type) |
| Sequence flow  | Directed arc                  | (wire with arrow)    |

**Phase 5: Component parameters + structural VERIFY** — DONE
- `params` metadata in `.kleist`: `"name:type:default"` semicolon-separated (like `ports`)
- Petri nets: `pn_source_place(tokens:int:1)`, `pn_place(tokens:int:0)`, `pn_sink_place(tokens:int:0)`
- Electronics: `resistor(R:real:1000)`, `capacitor(C:real:1e-6)`, `inductor(L:real:1e-3)`,
  `dc_voltage(V:real:5)`, `ac_voltage(V:real:120;freq:real:60)`, `dc_current(I:real:0.01)`
- `loadComponentDefs()` now stores `componentType`, `causalityType`, parsed `params` array
- `graphState.components[]` carries `params: {name: value}` initialized from defaults
- Property panel: editable inputs for each param when component selected
- AST encoding: `resistor(1000)` not `resistor()` — params become operation args
- Incidence matrix sign convention: connection index 0 = +1 (source), others = -1 (target)
  — encodes arc direction from user click order
- **VERIFY button**: generic data-driven structural checks from `verify_*` domain metadata
  - `verify_bipartite`, `verify_exactly_one`, `verify_requires_type`, `verify_no_isolated`,
    `verify_all_connected`, `verify_causality` — all read from `__domain_*` template metadata
  - No domain-specific JS functions. New domains add rules as `.kleist` metadata only.
  - Results shown in overlay panel with pass/fail per check

  **Decisions made:**
  - Arc weights: deferred — all arcs weight 1 for now. Most Petri nets use unit weights.
  - Token rendering: deferred to simulation phase. Initial marking is a parameter (property
    panel), not a visual overlay. Dots inside places only make sense during step-through
    simulation when tokens move between places.
  - Graph operations for Z3: generic primitives (`graph_incidence`, `graph_param`,
    `graph_component_type`). Server provides domain-agnostic graph data; companion `.kleis`
    theory derives domain semantics. No domain-specific Rust code.

  **Parameter type system (data-driven, no domain JS):**

  | Type | Example | UI widget | AST encoding |
  |------|---------|-----------|-------------|
  | `real` | R=1000 | number input | operation arg |
  | `int` | tokens=3 | number input (step=1) | operation arg |
  | `enum` | element=C | dropdown (future) | operation arg |
  | `ref` | model=2N2222 | dropdown from server (future) | operation arg (name) |

  **Separation of duties — Graph Editor never reads .kleis theory files:**
  - Template metadata (params, ports) comes from `/api/templates` (server reads `.kleist`)
  - Available models for `ref` params come from a new `/api/models?structure=X` endpoint
    (server reads `.kleis` theory, returns list of `define` names matching the structure)
  - Structural verification: client-side JS reading `verify_*` metadata
  - Deep Z3 verification: server-side via companion `.kleis` theory (Phase 6)

**Phase 6: Z3 verification via companion `.kleis` theory** — DONE
- Companion `.kleis` file convention: `std_template_lib/petri_net.kleis` next to
  `petri_net.kleist`. Domain config references it via `verify_theory: "petri_net"`.
- **CRITICAL ARCHITECTURAL DECISION: `server.rs` contains ZERO domain-specific code.**
  The server emits only generic graph primitives; companion `.kleis` theories derive
  all domain semantics from those primitives.
- Server `POST /api/verify_graph` endpoint:
  1. Load companion `.kleis` theory from domain config `verify_theory`
  2. Generate **domain-agnostic** preamble from graph data (see below)
  3. Concatenate preamble + theory, parse, evaluate, run examples via Z3
  4. Return per-example pass/fail results as JSON
  5. Use `tokio::task::spawn_blocking` (Z3 thread-local context safety)
- **Domain-agnostic preamble** — `build_graph_preamble(req, theory_source)` emits:
  - `graph_nc`, `graph_nn` — component/net counts
  - `graph_ctype(c)` — integer type code per component (auto-assigned from component_type strings)
  - `graph_param(c, j)` — positional parameters from component params
  - `graph_inc(net, comp)` — component-level incidence matrix (port-level entries aggregated)
  - `TYPE_X` constants — one per unique component_type, with distinctness/positivity axioms
  - **Closed-world axioms** for `ctype`, `inc`, and `param` (prevents Z3 from inventing values)
  - **Theory scanning** for `TYPE_X` references: assigns unused codes to types the theory
    references but the graph doesn't contain (prevents Z3 from equating undefined TYPE constants
    with existing ones)
- Companion theory `petri_net.kleis` derives domain semantics entirely from generic primitives:
  `is_place(c)`, `is_transition(c)`, `is_source(c)`, `is_sink(c)`, `tokens(c)` as `define`
  statements, plus `example` blocks for INITIAL MARKING, BIPARTITE, SOURCE EXISTS, SINK EXISTS.
- **23 Rust tests** covering: preamble structure (type codes, params, incidence, closed-world,
  distinctness, no domain-specific ops), Z3 pass/fail cases (linear workflow, no tokens, no sink,
  non-bipartite, empty graph, fork, join, mutex), missing theory errors.
- **Manually tested end-to-end**: Graph Editor → VERIFY button → 5 JS structural checks +
  4 Z3 verified examples → all green.
- **BMC removed**: Bounded Model Checking (BFS reachable states) was initially prototyped in
  server.rs but removed because it contained domain-specific Petri-net code. Structural
  verification via Z3 quantifiers is sufficient for current checks. If BMC is needed later,
  it should be a generic graph analysis module, not embedded in server.rs.
- **ADR-037 updated** to Phases 1–5 with full verification architecture documented.

  **Phase 7 (planned, new feature branch): Causal Network Verification Theories**

  Electronics and bond graphs share verification rules because both are instances
  of the **effort/flow duality** from network thermodynamics:

  | Concept | Electronics | Bond Graphs | General |
  |---------|------------|-------------|---------|
  | Effort variable | Voltage (V) | Effort (e) | Across variable |
  | Flow variable | Current (I) | Flow (f) | Through variable |
  | Source conflict | 2 voltage sources in parallel | 2 effort sources on same 0-junction | Conflicting effort constraints |
  | Source conflict | 2 current sources in series | 2 flow sources on same 1-junction | Conflicting flow constraints |
  | Short circuit | Voltage source shorted | Effort source with zero impedance path | Zero-resistance effort loop |

  **Layered theory architecture:**

  ```
  std_template_lib/causal_network.kleis     ← shared effort/flow conflict rules
  std_template_lib/electronics.kleis        ← imports causal_network, adds KVL/KCL
  std_template_lib/bond_graph.kleis         ← imports causal_network, adds causality assignment
  ```

  The shared `causal_network.kleis` would define generic rules derivable from
  the graph primitives (`graph_nc`, `graph_ctype`, `graph_inc`, `graph_param`):
  - No two effort sources on the same node
  - No two flow sources in the same loop
  - Every node must have at least one path to a reference
  - Source/sink balance

  Domain-specific theories add their own:
  - **Electronics**: Kirchhoff's voltage/current laws, component equations (V=IR, I=CdV/dt),
    short-circuit detection as SAT query
  - **Bond graphs**: SCAP causality assignment, 0-junction (common effort) and 1-junction
    (common flow) constraints, causality conflict detection

  This layered approach uses Kleis's structure system: shared axioms in a base structure,
  domain specialization via extension — same pattern as `stdlib/matrices.kleis` extending
  `minimal_prelude.kleis`.

  **Phase 8 (DONE): Theory-Driven Simulation**

  Simulation logic moved from hardcoded Petri-net Rust in `server.rs` to `.kleis`
  theory files, mirroring the verification architecture. Key changes:

  - `petri_net.kleis` now contains simulation functions: `sim_enabled(t)`,
    `sim_fire(t, c)`, `sim_halted()`, `sim_halt_reason()` — all using recursive
    enumeration over `graph_nc_val`/`graph_nn_val` with `nth`-based lookup.
  - `build_sim_preamble()` generates concrete `define` statements (not Z3 axioms)
    for `eval_concrete` execution.
  - Server `simulate_graph_core()` calls `eval_concrete` on theory-defined functions.
  - Multi-step `Run` optimized: re-parses only `define sim_state = [...]` between steps.
  - Reset uses client-provided initial state (from `.kleist` `stateParam` metadata).
  - Client sends `last_fired` for round-robin transition selection.
  - Bug fix: `"logical_and"`/`"logical_or"` aliases added to `eval_concrete` builtins.
  - 9 tests pass including 5-token pipeline completing in 10 steps.
  - **To add simulation for a new domain:** implement `sim_enabled`/`sim_fire`/
    `sim_halted`/`sim_halt_reason` in the companion `.kleis` theory file. No Rust
    or JS changes needed.

  **Phase 9 (DONE): Buffered Trajectory Simulation for Continuous Domains**

  Implemented chunked trajectory simulation for continuous domains. One API call
  returns N timesteps at once via `chunk_size` parameter on `SimulateGraphRequest`.
  `simulate_graph_core` loops `for step in 0..chunk_size`, calling theory-owned
  `sim_step(i)` per state variable, building `time_series: Vec<SimulateTimeSample>`.
  `sim_time` tracks and increments across chunks. Tests verify: bond graph RC
  circuit (1000 steps), electronics rectifier (100 steps), multivibrator (100 steps).

  **Phase 10 (planned): Graph Theory Domain & Königsberg Demo — arXiv Paper**

  The graph theory domain was added as an architecture validation — a fourth domain
  (after electronics, bond graphs, Petri nets) implemented with **zero code changes**:
  only 3 data files (`graph_theory.kleist`, `graph_theory.kleis`, `node.svg`).

  **Königsberg bridges demo:**
  - 4 graph_node components (landmasses), 7 edges (bridges)
  - Z3 verification catches parallel edges (SIMPLE GRAPH fails)
  - Demonstrates: incidence matrix → preamble → companion theory → Z3 result
  - All 4 nodes have odd degree → no Eulerian path (Euler 1736)

  **arXiv paper** planned to document the Graph Editor architecture:
  1. The problem: visual graph editors are domain-locked
  2. The architecture: incidence matrix + domain-agnostic preamble + companion theories
  3. The demo: Königsberg in 3 files, zero code changes
  4. The generality: same architecture for Petri nets (BPMN), electronics, bond graphs, graph theory
  5. The verification: Z3 checks domain-specific properties from data-driven theories
  6. UX: human-readable verification results (translate Z3 counterexamples to domain terms)

  **Verification UX improvement** (for paper and product):
  - Current: raw Z3 counterexamples (`n1 = 0, n2 = 0, c = 0`) — meaningless to users
  - Needed: interpretive layer mapping variable indices back to component/net labels
  - Positive framing: "MULTIGRAPH: parallel edges found between Node A and Node B"
    instead of "SIMPLE GRAPH failed"

  **Eulerian path check** requires bounded aggregation (counting incident edges mod 2).
  Planned as a Kleis language feature — would enable `degree(c)` and parity checks.

  **Short circuit detection — expressible now via matrix rank:**

  A short circuit exists iff the connector-only submatrix of `graph_inc` has a
  rank deficiency that places both terminals of a voltage source in the same
  connected component. This is a matrix rank condition, not a graph traversal.
  Kleis already has matrix operations and LAPACK integration — `rank()` on a
  filtered submatrix is a standard call. No new built-in needed; the check is
  an `example` assertion in `electronics.kleis` using existing primitives.

  The same linear algebra applies to the ODE formulation: the state-space matrix
  `M · dX/dt = A·X + B·u` derived from the incidence matrix becomes singular at
  a short circuit. The singularity is visible in the matrix before the solver
  even runs.

  **Theory interface declarations — DONE (PR #70):**

  Theories now explicitly declare `operation TYPE_X : ℤ` for each type they
  reference. The preamble generator parses the theory AST and extracts
  `OperationDecl` items instead of scanning text. See ADR-037 Section 7.

#### Still open

- ~~**Visual style mismatch with Equation Editor**~~ — DONE. Graph Editor
  restyled to match the Equation Editor's light theme: `#2c3e50` header gradient,
  white canvas, `#f8f9fa` panels, `#e0e0e0` borders, `#5568d3` hover accent,
  rounded container with shadow. Future: extract shared CSS variables if both
  editors need dark mode support.
- **Undo/redo** — not yet implemented for graph operations
- **Rubber-band multi-select** — single selection only
- **Copy/paste** — not implemented
- **Persistent trunk waypoints for multi-port nets** — trunk is recomputed each
  time; users can't manually adjust trunk segments
- **Graph ↔ Equation composition** — can graphs contain equations? Can equations
  embed graphs? Needs design work.

**PRIORITY: Connector / wiring UX issues:**

- **Waypoint manipulation is buggy** — moving, adding, and removing waypoints on
  existing connectors does not work reliably. Hit testing on segments and waypoint
  handles needs debugging. Coordinate transforms and snap-to-grid interaction may
  be the root cause.
- ~~**Obstacle avoidance in Manhattan routing**~~ — DONE (grid-based Dijkstra).
  Algorithm inspired by jose-mdz's Orthogonal Connector (20 forks, used by
  UMLBoard and BlockSuite):
  1. Inflate all component AABBs by `SHAPE_MARGIN = 10px` — **no exclusions**,
     including connected components (since `WIRE_STUB = 30 > SHAPE_MARGIN = 10`,
     stubs always extend beyond the obstacle boundary).
  2. Collect "rulers" from inflated edges + endpoint coordinates.
  3. Generate candidate spots at ruler intersections, filtering out any inside
     an obstacle AABB.
  4. Build adjacency graph connecting orthogonal neighbors with clear line-of-sight
     (`segmentClearOfObstacles`).
  5. Dijkstra shortest path with bend penalty (direction changes cost 1.5×
     edge weight) to minimise unnecessary turns.
  6. Simplify collinear intermediate points.
  Applied to 2-port nets (`computeDefaultWaypoints`), multi-port trunk-branch
  legs (`computeTrunkBranch`), and star-topology legs (`getMultiNetLegs`).
  Functions: `routeOrthogonal`, `segmentClearOfObstacles`, `buildObstacleList`,
  `getComponentAABB`, `pointInsideAnyObstacle`, `simplifyOrthogonalPath`.

  **Routing improvement roadmap (future approaches):**
  1. **Steiner tree for multi-port nets** — compute minimum spanning tree of all
     ports, route each edge with Dijkstra. Better multi-port topology than
     current trunk-branch (which picks the farthest pair as trunk).
  2. **Visibility graph** — expand each AABB by margin, use corners as graph nodes,
     find shortest orthogonal path. Exact optimal without ruler grid.
  3. **Channel router** — EDA-grade approach, define routing channels between
     components, assign wire tracks. Only warranted if Kleis grows into IC design
     with thousands of components.

- **Only Manhattan routing mode for electronics** — orthogonal (right-angle) routing
  is the only connector type. Missing: curved/spline connectors (future).
- **Electronics connector behavior is hard to understand** — the interaction model
  for wiring electronic components is confusing. Needs a UX review: what happens
  on click, drag, release at each stage of connection creation. Consider visual
  feedback (highlight valid ports, show snap targets, preview wire path).
- **These are the highest-impact UX issues** — if users can't reliably draw and
  adjust wires, the graph editor is frustrating regardless of backend power.
  Fix connector UX before adding new domains or features.

**PRIORITY: Domain feature completeness:**

- **Petri Nets — only linear workflows** — current implementation cannot model
  branching, concurrency, or conflict resolution. Real-world workflows need:
  fork/join (parallel execution), choice/merge (decision points), weighted arcs,
  inhibitor arcs, and token colors for distinguishing resource types.
- **Electronics — more component types needed** — op-amps, transformers,
  dependent sources. Better simulation visualization, reliable oscilloscope
  across circuit topologies. (Ground handling and multi-node MNA are done.)

**THEORY: Nonlinear elements via topology/causality invariance:**

The constitutive equation of a component can change (linear → nonlinear) without
affecting the graph topology or causality assignment. This is the architectural
separation:

1. **Topology** (incidence matrix, KVL, KCL) — graph property, invariant
2. **Causality** (which variable imposed, which derived) — topological, depends
   on component *type*, not on the constitutive equation
3. **Constitutive relation** (V=IR vs I=I_s*(exp(V/V_t)-1)) — pluggable, does
   not change the structural verification layer

A nonlinear resistor has the same causality as a linear one. A diode, a BJT,
a MOSFET — they all occupy the same position in the incidence matrix and obey
the same KVL/KCL constraints. Only the port equation changes.

**This principle extends beyond electronics.** Geometric nonlinearities on
airplane control surfaces are the same pattern: the topology of the control
linkage (which surface connects to which actuator) and the causality (which
variable is commanded, which is derived) remain fixed. The nonlinearity is in
the constitutive relation — the hinge moment as a nonlinear function of
deflection angle, dynamic pressure, and Mach number. The graph structure
(actuator → linkage → surface → aerodynamic load) is invariant.

**Develop a Kleis theory** that formalizes this separation:
- Structure for topology (incidence matrix, conservation laws)
- Structure for causality (effort/flow assignment, SCAP)
- Parametric structure for constitutive relations (pluggable equations)
- Prove: topology verification and causality assignment are independent of
  the constitutive relation
- Instantiate for: electronics (linear + nonlinear), bond graphs, and
  flight control surface linkages as a non-electrical example

**Envelope inference from constitutive relations:**

Once the constitutive relation is parametric (depends on operating conditions),
Z3 can infer the **operating envelope** where the system remains stable, well-posed,
or satisfies safety constraints. The topology and causality are fixed; the question
becomes: "for what range of parameters does the system maintain property X?"

- **Flight control:** Given nonlinear hinge moment C_h(δ, q, M), Z3 can find the
  (q, M) region where the control authority is sufficient: ∃δ such that the required
  moment is achievable. The boundary of that region IS the flight envelope for that
  control surface. Flutter boundaries, control reversal speed — all expressible as
  satisfiability queries over the constitutive relation.
- **Electronics:** Given a nonlinear amplifier, find the input voltage range where
  the circuit remains in the linear operating region (transistors not saturated,
  op-amps not clipping). The boundary IS the dynamic range envelope.
- **Bond graphs / mechanical systems:** Given a nonlinear spring or damper, find
  the force/displacement range where energy dissipation remains positive (passivity
  envelope). Beyond that boundary, the element can inject energy — instability.
- **Petri nets / workflows:** Given resource constraints (token limits, processing
  times), find the throughput range where the workflow completes without deadlock.
  The boundary IS the capacity envelope.

**The general pattern:** topology + causality define the system structure.
Constitutive relations parametrize the behavior. Z3 queries over the parameters
yield the envelope — the boundary between "the system works" and "the system
fails." This is domain-agnostic: the same Kleis theory structure works for
flight envelopes, safe operating areas, passivity boundaries, and workflow
capacity limits.

  **Key insight: equations inside components shift verification from structural to behavioral.**
  Currently `component_type` is ground truth for SCAP causality and conflict checks.
  Once users can write constitutive equations inside components (e.g., `e = R * f`
  for a resistor, `e = V_0` for a source), the equation becomes the actual ground
  truth and `component_type` becomes a *claim* that the equation must satisfy.

  The equation determines three properties the structural checks currently assume:
  1. **Passivity**: Does `e * f ≥ 0` always hold? (energy dissipation)
  2. **Causality**: Does the equation impose effort, impose flow, or accept either?
  3. **Linearity**: Is the equation affine in port variables?

  A nonlinear passive element (e.g., `e = R*f + k*f³`) is fine — still dissipates
  energy, still indifferent causality. But a regime-switching hybrid (e.g.,
  `e = V₀*sin(t)` when `f > 0`, `e = R*f` otherwise) acts as a source in some
  regimes and a resistor in others, breaking SCAP's assumption of fixed roles.

  Z3 can verify these properties: assert `∀(f : ℝ). e(f) * f ≥ 0` for passivity,
  and Z3 either proves it or finds a counterexample regime. The incidence matrix
  gives topology, the equations give physics, Z3 checks consistency between them.

  This is the natural progression: structural verification (Phase 7) → behavioral
  verification (equations) → simulation (ODE solver). Each layer adds constraints
  that the previous layer assumed.
- **Oscilloscope** — live WASM-powered oscilloscope as a graph component. The ODE
  solver runs client-side; connecting to a net shows real-time waveforms. This is
  a major UX win that only works because of the Rust/WASM architecture.

#### WASM: removed from active code path

The WASM crate (`graph-editor-wasm/`) duplicated what JS does in ~55 lines —
building the COO incidence matrix and the graph AST. The computation is
O(connections), trivial for any reasonable graph. The cost was: 126KB binary,
552 lines of generated glue, a `wasm-pack` build step, a `buildWasmState()`
marshaling layer, and dual maintenance. WASM was removed from the active code
path, the binary artifacts deleted from `static/wasm/`, and the build step
removed from `scripts/build-kleis.sh`. The crate source is kept in
`graph-editor-wasm/` as a scaffold for when heavier computation arrives:

1. **Oscilloscope** — ODE solver at animation frame rate (real need for Rust speed)
2. **3D Plotting** — grid evaluation for surface rendering
3. **Large graph analysis** — hundreds of components, real-time constraint checking

---

### Electronics MNA Theory — NONLINEAR CIRCUIT SIMULATION

**Status:** Phase 1 COMPLETE. Newton-Raphson MNA simulation working end-to-end.
**Theory file (reference):** `theories/electronics_mna_nonlinear.kleis`
**Test file (verified):** `theories/test_electronics_jacobian.kleis` (15 examples pass)
**Implementation:** `std_template_lib/electronics.kleis` (MNA + NR sim_step)
**End-to-end test:** `electronics_rectifier_trajectory` (requires `--features numerical`)

#### What we proved (session May 14, 2026)

1. **Symbolic Jacobian works.** `diff()` from `stdlib/symbolic_diff.kleis` computes
   exact Jacobian entries for diode (Shockley), BJT (Ebers-Moll), and MOSFET (Level 1).
   Same engine that derives Schwarzschild curvature now derives Newton-Raphson stamps.

2. **No bridge function needed.** Native Kleis arithmetic (`exp`, `sin`, `ln`, `solve`,
   `matrix`, `ode45`, `fft`) handles all numeric computation. The Expression AST +
   `diff()` is for *deriving and verifying* formulas. Simulation runs natively, same as
   the phi4 paper computes Feynman parameter integrals.

3. **All builtins exist.** `solve`/`linsolve` (LAPACK) for J·Δx = -F, `matrix` for
   assembly, `eigenvalues` for stability, `fft`/`dft` for frequency analysis. No Rust
   changes needed.

4. **Constitutive equations verified at concrete operating points:**

   | Component | Jacobian entry | Value at operating point |
   |-----------|---------------|-------------------------|
   | Diode g_d (0.7V forward) | `(Is/nVt)·exp(Vd/nVt)` | 0.287 S |
   | Diode g_d (-1V reverse) | same formula | 1.43×10⁻¹⁷ S |
   | BJT g_m (0.65V active) | `(Is/Vt)·exp(Vbe/Vt)` | symbolic exp(25.15) |
   | MOSFET g_m (3V sat) | `Kp·(Vgs-Vth)` | exactly 0.002 S |

#### The gap: matching bond_graph.kleis infrastructure

`bond_graph.kleis` is 509 lines of working infrastructure that the server calls through
a well-defined contract. The electronics theory needs the same depth:

**Phase 1: eval_concrete setup interface — DONE**
- `sim_state_count`, `sim_state_map(i)`, `sim_input_map(k)`, `sim_connected_net(c)`
- `sim_initial_state(i)`, `sim_input_value(k)` — same recursive walker pattern
- Z3 probe stubs: `sim_topology_source = ""`, `sim_probe_count = 0` — NR doesn't
  need linearized A/B extraction

**Phase 2: Constitutive equations — DONE**
- `diode_current(Vd, Is, nd, Vt)` and `diode_conductance(Vd, Is, nd, Vt)`
- Voltage limiting via `diode_vcrit(nd, Vt) = 10*nd*Vt` — linearize above critical
  voltage to prevent exp() overflow during Newton-Raphson

**Phase 3: MNA Jacobian assembly — DONE**
- `stamp_2port_J(np, nm_, g, n, m)` — generic symmetric ±g pattern for 2-port
- Per-type wrappers: `stamp_J_resistor`, `stamp_J_capacitor`, `stamp_J_diode`,
  `stamp_J_vsource`, `stamp_J_ground`
- `stamp_F_*` functions for residual vector
- Port-order-based terminal identification: `term_pos(c)`, `term_neg(c)` via
  `graph_port_net_val(graph_comp_port0_val(c))` — correct for directed components
- `stamp_J_component`/`stamp_F_component` dispatch by fine-grained component type
- `mna_build_J(v)` and `mna_build_F(v)` assembled via `list_fold` over components

**Phase 4: Newton-Raphson sim_step — DONE**
- `nr_iterate(v, iter)` — recursive NR with `solve(J, -F)`, convergence check
  via `vec_max_abs(dv) < nr_tol` (tol = 1e-9, max 50 iterations)
- `sim_step(i) = extract_state(i, nr_iterate(nr_initial_guess, 0))`
- `sim_halted() = false`

**Phase 5: Z3 verification — EXISTING (structural only)**
- Ground exists, source exists, load exists, no parallel V sources, no series I
  sources — all working with fine-grained type predicates
- Future: KCL conservation, passivity, operating region verification

**Phase 6: Server integration — DONE (zero domain-specific code)**
- `server.rs` stays domain-agnostic. Electronics uses `sim_mode: "continuous"`.
- `build_sim_preamble` now scans theory source for `operation TYPE_X : ℤ`
  declarations and assigns sentinel codes for types not in the circuit.
- `graph_port_net_val(p)` and `graph_comp_port0_val(c)` added to preamble
  (domain-agnostic port-level connectivity).

**Phase 7: Adaptive timestep — DEFERRED**
- Needs Z3 probe protocol for linearized eigenvalue extraction
- Newton convergence monitoring for adaptive dt can be added later

**Fine-grained component types in electronics.kleist — DONE**
- Resistor, Capacitor, Inductor, Diode, LED, ZenerDiode, NPN, PNP, NMOS, PMOS,
  OpAmp, VoltageSource, ACVoltageSource, CurrentSource, Ground, Connector, Measurement
- Each with specific `params:` (e.g., diode: `Is:real:1e-12;n:real:1;Vt:real:0.02585`)
- electronics.kleis defines fine-grained `TYPE_X` operations and `is_X` predicates
- Coarse predicates (`is_passive`, `is_active`, `is_source`) as disjunctions

#### Key architectural decisions

1. **MNA, not bond graph formulation.** Electronics uses node voltages (MNA), not
   effort/flow (bond graph). The two formulations are mathematically equivalent but
   MNA is standard for electronics and handles nonlinear elements more naturally.

2. **Newton-Raphson in Kleis, not Rust.** The simulation loop is a Kleis function
   using `solve()`, `matrix()`, and native arithmetic. Same as phi4 uses `ode45()`.
   The server calls `eval_concrete` on theory-defined functions. Zero domain-specific
   Rust code. The server knows only "discrete" and "continuous" — it NEVER learns
   what domain it is simulating.

3. **Symbolic diff() for derivation and verification only.** The Jacobian formulas
   are derived symbolically (proof of correctness) then implemented as native Kleis
   functions (performance). `diff(diode_I_expr, "Vd")` proves the formula;
   `diode_gd(Vd) = (Is/nVt) * exp(Vd/nVt)` computes it.

4. **Companion models for C and L.** Backward Euler discretization turns reactive
   elements into resistor + current source at each timestep. The MNA matrix is
   purely algebraic — no ODE solver needed inside the Newton loop.

5. **Topology invariance.** The Jacobian sparsity pattern comes from the incidence
   matrix (topology). Only the stamp VALUES change with operating point. Replacing
   a resistor with a diode between the same nodes: same sparsity, different values.

#### Test circuit for development

Half-wave rectifier with LC filter (from test_electronics_jacobian.kleis):
```
        R_s (10Ω)    L (1mH)       D1 (1N4148)
 Vs ─────┤├───── n1 ──∿∿∿── n2 ──|►── n3 ──┬── C (100μF) ── GND
 (12Vpk 60Hz)                                │
                                         R_L (1kΩ)
                                              │
                                             GND
```
5 MNA unknowns (V1, V2, V3, V4, I_Vs), 1 nonlinear element, 2 state variables.
Small enough to debug, complex enough to exercise the full Newton-Raphson path.

**Phase 8: BJT support + multivibrator test — DONE**
- 3-terminal net helpers: `term_base(c)`, `term_coll(c)`, `term_emit(c)` via port-order
  (base=port0, collector=port1, emitter=port2 per electronics.kleist)
- Simplified Ebers-Moll BJT model: `bjt_ib`, `bjt_gbe`, `bjt_ic`, `bjt_gm`
  Reuses diode voltage limiting (ideality n=1) to prevent exp() overflow
- NPN Jacobian stamp (`stamp_J_npn`): 3x3 sub-pattern at (base, coll, emit) nodes
  - base row: +gbe, -gbe
  - coll row: +gm, -gm
  - emit row: -(gbe+gm), +(gbe+gm)
- NPN residual stamp (`stamp_F_npn`): ib at base, ic at collector, -(ib+ic) at emitter
- Wired into `stamp_J_component`/`stamp_F_component` dispatch
- **Tests:**
  - `multivibrator_setup_ok` — 10-component, 6-net circuit sets up correctly
    (2 state variables for C1, C2)
  - `electronics_multivibrator_oscillates` — end-to-end simulation runs
    (requires `--features numerical`)

**Multivibrator test findings:**
- Circuit: 2x NPN cross-coupled via capacitors (R1=R2=1k, R3=R4=10k,
  C1=C2=10uF, Vcc=5V)
- Symmetry breaking: C1 initial=0.1V, C2 initial=0V — required to escape
  the unstable symmetric equilibrium
- After 100 steps (dt=0.1ms, total t=10ms): C1=0.92V, C2=0.85V
- Both capacitors charging monotonically through base resistors — physically
  correct initial transient (RC time constant = 10k*10uF = 0.1s)
- First switching event expected around t ≈ 0.07s (700 steps) when a base
  voltage exceeds ~0.6V threshold

**Phase 9: Sparse stamp optimization — DONE**

Replaced dense O(n²×nc) Jacobian/residual assembly with sparse entry-list
approach. Two domain-agnostic evaluator builtins added:

- `assemble_matrix(n, entries)` — builds n×n matrix from `[row, col, value]`
  triples with scatter-add accumulation. Pure Rust, no interpreter overhead.
- `assemble_vector(n, entries)` — builds length-n vector from `[index, value]`
  pairs with scatter-add.

Each stamp function now emits a short list of non-zero entries (e.g., a
resistor emits 4 J-entries, a BJT emits 6) instead of being called n×n times
and returning 0 for most positions. The `list_fold` over components produces
the full entry list, then the Rust builtin assembles the matrix natively.

**Performance results:**

| Test                     | Before (dense) | After (sparse) | Speedup |
|--------------------------|---------------|----------------|---------|
| Rectifier (100 steps)    | 5.7s          | 1.24s          | 4.6x    |
| Multivibrator (100 steps)| 128s          | 12.0s          | 10.7x   |

The speedup is larger for the multivibrator because it has more components
(10 vs 5), widening the O(n²×nc) vs O(nnz) gap. Numerical results are
identical — same voltages, same convergence behavior.

**Why the J_linear/J_nonlinear split didn't help:** The earlier attempt to
decompose J into constant and voltage-dependent parts showed no improvement
(actually slightly slower) because: (1) the interpreter overhead of the
nested `map × map × list_fold` loop dominated, not which stamps were called;
(2) `is_vdep_jacobian` predicate checks (7 `∨` operations × 360 calls) cost
more than the stamps they skipped; (3) `eval_concrete` re-evaluates
zero-param defines every time (no caching), so a top-level J_linear wasn't
truly computed once.

**Why sparse works:** It eliminates the nested loops entirely. Instead of
nn×nn×nc = 360 interpreter calls (most returning 0), we make nc = 10 calls
each producing 4-6 entries, then Rust does 41 f64 additions. The interpreter
overhead reduction is proportional to (nn²/avg_entries_per_component).

**Scaling path for IC-scale circuits (thousands of components):**

The entry-list `[row, col, value]` format is the universal sparse contract.
The Kleis theory produces it identically regardless of circuit size. Only the
solver backend needs to change:

| Scale             | Assembly                     | Solver                           |
|-------------------|-----------------------------|---------------------------------|
| Small (< 50 nodes)| `assemble_matrix` (dense)   | LAPACK `dgesv` (current)        |
| Medium (50-5000)  | CSC build from entries       | KLU sparse direct               |
| Large (5000+)     | CSC build from entries       | Iterative (GMRES + precond.)    |

A future `sparse_solve(n, entries, rhs)` builtin would take the same entry
list, build a CSC representation, and call KLU — zero changes to
`electronics.kleis`. The Rust ecosystem has `sparse21` and SuiteSparse
bindings. Circuit Jacobians are inherently sparse because components connect
only a few nodes each (unlike neural network weight matrices which are dense).

**Phase 10: Graph Save/Load — DONE**

Implemented domain-agnostic Save/Save As/Load for the Graph Editor. Graphs
are persisted as `.kleis` files using `define` statements — the Kleis parser
itself is the file format parser.

**Server endpoints** (in `server.rs`, domain-agnostic):
- `POST /api/save_graph` — synthesizes `.kleis` text from `graphState` JSON,
  using `.kleist` template metadata for param ordering
- `GET /api/load_graph?path=...` — parses `.kleis` via `parse_kleis_program`,
  loads into `Evaluator`, extracts `graph_domain`, `graph_components`,
  `graph_nets` via `eval_concrete`, maps positional params back to named
  params using `.kleist` lookup
- `GET /api/list_graphs?domain=...` — lists `.kleis` files in
  `examples/{domain}/graph-editor/`

**Client-side** (`graphEditorMain.js`):
- Save (Ctrl+S), Save As, Load buttons in toolbar
- `loadGraphState(data)` rebuilds `graphState`, auto-routes wires, fits view
- File picker via `prompt()` (lists available files from server)

**Save format** (valid Kleis program):
```kleis
define graph_domain = "electronics"
define graph_components = [
    ["c0", "dc_voltage", "VoltageSource", 100, 150, 0, [5.0]],
    ...
]
define graph_nets = [
    ["n0", [["c0", "pos"], ["c1", "anode"]], []],
    ...
]
```

**Seed files** for manual testing:
- `examples/petri-nets/graph-editor/linear.kleis` (5 components, 4 nets)
- `examples/electronics/graph-editor/rectifier.kleis` (5 components, 3 nets)
- `examples/electronics/graph-editor/multivibrator.kleis` (10 components, 6 nets)
- `examples/bond-graph/graph-editor/rc_circuit.kleis` (4 components, 3 nets)

**Gap analysis**: Verified all three domains (electronics, bond graphs, Petri
nets) against existing `server.rs` test circuits. No blocking gaps. Derived
state (incidence matrix, A/B matrices, dt) is recomputed on load via
`/api/simulate_setup`. Causal strokes (bond graphs) stored via
`graph_net_causal`. Integer params (Petri nets) preserved naturally.

**Next steps for multivibrator:**
1. Extend simulation to first switching event (~700 steps, now feasible at
   0.12s/step ≈ 84s total) — verify cross-coupling voltage swing
2. Implement LCG pseudo-random noise generator in Kleis for automatic
   bifurcation perturbation (circuit-agnostic startup)
3. FFT-based harmonic analysis of the oscillation waveform

#### Stretch goal: synthesizer circuit

A Moog-style analog voice (~30-50 nodes, 20-40 nonlinear BJTs, 10+ op-amps).
Requires: op-amp macromodel, operating region switching, frequency analysis via
FFT. All builtins exist. The architecture scales — SPICE uses the same algorithm
for millions of transistors.

---

### HACKATHON CODE REVIEW — IN PROGRESS

**Last Updated:** April 19, 2026

### Context
Applied the HACKATHON 5-angle AI code review methodology to the full Kleis codebase. Deep Claude review found **39 findings at confidence ≥ 80**.

### Merged (PR #35 on origin, #33 on fork — branch `review/hackathon-code-review`)
13 fixes across Z3 backend, type system, evaluator, MCP servers, DAP, LSP:
- Z3 push/pop leaks in `evaluate()` and `are_equivalent()`
- Watchdog timeout for `are_equivalent()` + explicit `Unknown` handling
- `evaluate()` Sat path: explicit error when model extraction fails
- `translate_list_to_cons` panic → proper Err return
- `dynamic_to_set` type-unsoundness (verify Array range is Bool)
- `dynamic_to_string` type-unsoundness (use `Z3_is_string_sort`)
- `TypeExpr::Var` collapse to `TypeVar(0)` — unique ids per variable name
- `pretty_print_matrix` panic on empty/ragged matrices
- Path traversal defense in `save_theory` (null byte + canonicalize)
- Content-Length cap (64 MiB) in all MCP servers + DAP
- DAP ephemeral port race (pass pre-bound TcpListener)
- Redundant `STDIO_MODE.store` removal

### Current branch: `review/hackathon-code-review-2`
1 fix so far:
- `verify_axiom_impl` swallowed `ensure_structure_loaded` errors → now propagates with `?`

### Remaining findings (26 items, by tier)

**Tier 1 — Critical bugs (2):**
- #6 (Conf 88): `foldLines` arg order swapped vs documentation in `evaluator/builtins.rs`
- A (Conf 90): `check_consistency` error swallowed in `axiom_verifier.rs:649`

**Tier 2 — Security (3):**
- #10 (Conf 90): `readFile` arbitrary file read — no path restriction
- #11 (Conf 88): Unescaped import strings inject into session file
- #12 (Conf 88): `check_file` arbitrary file read — no workspace root restriction

**Tier 3 — Important bugs (8):**
- #13 (Conf 97): Lossy rational conversion via f64
- #14 (Conf 96): `bind_pattern_variables` ignores ADT field types
- #15 (Conf 96): `Type::Data` unification ignores constructor identity
- #16 (Conf 96): `check_action` schema mismatch for `git_push`
- #18 (Conf 95): Pattern guard default is `true`
- #19 (Conf 94): `check_function_def` stores body type, not arrow
- #20 (Conf 93): Unknown `TypeExpr::Named` silently becomes scalar
- #21 (Conf 90): Equality uses sort_kind not sort identity

**Tier 4 — Quality/DoS (13):**
- #23–39: Recursion depth limits, stack overflow in cons lists, parser nesting limits, operation registry silent overwrites, `declare_uninterpreted` always Int→Int, `alpha_convert` doesn't descend into Quantifier/Match, type ascription ignored, fail-open policy, unknown types default to Int sort, and more.

### Workflow
Generic command: "Pick the next unfixed finding from the Claude code review triage (Tier 1 first, then Tier 2, then Tier 3, by descending confidence). Read the code, understand the bug, fix it. Run all ~2400 tests. Then do a proper deep Claude code review — read the changed code and surrounding context yourself, apply the 5-angle HACKATHON methodology, and produce findings with confidence scores. Not just the MCP lint tool. Commit, push to both origin and fork, and update the PR."

### Key lesson
The MCP `check_code` lint tool is NOT a Claude code review. A proper review means reading the code, understanding the semantics, and producing findings with confidence scores using the 5-angle HACKATHON methodology.

---

### Future Kleis Capability: Tensor and FFT Support

The Python programs developed for the Fiber Solvability paper (`solve_3d_ising.py`, `gauge_representation_3d_ising.py`) use capabilities that Kleis should eventually support natively:

1. **Tensor operations**: The factorized transfer matrix uses a tensor product decomposition of 2×2 matrices applied to state vectors. The key trick: reshaping a 2^(N²) vector as an N²-index tensor, applying each 2×2 factor along one axis, then reshaping back. Natural for a Kleis `Tensor` structure with `contract`, `reshape`, and `kronecker` operations.

2. **DFT/FFT**: The Fourier decomposition of the transfer matrix into momentum sectors (k_x, k_y) uses the discrete Fourier transform. Kleis could support `dft(vector)` and `fft(vector)` as built-in operations on numeric arrays.

3. **Walsh-Hadamard transform**: Used for interaction-order analysis. Binary analog of the DFT, applying the Hadamard matrix H⊗H⊗...⊗H. Could be a Kleis built-in: `walsh_hadamard(vector)`.

4. **Power iteration / eigenvalue computation**: Finding the top eigenvalues of a matrix too large to diagonalize. Adding numerical linear algebra (at least `power_iteration(matrix_apply_fn, dim)`) would allow these computations to live inside `.kleis` files.

5. **Sparse/structured matrix-vector products**: The inter-layer coupling is never stored as a dense matrix — it's applied as a sequence of 2×2 operations. Kleis could support lazy/functional matrix representations where `apply(T, v)` is defined by a function, not by storing T explicitly.

**Goal:** Reproduce the 3D Ising β_c computation entirely in Kleis, with the tensor trick, FFT decomposition, and power iteration all expressed as Kleis operations verified alongside the Z3 theory.

---

### Planned: LilyPond Integration (Phase 1.5)

#### Decision (ADR-033 updated March 2026)

LilyPond cannot be compiled as a library (107k LOC monolithic CLI, deep Guile
Scheme dependency, no embedding API). Strategy: subprocess via
`render_score_svg()` built-in, feature-gated under `lilypond`. See ADR-033.

#### Implementation

- `src/evaluator/music.rs` — `render_score_svg(score)` built-in
- `Cargo.toml` — `lilypond` feature flag
- `scripts/build-kleis.sh` — LilyPond detection

---

### TODO: Integrate 3D Plotting in Kleis (plotsy-3d)

**Priority:** Medium (no urgency) — enables 3D visualization in papers, Jupyter notebooks, and REPL.

#### Context and Prototype

Prototyped 3D surface plotting using `plotsy-3d` Typst package (v0.2.1, built on CeTZ). Pure Typst/SVG output, compiles in ~1.4s.

**Prototype file:** `examples/plotting/plotsy3d_itcm_kernel.typ` — fully working, do not start from scratch.

#### Key Architectural Finding: Lilaq and plotsy-3d Cannot Compose

Lilaq uses native Typst primitives. plotsy-3d uses CeTZ. Two separate rendering stacks. However, Kleis documents can contain both as separate figures.

#### Pipeline (shared infrastructure)

```
2D: diagram(plot(...)) → PlotElement structs → generate lilaq Typst    → compile_to_svg()
3D: diagram3d(surface(...)) → [new structs]  → generate plotsy-3d Typst → compile_to_svg()
                                                                              ↓
                                                                    PLOT_SVG → Jupyter/REPL
```

#### Three Implementation Options

**Option A: Full Mirror** — New `src/plotting3d.rs`, `PlotElement3D` enum, `diagram3d()` builtin.
Pro: Cleanest API. Con: Most Rust work; lambda-to-Typst fragile.

**Option B: Thin Data Wrapper** — Pre-evaluate lambdas on grid in Rust, bake z-values into Typst.
Pro: Much less work; faster. Con: Grid resolution fixed at call time.
**This is the recommended starting point.**

**Option C: Raw Typst Escape Hatch** — `typst_svg(code_string)` builtin.
Pro: Zero Rust changes. Con: Not Kleis-native.

**Option D: Wait for Lilaq Native 3D** — Monitor [lilaq issue #31](https://github.com/lilaq-project/lilaq/issues/31).
Pro: Zero effort. Con: No timeline.

#### Gotchas from Prototyping

- `scale-dim` values must be tiny (0.01-0.05 range)
- `plotsy-3d` uses integer `range()` internally
- `subdivision-mode: "decrease"` = coarser grid, `"increase"` = finer
- Color functions receive 9 args: (x, y, z, x-lo, x-hi, y-lo, y-hi, z-lo, z-hi)
- Multiple surfaces in one scene is feasible via composable render functions

#### Decision: Deferred, but elevated as WASM learning vehicle

Options ranked: D (wait) > B (thin wrapper) > A (full mirror) > C (raw Typst).

**New consideration:** 3D plotting is a natural first WASM project — smaller scope
than the Graph Editor, mostly computation→visualization (one direction), and teaches
the `wasm-pack` / `wasm-bindgen` / `web-sys` workflow. The Graph Editor (WASM step 2)
adds bidirectional interaction on top of those patterns. Doing 3D plotting first
means arriving at the Graph Editor with WASM experience already in hand.

**WASM learning progression:**
1. 3D plotting: Rust evaluates grid → WASM → browser renders surface (one-way)
2. Graph Editor: user events → WASM graph state → SVG rendering → user events (bidirectional)

---

### TODO: Additional Kleis Publication Templates

Create Kleis template wrappers for major publication venues:

**Priority targets (Typst packages already exist):**
- [ ] **AMS** (`unequivocal-ams`) — American Mathematical Society style
- [ ] **Springer Nature** (`stellar-springer-nature`) — Nature, Nature Physics, etc.
- [ ] **IEEE** (`charged-ieee`) — IEEE conferences/journals
- [ ] **APS / RevTeX** (`revtyp`) — Physical Review style

**Secondary targets:**
- [ ] **Elsevier** (`elspub`)
- [ ] **NeurIPS** (`bloated-neurips`)
- [ ] **LNCS** (`fine-lncs`)
- [ ] **IOP** (`ioppub`)

---

## Reference Material

### ResearchGate DOIs

Papers published on ResearchGate with permanent DOIs:

| Paper | DOI | Date |
|-------|-----|------|
| Independence as Non-Invariance: Detecting Undecidability via Projection Fibers in SMT-Backed Shadow Theories | 10.13140/RG.2.2.22374.18243 | 2026-04-24 |
| Observable Bounds on Ontological Dimension: A Constructive Consequence of Projection Fiber Theory | 10.13140/RG.2.2.11468.99206 | 2026-04-24 |

**Next to upload:** Abstract K-Q Framework (uploaded but rate-limited, needs license/details fix), Moonlight Sonata

### Recommended Publication Order (ResearchGate)

1. ~~Independence as Non-Invariance (Projection Fibers)~~ — **DONE**
2. ~~Observable Bounds on Ontological Dimension (Fiber Dimension)~~ — **DONE**
3. The Abstract K-Q Framework — **uploaded, needs details fix**
4. The Beauty is in the Skolems (Moonlight Sonata)
5. Theory Selection and Divergence Kernels
6. Flat Galactic Rotation Curves (POT)
7. Electrodynamics as a Theorem of POT
8. Confinement as Fiber Non-Invariance (Yang-Mills)
9. Admissibility Restoration (Higgs necessity)
10. Renormalization as Projected Ontology (Volume VII)
11. Conditional Reduction of Yang-Mills Mass Gap (Volume VIII)
12. Yang-Mills Vacuum Stability (Volume IX)
13. Projection Singularities: Why Physics Has No Infinities (Volume X)
14. Quantization as Projection Kernel (Volume XI)
15. The Spectral Comb and the Riemann Hypothesis
16. Transfer Function (Hilbert-Pólya)
17. The Hum (Twin Prime Beat Structure)
18. NS Smoothness (Half-Derivative Gap)
19. NS Geometric Depletion
20. NS Bent Tubes
21. NS Dynamical Closure
22. NS Forced Localization
23. NS Unconditional Regularity (Grand Finale)
24. NS Epilogue (Kernel and the Fluid)
25. φ⁴ One-Loop
26. QED Vacuum Polarization
27. YM One-Loop Gluon Self-Energy
28. Ghost Activity Theorem
29. Gauge Dependence and Ghost Activity
30. Structural Atlas of ker(Q)
31. POT vs GR: Gravitational Lensing
32. Schanuel's Conjecture
33. Toeplitz Inscribed Square
34. Selberg Universality
35. Classical Spectral Essay (Mass Gap Epilogue)
36. Technical Brief: Realization Tautology
37. Quantum Entanglement as POT

**Rationale:** Lead with the conceptual root (fibers), then the generalization (K-Q), then the attention-grabber (music). Follow with POT foundations, the renormalization arc, RH, NS, one-loop stress tests, and remaining papers.

---

### POT PHILOSOPHICAL BOUNDARY: NON-IDENTIFIABILITY OF ONTOLOGY

**This is a non-negotiable constraint on all future papers.**

#### The Three-Part Principle

**Principle (Non-identifiability of ontology).** Observable data determine only im(Q). The pre-image is many-to-one; therefore ontology is not uniquely identifiable.

**Consequence.** Do not specify ontological dynamics (e.g., a Lagrangian for the modal flow in ontological Hilbert space). Instead, characterize the admissible structure of (K, Q).

**Structural Claim.** ker(Q) encodes the constraints discarded by projection; its internal organization is observable via its effects on im(Q), even though its elements are not.

#### ker(Q) is constrained residue, not arbitrary

From the five K-Q papers, ker(Q) is constrained by:
- **Symmetries** — gauge/Lorentz invariance → Ward/Slavnov-Taylor identities
- **Consistency of Q** — same observable must arise from equivalent pre-images
- **Compatibility with K** — Q∘K must land in im(Q) with correct invariants
- **Regularity/admissibility** — existence of convergent representatives on [0,1]

#### ker(Q) is already being studied

| Paper | What was found in ker(Q) |
|-------|--------------------------|
| φ⁴ | Contains A₀ and scheme-dependent constants. Passive. |
| QED | Ward identity shrinks ker(Q). Ghost sector present but inert. |
| Yang-Mills | Ghost sector active — shapes β₀ through Q∘K. |
| Ghost theorem | Activity iff f^{abc} ≠ 0. Algebra determines structure. |
| Gauge dependence | ker(Q) realization is representation-local; effects on im(Q) are invariant. |

#### Promote structure, not dynamics

Do not write a Lagrangian for the pre-projection modal flow. But do extract structural statements about ker(Q) that are representation-robust:
- Representation-local vs. representation-invariant
- Active vs. inert sectors
- Kernel-induced constraints

#### Natural next theorem

**Representation-Invariant Decomposition (sketch).** For representations R₁, R₂ of the same theory:
- im(Q) is identical
- the realization of ker(Q) differs
- there exists a transformation that pushes forward the constraint flow through K so that Q∘K agrees on invariants

---

### POT VUFT Series (current inventory)

| Volume | Title | Kernel | Status |
|--------|-------|--------|--------|
| I | Flat Galactic Rotation Curves from Projected Ontology | Gravitational (logarithmic Green's function) | Published |
| II | Quantum Entanglement as a Projection Artifact | Measurement (spinor projections) | Published |
| III | Electrodynamics as a Theorem of Projected Ontology | Gauge (d\|_Ω¹, admissible, nilpotent) | Complete |
| IV | Confinement as Fiber Non-Invariance | Non-admissible Yang-Mills (Lie bracket defect) | Complete |
| V | Admissibility Restoration: Structural Necessity of SSB | Restored (coupling to Higgs restoring field) | Complete |
| VI | The Kernel and the Fluid: An Epilogue | Biot-Savart (epilogue, all four forces) | Complete |
| VII | Renormalization as Projected Ontology: The Theory That Was Never Divergent | Composite (FP ∘ K_ren ∘ K_path), ITCM hypergeometric | Complete |

Each volume is independently verifiable via `kleis test`. The substrate (stdlib) is shared.

---

### Seven-paper inventory (K-Q series) + abstract framework

| # | Paper | Theory file | Paper file | Results |
|---|-------|-------------|------------|---------|
| 1 | φ⁴ one-loop | pot_phi4_oneloop.kleis | pot_phi4_oneloop_paper.kleis | 18 worked |
| 2 | QED vacuum pol. | pot_qed_vacuum_polarization.kleis | pot_qed_vacuum_polarization_paper.kleis | 15 worked |
| 3 | Yang-Mills | pot_ym_vacuum_polarization.kleis | pot_ym_vacuum_polarization_paper.kleis | 14 worked |
| 4 | Ghost theorem | pot_ghost_activity_theorem.kleis | pot_ghost_activity_theorem_paper.kleis | 17 Z3 |
| 5 | Gauge dependence | pot_gauge_dependence_ghost.kleis | pot_gauge_dependence_ghost_paper.kleis | 16 Z3 |
| 6 | Structural atlas | pot_ker_q_atlas.kleis | pot_ker_q_atlas_paper.kleis | 24 Z3 |
| 7 | Abstract K-Q framework | pot_abstract_kq_framework.kleis | pot_abstract_kq_framework_paper.kleis | 24 Z3 |

---

## Growing Brain — Distributed Expert Architecture

**Insight (April 30, 2026):** The self-growing transformer brain is not just a Kleis learner — it's a general-purpose architecture for building a *community of small expert models* that self-organize.

### The Architecture

1. **Train specialized brains independently** — one per subject area (algebra, topology, physics, music, etc.), each with its own domain-tuned tokenizer. Each brain self-grows its architecture (layers, heads, FFN) to match the complexity of its domain.

2. **Self-selecting routing** — no explicit router needed. Feed the prompt to all brains; each reports its perplexity. Low perplexity = "I understand this." The brain with the lowest score handles generation.

3. **Graceful uncertainty** — if all brains score high perplexity, the system knows it doesn't know. Signal to train a new brain. No hallucination.

4. **Independent updates** — when new information arrives in one domain, retrain only that brain. Others remain stable. No catastrophic forgetting by design.

5. **Kleis as arbitration** — when multiple brains claim knowledge (e.g., algebra and topology both recognize "fundamental group"), they generate competing completions. Kleis formalizes both as structures, Z3 checks consistency, detects subsumption or genuine contradiction.

### The Closed Loop

```
Train brains independently (knowledge production)
         ↓
Brains self-select on prompts (routing via perplexity)
         ↓
Multiple brains respond (competing claims)
         ↓
Kleis formalizes and verifies (arbitration via Z3)
         ↓
Consolidated knowledge feeds back (learning)
```

Disagreement between brains isn't a bug — it's a signal that there's a deeper connection to formalize. This is how mathematics advances: fields develop independently, then someone discovers they're isomorphic, and a unifying structure emerges.

### Why We Sleep (tongue in cheek)

Turns out nature already shipped this architecture. Two hemispheres (left: symbolic/sequential, right: spatial/holistic) are independent experts with different internal architectures, connected by a 200-million-fiber arbitration bus (corpus callosum).

And the reconciliation pass? That's sleep:

- **Awake**: Independent experts process input, accumulate competing claims
- **Sleep**: System goes offline for batch verification — replay, check consistency, merge convergent knowledge, flag contradictions
- **Wake**: "I figured it out overnight" — unified knowledge available

Sleep isn't triggered by darkness. It's triggered by the reconciliation queue hitting capacity. The drowsiness signal is: "experts have diverged beyond threshold, batch consolidation required." Motor atonia (muscle paralysis during REM) is the I/O lockout — you don't let half-merged knowledge drive actuators. Children sleep more because they accumulate more competing claims per day. Sleep deprivation causes hallucination because unreconciled claims leak into output when you skip the verification step.

We accidentally reinvented the biological brain architecture: independent self-growing experts, perplexity-based self-selection, mandatory offline arbitration with I/O lockout until consistency is restored. Nature's cron job for formal verification.

### Kleis as the Native Tongue + HM Unification for Reconciliation

The growing brains don't just get verified by Kleis — they *think* in Kleis. It's simultaneously:
1. The training corpus (what they learn)
2. The output language (what they produce)
3. The reconciliation protocol (how they talk to each other)

No lossy translation. The verification language and the thinking language are the same.

**The naming problem:** Different brains will inevitably produce similar structures named differently and parametrized differently. An algebra brain might call it `Group(T)` with `op`, `inv`, `e`. A topology brain might call it `LoopSpace(X)` with `compose`, `reverse`, `constant`. Same axioms, different vocabulary.

**The solution:** Hindley-Milner unification (already in Kleis) strips the names and compares structural shapes:

```
Group(T)     :  (T → T → T) × (T → T) × T × (∀x. op(x, e) = x)
LoopSpace(X) :  (Path(X) → Path(X) → Path(X)) × (Path(X) → Path(X)) × Path(X) × (∀p. compose(p, constant) = p)
```

HM unifies `T ~ Path(X)`, `op ~ compose`, `inv ~ reverse`, `e ~ constant`. Z3 then checks that the axioms are equivalent under substitution. If it unifies → same theory, merge. If not → genuinely different, keep both.

This mechanizes what mathematicians do by hand and call "recognizing an isomorphism." The naming problem — the hardest problem in distributed knowledge — is solved by the type system. Brains don't need to agree on names. The type system sees through them.

### Kleis as EDIFACT Replacement

EDIFACT (Electronic Data Interchange for Administration, Commerce, and Transport) is the decades-old standard for B2B communication: rigid message formats where both trading partners must conform to pre-negotiated segment structures. Adding a field means updating the entire standard through international committees.

Kleis replaces this with *semantic* interoperability:

- **EDIFACT**: both parties must speak the *same syntax* (segment UNH, field positions 1-9, exact order)
- **Kleis**: both parties express their domain in their own structures; HM unification proves type-compatibility

Each party keeps their internal representation. A supplier calls it `SKU` with `qty`; a buyer calls it `ProductCode` with `quantities`. The type system unifies them structurally, Z3 verifies the axioms match. No pre-negotiated format, no version committees, no rigid segment positions.

Onboarding a new trading partner doesn't require them to change their internal data model — just expose it as a Kleis structure and let unification prove compatibility. This is the same reconciliation mechanism the growing brains use: different vocabularies for the same semantic content, resolved by the type system.

**Beyond execution — enabling negotiation itself:**

EDIFACT only handles *executing* pre-agreed transactions. Kleis enables the *negotiation*:

1. Both parties submit their structures with their constraints
2. Divergence kernel localizes exactly where they disagree (not "incompatible" but "you require `delivery_window ≤ 3`, I require `≤ 7`")
3. One proposes a modification: `delivery_window ≤ 5`
4. Z3 checks: does this satisfy both parties' remaining hard constraints? Is the combined structure consistent?
5. If yes — agreement. If no — iterate on the next predicate.

The system can even *suggest* the rational middle ground: "the weakest axiom that satisfies both parties' hard constraints is X." That's constraint solving over the union of both structures — exactly what Z3 does. Disagreement becomes a set of predicates, negotiation becomes constraint relaxation, and agreement is the moment both structures unify under a shared weakening.

The divergence kernel computation (`examples/papers/divergence_kernels_paper.kleis`) is the theoretical foundation for this.

### Repositories

- [kleis-brain-v1](https://github.com/engingithub/kleis-brain-v1) — Character-level, Rust
- [kleis-brain-v2](https://github.com/engingithub/kleis-brain-v2) — Kleis-aware BPE tokenizer, Rust
- Python prototype: `examples/mathematics/growing_transformer_brain.py` (branch: `feature/growing-brain-python`)

---

## Archived

Completed session notes and paper documentation have been moved to:

**[`docs/archive/sessions/sessions-2026-feb-apr.md`](../archive/sessions/sessions-2026-feb-apr.md)**

Covers: Sessions 6-32f (Feb 23 - Apr 27, 2026), all completed POT papers (Volumes III-VII, K-Q Papers 1-7), NS regularity papers (Papers 1-5), RH papers, Music Theory, Independence Paper, GR Lensing, and engineering work.

Previous archive: [`docs/archive/sessions/sessions-dec-jan.md`](../archive/sessions/sessions-dec-jan.md)
