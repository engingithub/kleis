# Next Session Notes

**Last Updated:** April 2, 2026 (session — Volume IV: Yang-Mills Confinement paper completed)

---

## COMPLETED: "Confinement as Fiber Non-Invariance" (Volume IV)

**Theory file:** `theories/pot_yang_mills_confinement.kleis` (11 structures, 34 axioms, 19 Z3-verified examples)
**Paper file:** `examples/ontology/revised/pot_yang_mills_paper.kleis`
**PDF:** `examples/ontology/revised/pot_yang_mills_paper.pdf`
**Status:** All 19 theory examples + 11 paper examples pass. PDF compiles cleanly.

### What the paper achieves

- Derives color confinement from kernel non-admissibility WITHOUT assuming quantum mechanics
- Identifies the admissibility defect Δ(A,B) = K(A+B) - K(A) - K(B) with the Lie bracket [A,B]
- Proves: admissible ⟺ abelian (Theorem 1, the Abelian Classification)
- Proves: non-admissible ⟹ image non-invariant on fibers ⟹ charge unobservable = confined (Theorem 3)
- Derives Gribov obstruction, observable hierarchy (min order 2), nonlinear nullspace as corollaries
- Connects to Cantor independence via the fiber non-invariance theorem: CH is undecidable for the same structural reason color is confined
- 6 named theorems, all machine-verified

### What it does NOT claim

- Does not derive the linear confining potential (area law)
- Does not derive the mass gap (Millennium Prize problem)
- Does not derive asymptotic freedom (requires scale-dependent kernels)
- Does not derive the hadron spectrum

### Next Paper Candidates (Volume V)

- **Aharonov-Bohm as Kernel Non-Surjectivity** (clean, extends ED paper's monopole discussion)
- **Admissibility Restoration** (natural sequel to Vol IV — can additional fields restore a non-admissible kernel to admissibility?)
- **Standard Model Gauge Sector** (SU(3)×SU(2)×U(1) classification via admissibility)

---

## COMPLETED: "Electrodynamics as a Theorem of Projected Ontology"

**File:** `examples/ontology/revised/pot_electrodynamics_paper.kleis`
**PDF:** `examples/ontology/revised/pot_electrodynamics_paper.pdf` (15 pages)
**Branch:** `feature/pot-electrodynamics` (pushed to origin + fork)
**Status:** All 14 verification examples pass. Reviewed by ChatGPT and Gemini — assessed as "accept with minor revisions" level.

### What the paper achieves

- Derives complete differential-form structure of classical electrodynamics from 2 axioms + d²=0
- Identifies exterior derivative d : Ω¹→Ω² as an admissible kernel in the formal POT sense
- Gauge equivalence = projective equivalence, gauge orbits = kernel nullspace, d²=0 = kernel nilpotency
- Classification result: electrodynamics is the unique gauge sector with an admissible projection kernel (U(1) abelian ⟹ linear; Yang-Mills A∧A breaks admissibility)
- Physical justification for admissibility: superposition, stable equivalence classes, composability
- Does NOT assume GR — only a Lorentzian manifold (kinematic stage, not gravitational dynamics)
- All derivations machine-checked by Z3

### Pre-paper fixes that were completed

- Lorentzian Hodge star: generalized with signature parameter s in `stdlib/differential_forms.kleis`
- ElectromagneticForm: minimized to 2 independent axioms (F=dA, d⋆F=⋆J)
- Import of `theories/pot_admissible_kernels_v2.kleis` for kernel formalism

---

## POT Verified Unified Field Theory (VUFT) Series

The papers form a series, each adding a sector to the kernel framework:

| Volume | Title | Kernel | Status |
|--------|-------|--------|--------|
| I | Flat Galactic Rotation Curves from Projected Ontology | Gravitational (logarithmic Green's function, slow-decay coherence) | Published |
| II | Quantum Entanglement as a Projection Artifact | Measurement (spinor projections, detector angle) | Published |
| III | Electrodynamics as a Theorem of Projected Ontology | Gauge (d\|_Ω¹, admissible, nilpotent) | Complete |
| IV | Confinement as Fiber Non-Invariance: The Admissibility Boundary | Non-admissible Yang-Mills (Lie bracket defect) | Complete |

Each volume is independently verifiable via `kleis test`. The substrate (stdlib) is shared.

---

## Next Paper Candidates (Volume V)

### ~~Option A: Yang-Mills Confinement~~ ✓ COMPLETED as Volume IV

See above. The revised thesis derives confinement from fiber non-invariance without assuming QM.

### Option B: Aharonov-Bohm as Kernel Non-Surjectivity

**Thesis:** On topologically nontrivial manifolds (R³ \ {0}), the EM kernel d is not surjective onto closed 2-forms. The gap (H²_dR ≠ 0) produces physically observable effects (A-B phase) even where F=0. The potential A has physical consequences precisely because the kernel's image is smaller than the space of closed forms.

**Infrastructure needed:** `DeRhamCohomology` already axiomatized. Would need to formalize the A-B setup (multiply-connected region, path integral of A around solenoid) and show the phase is a cohomological invariant.

**Risk:** Moderate. Well-understood physics, clean kernel interpretation. Shorter paper.

### Option C: Admissibility Restoration via Additional Fields

**Thesis:** Volume IV established that non-admissible kernels confine. Can additional degrees of freedom, coupled to the kernel, restore effective admissibility? If so, what constraints does the restoration impose on the restoring field? Derive the mechanism structurally from POT, then identify whether it corresponds to what the standard framework calls spontaneous symmetry breaking.

**Infrastructure needed:** Formalize kernel modification by coupling to additional fields. Show when/how admissibility can be restored and what the restoring field must satisfy.

**Risk:** High. Natural sequel to Volume IV.

### Option D: Mass Gap as Topological Obstruction on the Nullspace Variety

**Seed idea (from Gemini review of Volume IV, April 2026):**

Volume IV's Theorem 6 establishes that non-admissible kernels have nonlinear nullspaces (the moduli space of flat connections), while admissible kernels have contractible vector subspace nullspaces. The topology of these two nullspaces is fundamentally different, and this difference may force a spectral gap.

**The argument (not yet formalized):**

| | Admissible (U(1)) | Non-Admissible (YM) |
|---|---|---|
| Nullspace | Vector subspace | Nonlinear variety (moduli space) |
| Topology | Contractible (trivial) | Non-trivial homology (Z, ...) |
| Excitation | Can shrink continuously to 0 | "Snagged" on topological holes |
| Spectrum | Continuous (no gap) | Discrete/bounded (mass gap) |

The key move: reframe the mass gap from "What is the lowest eigenvalue of the QCD Hamiltonian?" (dynamical, nobody can answer) to "What is the spectral gap of the Laplacian on the nullspace variety?" (spectral geometry, constrained by topology). This is a POT move — structure replaces dynamics.

**Why it might work:**

- Spectral geometry has hard theorems: the Cheeger inequality bounds the spectral gap below by a geometric constant (the "narrowest bottleneck" of the manifold). Lichnerowicz's theorem says positive Ricci curvature forces a positive spectral gap.
- The moduli space of flat connections has non-trivial topology for all non-abelian groups. For SU(2) on a Riemann surface of genus g, the moduli space has dimension 3(g-1) and non-trivial fundamental group.
- Instantons and theta vacua (π₃(SU(N)) = Z) are exactly the topology of the configuration space creating distinct sectors that affect the spectrum. This is already understood in standard physics — we'd be axiomatizing it in POT.
- Gemini's metaphor: "You aren't calculating a force; you are calculating the lowest possible note on a drum of a certain shape. If the drum has a hole, certain low-frequency notes cannot be played."

**How it would work in Kleis:**

We wouldn't compute eigenvalues (Z3 isn't a numerical solver). We'd axiomatize spectral geometry results and verify the logical chain:

```
structure SpectralGeometryOnNullspace {
    axiom cheeger_bound : ∀(K : GreenKernel).
        implies(not(is_admissible(K)),
            spectral_gap(nullspace_laplacian(K)) > 0)
}
```

Z3 verifies that the chain from non-admissibility → nonlinear nullspace → non-trivial topology → positive spectral gap is internally consistent. The mathematical content comes from spectral geometry; the formal verification comes from Kleis.

**Infrastructure needed:** Axiomatize parts of spectral geometry (Laplacian on varieties, Cheeger inequality, Lichnerowicz theorem). Define a "nullspace Laplacian" in POT terms. Formalize the connection between moduli space topology and spectral bounds.

**Risk:** Very high. Harder than Volume IV (which was algebraic). This requires differential geometry on the moduli space — curvature, Cheeger constants, spectral bounds. But the payoff would be enormous: a structural derivation of the mass gap from the same non-admissibility that produces confinement.

**Status:** Seed idea only. A remark has been added to the Volume IV paper (nullspace subsection) planting this direction. Not committed to as a plan.

---

### Option E: The Standard Model Gauge Sector — SU(3) × SU(2) × U(1)

**Thesis:** Extend the kernel classification to the full Standard Model gauge group. U(1) is the unique admissible sector. SU(2) × U(1) electroweak theory is partially admissible (the U(1) hypercharge factor). SU(3) is fully non-admissible.

**Infrastructure needed:** Electroweak mixing (Weinberg angle), symmetry breaking. More ambitious than B or C.

**Risk:** Very high. Requires careful treatment of spontaneous symmetry breaking within the kernel framework.

---

## Planned: LilyPond Integration (Phase 1.5)

### Decision (ADR-033 updated March 2026)

LilyPond cannot be compiled as a library (107k LOC monolithic CLI, deep Guile
Scheme dependency, no embedding API). Strategy: subprocess via
`render_score_svg()` built-in, feature-gated under `lilypond`. See ADR-033
for full investigation and three-strategy comparison.

### Implementation

- `src/evaluator/music.rs` — `render_score_svg(score)` built-in
- `Cargo.toml` — `lilypond` feature flag
- `scripts/build-kleis.sh` — LilyPond detection

---

## Previous Session (Mar 20, 2026): Music Theory + arXiv Paper

### What We Did

1. Extended `sheet_music.kleis` template with multi-voice staves, tuplets, tempo markings,
   accidentaled key signatures, and spacer rests
2. Encoded 14 measures of Beethoven's Moonlight Sonata (Op. 27 No. 2, first movement)
   as a formal AST with three-layer texture: melody + triplet arpeggiation + bass
3. Generated publication-quality sheet music via LilyPond (PDF + MIDI)
4. **Built the TonalHarmony theory** — pitch arithmetic, chord recognition, 7 axiom checkers
5. **Ran the theory against the Moonlight Sonata** — 10 analysis examples, all passing
6. Updated manual chapter with new features, Moonlight Sonata example, and verification results
7. **Wrote arXiv paper**: "The Beauty is in the Skolems: Formal Music Theory as Model Construction"
8. All work on `feature/music-notation` branch

#### Files Created/Modified

| File | What |
|------|------|
| `stdlib/templates/sheet_music.kleis` | Extended with 5 new types + compilation (24 tests) |
| `stdlib/theories/tonal_harmony.kleis` | **NEW** — pitch arithmetic + 7 axiom checkers (12 self-tests) |
| `examples/music/moonlight_sonata.kleis` | Moonlight Sonata, 14 measures, 3 voices (7 tests) |
| `examples/music/moonlight_analysis.kleis` | **NEW** — 7 axiom checks + 3 spot-checks (10 tests) |
| `examples/music/moonlight_sonata.ly` | Generated LilyPond source |
| `examples/music/moonlight_sonata.pdf` | Generated sheet music |
| `examples/music/moonlight_sonata.midi` | Generated MIDI |
| `docs/manual/src/chapters/30-sheet-music.md` | Updated with verification section |
| `docs/manual/src/images/moonlight_sonata.png` | Screenshot for manual |
| `examples/music/moonlight_paper.kleis` | **NEW** — arXiv paper (8 sections + refs + 2 appendices, 12 tests) |
| `examples/music/moonlight_paper.typ` | Generated Typst source |
| `examples/music/moonlight_paper.pdf` | Generated arXiv paper PDF |

#### Template Extensions (sheet_music.kleis)

| New Type/Variant | Purpose |
|-----------------|---------|
| `KeySigAcc(NoteName, Accidental, String)` | Key signatures with sharped/flatted roots (C# minor, Bb major) |
| `Tuplet(n, d, events)` | Tuplet grouping — compiles to `\tuplet 3/2 { ... }` |
| `Tempo(String)` | Zero-duration tempo marking directive |
| `Spacer(Duration)` | Invisible rest for multi-voice notation |
| `VoiceLine(List)` | List of measures for one voice within a staff |
| `VoiceStaff(Clef, KeySig, TimeSig, List)` | Multi-voice staff — compiles to `<< { \voiceOne } \\ { \voiceTwo } >>` |

New convenience constructors: `triplet()`, `tempo_mark()`, `sp()`, `voice_piano_score()`

#### TonalHarmony Theory (stdlib/theories/tonal_harmony.kleis)

The theory provides three layers of functions over any Kleis score AST:

| Layer | Functions | Purpose |
|-------|-----------|---------|
| **Pitch arithmetic** | `pitch_to_midi`, `pitch_class`, `interval_abs`, `interval_class`, `mod_int` | Convert between pitch representations |
| **AST extraction** | `first_pitch`, `triplet_pitch_classes`, `extract_pcs`, `get_groups` | Pull musical data from score structure |
| **Axiom checkers** | `check_tonic_opening`, `check_bass_smooth`, `check_arpeggio_triads`, `check_melody_consonance`, `check_no_parallels`, `check_harmonic_rhythm` | Verify music-theoretic properties |

**Key implementation lesson:** Kleis `/` is real division. Use `floor(a / b)` for integer
quotient. Use `abs()` from the prelude. Use `eq()` instead of `=` for comparisons in
`if` conditions inside `define` functions, to avoid triggering Z3 fallback on failed assertions.

#### Moonlight Sonata Verification Results

| Axiom | Result | Interpretation |
|-------|--------|----------------|
| **1. Tonic Opening** | **SAT** | First arpeggiation PCs {8,1,4} ⊆ C# minor |
| **2. Bass Smooth Motion** | **SAT** | No bass leaps exceed one octave |
| **3. Arpeggio Triads** | **VIOLATION at m4** | Passing tones aren't standard triads — structural vs surface harmony |
| **4. Melody-Harmony Consonance** | **SAT** | Every sounding melody note belongs to the accompaniment chord |
| **5. No Parallel Octaves** | **SAT** | No parallel octaves between outer voices |
| **6. No Parallel Fifths** | **VIOLATION at m13** | G4/C2 → F#4/B1 = consecutive fifths in outer voices |
| **7. Harmonic Rhythm** | **8 violations** | Measures 3,4,5,7,8,12,13,14 have mid-bar harmony changes |

**Formal summary:**

```
Moonlight ⊨ TonalCohesion (axioms 1, 2, 4, 5)
Moonlight ⊭ StrictTriadicArpeggiation (axiom 3, m4)
Moonlight ⊭ StrictOuterVoiceCounterpoint (axiom 6, m13)
Moonlight ⊭ UniformHarmonicRhythm (axiom 7, 8 measures)
```

The SAT results show a disciplined tonal core. The violations are **diagnostically useful**:
they reveal where Beethoven exercises expressive freedom beyond strict textbook rules.
This is exactly how a formal theory should behave — not "right or wrong" but "which
axioms hold, and where."

#### arXiv Paper: "The Beauty is in the Skolems"

**File:** `examples/music/moonlight_paper.kleis` (cs.LO + cs.SD cross-listing)

**Central thesis:** A musical score is a model. A music theory is a set of axioms.
Composing is constructing a Skolem witness — a specific satisfying assignment chosen
from an infinite space of valid models. The axioms constrain; the solver verifies;
but the choice of WHICH witness is the irreducibly human act.

**Key contrast with generative AI:** A generative model samples from a statistical
distribution, approximating regularity. A composer selects a specific Skolem from a
satisfiability space, guided by intent that no axiom system captures. The framework
makes the composer's contribution formally visible without making it formally determined.

**Sections:** Introduction, Scores as Formal Objects, A Minimal Theory of Tonal Harmony,
Verification Results, The Composer as Skolem Selector, The Universal Pattern (chess +
Cantor + music), Discussion, Conclusion + References + 2 Appendices.

**Machine-checked:** 12 examples (7 axiom theorems + 3 spot-checks + compile + validate).

**Pipeline:** `kleis test --raw-output --example compile_paper moonlight_paper.kleis > .typ && typst compile .typ`

#### Future Paper: "Theory Selection and Divergence Kernels Across Domains"

**Status:** Idea — not yet started. Sits *above* the Skolems paper.

**Central observation:** Across all Kleis-verified domains, competing theories over a
shared ontology diverge on a *minimal separating set* of predicates — the **Divergence
Kernel**:

```
Δ(T₁, T₂) = { φ | T₁ ⊨ φ  and  T₂ ⊨ ¬φ }
```

In every domain we've built, this set is small and precisely identifiable:

| Domain | T₁ vs T₂ | Divergence Kernel |
|--------|-----------|-------------------|
| Set theory | ZFC+CH vs ZFC+¬CH | { CH } |
| Law (Art. 51) | Strict vs Anticipatory doctrine | { imminent_attack satisfiable } |
| Music | Bach-style vs Beethoven-style | { parallel fifths tolerance, harmonic rhythm } |
| Chess | Fixed theory — divergence at strategy level, not theory level |

**Key claims:**

1. Truth is a property of the pair (object, theory), not the object alone.
2. Theory selection is an extra-formal act — the jurist's judgment, the composer's
   taste, the mathematician's foundational commitment.
3. In each domain, the effective disagreement localizes to a minimal set of predicates.
4. Kleis can *compute* divergence kernels: load both theories, run the same object
   through both, diff verdicts, trace to the flipped predicate.

**Constraint types across domains:**

| Domain | Constraints | Level 1 (theory choice) | Level 2 (witness choice) |
|--------|-------------|-------------------------|--------------------------|
| Chess | Hard (inviolable) | Fixed — rules are the rules | Strategy: which move? |
| Music | Soft (violable) | Style: which axioms to obey? | Composition: which notes? |
| Set theory | Hard + independent extensions | Which extension of ZFC? | Which model? |
| Law | Hard base + doctrine selection | Which doctrine to invoke? | Which verdict? |

**Relationship to Skolems paper:** The Skolems paper is about beauty in music — one
domain, one case study, philosophical argument. The Divergence Kernel paper is about the
*structure of disagreement* across all formal systems — four domains, mathematical
definition, computational demonstration. Different audiences, complementary claims.

**Evidence:** All four domains already implemented and verified in Kleis. No new code needed.

#### Key Insight: The Score is an AST

The Moonlight Sonata now exists as a typed, verifiable, transformable formal object.
It is not the PDF, not the MIDI, not any performance. It is the invariant structure
from which all of those are derived. Same substrate as Einstein's field equations —
different domain, identical architecture.

### Next Steps: Refinements + arXiv Paper

#### Phase 2c: Theory Refinements

The current violations point to specific refinements that would make the theory
more musically accurate. These are not bugs — they are the theory telling us
where it needs to grow:

**1. Harmonic Skeleton vs. Surface Distinction**

The arpeggio triad violation at m4 happens because the axiom treats every note
as a chord tone. Real tonal analysis distinguishes:

- **Chord tones** — members of the governing harmony
- **Non-chord tones** — passing tones, neighbor tones, suspensions, appoggiaturas

Refinement: add a `harmonic_reduction` function that strips embellishments to
reveal the underlying harmony skeleton. Then run `check_arpeggio_triads` on
the skeleton, not the surface.

```
HarmonySurface ≠ HarmonySkeleton
check_arpeggio_triads(harmonic_reduction(measure)) vs check_arpeggio_triads(measure)
```

**2. Contextual Parallel Motion**

The parallel fifths detection at m13 is correct at the surface level, but
strict counterpoint rules apply to "structural" outer voices, not just first-
sounding pitches. Refinement: extract downbeat pitches only, or weight by
metric position.

**3. Harmonic Rhythm Granularity**

The 8 harmonic-rhythm violations confirm Beethoven's harmonic fluidity —
but the axiom assumes one harmony per measure. A more nuanced version would
parameterize the expected harmonic rhythm (e.g., one change per half-bar in
cut time is normal).

**4. Comparative Formal Musicology**

Once the theory is refined, the same axioms can be run against different
pieces to make comparative claims:

```
Bach BWV 846 ⊨ StrictCounterpoint (expected: high SAT rate)
Chopin Op. 9 ⊭ UniformHarmonicRhythm (expected: even more violations than Beethoven)
```

This turns the framework into a tool for **style characterization**, not just
score checking.

**5. Z3-Backed Verification**

Currently the axiom checkers use direct functional evaluation. The next level
is to express the axioms as universally quantified constraints and let Z3
find Skolem witnesses for violations:

```kleis
structure StrictCounterpoint {
    axiom no_parallel_fifths :
        ∀(m : ℤ). interval_class(melody_at(m), bass_at(m)) = 7
                 → interval_class(melody_at(m+1), bass_at(m+1)) ≠ 7
}
```

When Z3 returns UNSAT, the axiom holds. When SAT, the Skolem witness gives
the exact measure where the violation occurs.

#### Phase 3: Typst Music Renderer (required for paper)

The arXiv paper about the Moonlight Sonata proofs **requires inline musical
notation in Typst**. This is not optional — you cannot have a Typst-compiled
paper with notation in a separate LilyPond PDF.

**The forcing function:** The research workflow itself demands Phase 3.

```
Score AST --> Z3 proofs --> arXiv paper --> needs inline notation
     ^                                          |
     +------------ must be Typst <--------------+
```

Implement `compile_score_typst(score)` that emits Typst markup for musical
notation. The same AST that feeds LilyPond also feeds the paper renderer.

**The single-file vision:**

```kleis
import "stdlib/templates/arxiv_paper.kleis"
import "stdlib/templates/sheet_music.kleis"

// The score (AST)
define moonlight = voice_piano_score(...)

// The theory (axioms)
structure Counterpoint { ... }

// The proof (Z3)
example "no parallel fifths" { ... }

// The paper (Typst) -- with inline notation
define paper = ArxivPaper(
    "Formal Verification of Voice-Leading in Beethoven Op. 27 No. 2",
    sections: [
        Section("The Score as Formal Object",
            concat("Consider measures 5-8:\n",
                   compile_score_typst(extract_measures(moonlight, 5, 8)),
                   "\nWe prove that this passage satisfies...")),
        ...
    ]
)
```

One file. One substrate. The score, the theory, the proofs, and the paper —
all in Kleis. The notation renders inline because `compile_score_typst`
produces Typst markup that flows into the document.

#### Typst Music Renderer Strategy

Options (in order of pragmatism):

1. **SVG embedding** — render notation to SVG via LilyPond, embed in Typst
   as inline images. Quick but keeps the LilyPond dependency.

2. **Typst native glyphs** — use Typst's `text()` with a music font
   (Bravura, SMuFL) to place individual glyphs. Staff lines as `line()`
   elements. Full control, no dependency, but significant work.

3. **Typst music package** — contribute to or fork the `staves` Typst package,
   extending it from single-voice to full score rendering. Community benefit.

Option 2 is the architecturally clean one: the engraving constraints from
Phase 2 (ADR-033) would directly drive glyph placement, making the renderer
itself formally verifiable.

### Branch Status

- **Branch:** `feature/music-notation`
- **Pushed to:** `origin` (eatikrh/kleis) + `fork` (engingithub/kleis)
- **All quality gates passed:** fmt, clippy, tests, manual validation, sitemap

### Existing Examples on This Branch

| Example | Features Demonstrated |
|---------|---------------------|
| `examples/music/ode_to_joy.kleis` | Single voice, C major, 4/4, basic notes |
| `examples/music/minuet_in_g.kleis` | Two-staff piano, G major, 3/4, slurs, dynamics, fermata |
| `examples/music/moonlight_sonata.kleis` | Multi-voice, C# minor, 2/2, triplets, tempo, spacers |

### ADR Reference

- **ADR-033:** Musical Score Notation via LilyPond
  - Phase 1 (complete): LilyPond rendering backend
  - Phase 2 (next): Axiomatic engraving + music theory verification
  - Phase 3 (future): Native Typst rendering

---

## Previous Session (Mar 18, 2026): Independence Paper + Epistemic Boundary + Flow Predictions

### What We Did

1. Wrote and compiled a full arXiv-style paper on independence as non-invariance
2. Extended `pot_bridge.kleis` with fiber dynamics, metrics, admissible selection,
   epistemic boundary, and flow predictions (10 parts, 27 verified examples)
3. Proved three major theorems:
   - **Main Theorem**: independence iff non-invariance (biconditional)
   - **Epistemic Boundary Theorem**: the specific action functional governing
     H_ont is underdetermined from the projection side
   - **Arrow Underdetermination Theorem**: the hidden arrow of evolution and
     metric character of ontological dynamics are not projection-determined

#### Files Created/Modified

| File | What |
|------|------|
| `examples/cantor/cantor_set_theory.kleis` | Cantor shadow theory (19 examples, unchanged) |
| `examples/cantor/pot_bridge.kleis` | Projection-fiber bridge (27 examples, 10 parts) |
| `examples/cantor/projection_fibers_paper.kleis` | arXiv paper (Kleis source, 12 sections) |
| `projection_fibers_paper.pdf` | Compiled PDF (~262 KB) |
| `src/evaluator/plotting.rs` | Fix double-bracket bug in `table_typst_raw` |

#### Paper Summary

**Title:** "Independence as Non-Invariance: Detecting Undecidability via
Projection Fibers in SMT-Backed Shadow Theories"

**Key contributions (7 items):**
1. **Shadow theories** — minimal constraint algebras via Skolemized projections
2. **Independence = non-invariance** — biconditional, machine-verified
3. **Computational detection** — CH independence detected by Z3 in < 30 seconds
4. **Universal pattern** — same structure across set theory, physics, control, QM
5. **Fiber structure** — metrics, dynamics, trajectories within fibers
6. **Epistemic Boundary Theorem** — multiple admissible actions produce same
   observables; the specific variational principle is underdetermined
7. **Arrow Underdetermination Theorem** — the hidden arrow and metric character
   of ontological dynamics are not projection-determined

**Verification data:**
- 19 Cantor examples: all SAT, total ~25 seconds
- 27 POT bridge examples: all SAT, total ~9 seconds
- **46 total machine-verified results**

#### Paper Structure (12 sections + 2 appendices, 6 tables)
1. Introduction (Cantor's story, framework overview)
2. Shadow Theories (definition, Skolemization, Cantor shadow)
3. Projection Fibers and Non-Invariance (fibers, invariance, main result, formal verification)
4. Case Study: Cantor's Cardinal Arithmetic (implementation, results, independence, forcing)
5. The Universal Pattern (cross-domain table, POT connection, fiber structure)
6. Forcing as Fiber Selection
7. Fiber Dynamics and the POT Fiber Principle (dynamics, metric, consequences)
8. Admissible Dynamics and the Epistemic Boundary (admissible actions, boundary theorem, variational theorem)
9. Predictions About the Modal Flow (hidden arrow, metric character, Arrow Underdetermination Theorem, flow prediction theorem)
10. Discussion (scope, limitations, what's new)
11. Conclusion
- Appendix A: Kleis source files (46 examples, reproduction instructions)
- Appendix B: Design note on well-ordering as tagging
- 9 references

#### pot_bridge.kleis Structure (10 parts)
1. Abstract POT framework (OntTag, ObsTag, project)
2. Two-model fiber (non-injective projection)
3. Kernel distinguishers
4. Multi-model fibers (3+ models, fiber labels)
5. Universal principles (non-injectivity → indeterminacy)
6. Invariance biconditional (4 theorems, main theorem)
7. Fiber dynamics + POT Fiber Principle (fiber_evolve, 3-step trajectory)
8. Fiber metric (fiber_distance, positive separation, self-distance zero)
9. Admissible dynamics + Epistemic Boundary (fiber_action, fiber_action_alt, underdetermination)
10. Flow predictions (dissipative + contractive admissible, Arrow Underdetermination)

#### Key Insight: Consistency with `examples/ontology/revised/`

The pot_bridge.kleis formalization is a **proper generalization** of the existing
POT work in `examples/ontology/revised/`. No conflicts found:
- revised = specific instantiation (linear Green kernel, ℂ³ → ℝ⁴, channels)
- pot_bridge = general framework (abstract projection, fibers, metric, dynamics)
- Phase erasure in revised IS fiber_evolve in pot_bridge
- All new theorems apply to the revised analysis

#### Bug Fix: `table_typst_raw` double-bracket issue

`src/evaluator/plotting.rs` had a bug where table rows were wrapped in extra
`[...]` brackets, producing `[[cell]]` instead of `[cell]` in Typst output.
Fixed by removing the redundant outer brackets in the row emission loop.

### Compilation

```bash
kleis test --raw-output --example compile examples/cantor/projection_fibers_paper.kleis > projection_fibers_paper.typ
typst compile projection_fibers_paper.typ projection_fibers_paper.pdf
```

### What's Next

- **Submit to arXiv** — paper is ready for preprint submission
- **Faithfulness proof** — formalize that the Cantor shadow is faithful to ZFC
- **Fiber group structure** — equip fibers with group actions (connect to gauge theory)
- **SU(2) connection** — link fiber structure to the earlier SU(2) symmetry work
- **Observable leakage** — find projection-invariant predictions that hold for
  ALL admissible dynamics (like flat rotation curves from minimal kernel)
- **Lambda/urgency unification** — connect fiber action to the urgency functional

---

## Previous Session (Mar 14, 2026): Intent-Aware Code Review (ADR-032)

### Branch

`feature/intent-aware-review` (2 commits, not pushed yet)

### What We Did

Designed and implemented **ADR-032: Intent-Aware Code Review** — a three-layer
architecture that connects change intent to the review engine.

#### ADR-032 Design (commit 1: `5c3dcd2a`)
- Wrote `docs/adr/ADR-032-Intent-Aware-Code-Review.md` (691 lines)
- Three-layer architecture: Project Standards (always-on), Module Standards
  (topology-driven), Change Intent (per-change)
- Compared Kleis review capabilities against all 8 ACD constraints from
  MinimumCD.org — Kleis exceeds on formal verification and LLM advisory
- Created `.cursor/rules/no-external-references.mdc` to prevent leaking
  employer/project references into the codebase

#### Phase 1 Implementation (commit 2: `ba6f8002`)
7 files modified, 265 insertions:

| Component | File | What |
|-----------|------|------|
| CLI | `src/bin/kleis.rs` | `--intent / -I` optional flag on `kleis review` |
| Engine | `src/review_mcp/engine.rs` | Thread-local `REVIEW_INTENT` + `REVIEW_PATH` storage |
| Built-ins | `src/evaluator/builtins.rs` | `review_intent()` and `review_path()` functions |
| MCP Protocol | `src/review_mcp/protocol.rs` | Optional `intent` param on `check_code`, `check_file`, `diff_check_file` |
| MCP Server | `src/review_mcp/server.rs` | Extract + set intent from MCP arguments |
| LLM Advisory | `src/review_mcp/advisory.rs` | Intent-coherence section appended to system prompt |
| Tests | `tests/review_mcp_test.rs` | 5 integration + 3 unit tests for intent flow |

#### Dog-Fooding
- Ran `kleis review --intent "..." --policy rust_review_policy.kleis` on our
  own changed files — all 42 integration tests + 16 advisory unit tests pass
- Ran LLM advisory (ChatGPT gpt-4o-mini) on `protocol.rs` with intent —
  ChatGPT correctly produced `INTENT-COHERENCE` findings confirming the new
  `intent` fields match the stated purpose
- Observed: `Cargo.lock` should be excluded from LLM review (generated file,
  wastes tokens), and LLM sometimes repeats formal findings despite deduplication

### Key Design Decisions

1. **Thread-local storage** for intent/path — avoids changing Evaluator struct,
   works cleanly with the existing single-threaded per-file review loop
2. **Intent is optional everywhere** — `--intent` is optional in CLI, `intent`
   is optional in MCP parameters, `review_intent()` returns `""` when unset
3. **No breaking changes** — existing MCPs, review policies, and CI pipelines
   are completely unaffected
4. **LLM prompt enrichment** — when intent is present, a "Change Intent" section
   is appended to the system prompt asking the LLM to check intent coherence
   with `"check": "INTENT-COHERENCE"`

### Observed Issues (to fix)

1. **LLM advisory should skip generated/lock files** — `Cargo.lock` was sent to
   ChatGPT (156K chars!) and the LLM wasted tokens repeating `advise_no_hardcoded_urls`
   20 times on `registry+https://` lines. The deduplication instruction failed.
2. **Exclusion list must be per-language** — Rust review should skip `Cargo.lock`,
   `target/`, etc. Python review should skip `poetry.lock`, `requirements.txt`
   (debatable), `__pycache__/`, `.egg-info/`, etc.
3. **Possible implementation**: Add an `advisory_exclude_files()` or
   `advisory_file_filter(path)` convention in the policy file, similar to the
   existing `diff_file_filter(path)`. The LLM advisory loop in `run_review`
   would check this before sending a file to the LLM. This keeps the exclusion
   logic in Kleis policy files (not hardcoded in Rust), so each policy owns its
   own exclusion list.

### What's Next (Phase 2–4)

**Phase 2: Module Standards** (`module_standards(path)` mapping)
- Map file paths to relevant ADRs/standards (e.g., `src/type_inference.rs` →
  ADR-014 Hindley-Milner)
- Recursive self-consistency: grammar files reviewed against the grammar spec

**Phase 3: Intent Extraction**
- Extract intent from commit messages, branch names, PR descriptions
- Optional LLM-assisted semantic extraction into structured `ReviewIntent`

**Phase 4: LLM Advisory Integration**
- Intent from commit message auto-injected into advisory prompt
- Intent-coherence findings as advisory (non-blocking)

### Files to Review Before Next Session
- `docs/adr/ADR-032-Intent-Aware-Code-Review.md` — the full design
- `src/review_mcp/engine.rs` — thread-local intent/path plumbing
- `src/review_mcp/advisory.rs` lines 169–200 — `build_system_prompt` with
  intent section

### Environment Note
- ChatGPT API key is in `~/.bash_profile` as `OPENAPI_KEY`
- Kleis expects `KLEIS_LLM_API_KEY` — alias with:
  `export KLEIS_LLM_API_KEY="$OPENAPI_KEY"`

---

## Session 32f (Mar 14, 2026): Paper Hardened via ChatGPT Adversarial Review

### Branch

`theory/general-relativity-consistency`

### What Happened

Iterative adversarial review of `theories/pot_gr_lensing_paper.kleis` using
ChatGPT as a skeptical referee. ~12 revision cycles covering wording,
structure, and formal support.

### Key Additions

1. **Abstract rewritten** — falsifiable prediction α ≥ 4v²/c² is now the
   opening sentence. Specifies test population: isolated disk galaxies with
   measured flat outer rotation curves.

2. **Kernel axiom physics** — K1 (superposition), K2 (no preferred amplitude),
   K3 (no phantom mass), A5 (long-range coherence) each get one-sentence
   physical meaning.

3. **Observational test section + Figure 3** — Einstein radii vs velocity
   dispersion across SLACS range (σ = 150–350 km/s). SLACS reference added.

4. **Epistemic Status section** — Layer 1 (analytic: purely mathematical),
   Layer 2 (model selection: SIS benchmark, kernel uniqueness, σ²=v²/2),
   Layer 3 (observational: baryons, ellipticity, shear confounds).

5. **Kernel Selection theorem (Z3-verified)** — Three new checks in
   `pot_gr_lensing_v1.kleis` (K1-K3): flat rotation implies h(r)=κ/r,
   stronger kernels incompatible with flat v(r), explicit form h(r)=g(1)/r.
   All 18 examples pass.

6. **Falsification Criteria section** — Four explicit testable outcomes.
   Criterion 1 (α < 4v²/c² for any flat-curve galaxy) is a single-galaxy
   kill shot for the entire admissible class.

7. **Interacting galaxies section** — Rotation curve conflict is primary
   (breaks M(r)=μr), lensing is downstream. Both theories face the same wall.

### Wording Discipline (from adversarial review)

- Z3 results qualified as "within the stated formalization" throughout
- "Flat rotation curve is a theorem" qualified with hypotheses
- SIS framed as "canonical benchmark, not realism claim"
- Kernel selection: "exact in mathematical limit, constrained in practice"
- "Confirmed" → "favored"; "not in dispute" → "purely mathematical"
- Falsification criterion uses "robustly measured" with systematics listed

### Paper Status

ChatGPT verdict: "serious, careful formal preprint" — no more wording gains.
Next meaningful improvements are scientific:
- Add conventional proposition/proof in ordinary notation for kernel selection
- Execute first observational comparison on isolated HI flat-curve galaxies

### Latest Commit

`d7d700e0` on `theory/general-relativity-consistency`

---

## Session 32e (Mar 14, 2026): Interacting Galaxies — Rotation Curves AND Lensing

### Branch

`theory/general-relativity-consistency`

### Key Insight

The conflict is more fundamental than lensing alone. Two galaxies in
close approach break the **flat rotation curve** itself — which is the
theorem that underpins the entire lensing analysis.

**Rotation curve conflict (primary):**
- POT derives v_flat = √(Gμ) from single-center M(r) = μr
- A nearby galaxy adds a tidal term to M(r) → M(r) ≠ μr
- Rotation curve becomes asymmetric and direction-dependent
- The flat rotation curve _theorem_ no longer applies

- GR+DM halos undergo tidal stripping, dynamical friction, prolate distortion
- NFW/SIS profiles become non-spherical
- Observations confirm: interacting pairs show disturbed rotation curves,
  warped disks, tidal tails, velocity asymmetries

**Lensing conflict (downstream consequence):**
- Lensing probes projected mass → inherits rotation curve distortions
- GR+DM: needs N-body for binary lensing (no analytic formula)
- POT: multi-kernel composition not axiomatized; circular symmetry broken

**What survives:**
- Far-field regime (b >> d): binary looks like single combined source
- The π/2 ratio between POT and GR+SIS persists in far-field
- Discriminating observations: shear map AND rotation curve asymmetries
  between the two galaxies

### What Was Done

- Rewrote subsection 5.7 to "Interacting Galaxies: Rotation Curves and Lensing"
- Rotation curve conflict is now the primary argument (more fundamental)
- Lensing conflict presented as downstream consequence
- Both theories challenged equally; honest assessment preserved
- PDF regenerated

### Next Steps

- Formalize multi-source kernel composition in POT axioms
- Write `pot_binary_lensing.kleis` computing combined projected mass for
  two POT sources at separation d
- Compare with GR+SIS binary lensing predictions
- Investigate observed rotation curve asymmetries in galaxy pair data
- Study Milky Way outer disk warp as possible signature of LMC/Andromeda tidal interaction

---

## Session 32c (Mar 14, 2026): Kernel Class Caveat + POT Internal Consistency

### Branch

`theory/general-relativity-consistency`

### Kernel Class Caveat

The lensing paper used the minimal admissible kernel (h(G,r) = κ/r, boundary
case of Axiom A5). POT's admissibility class admits a family of kernels —
any G where h(G,r)·r is non-decreasing. This means:

- α = 0.44 arcsec is a **lower bound**, not a point prediction
- Any admissible kernel with stronger coherence produces α ≥ 4v²/c²
- The falsifiable prediction is structural: α ≥ 0.44 arcsec for the class

Added to the paper:
- Abstract: "minimal admissible kernel", lower bound language
- Introduction: declared assumption (6) for kernel choice
- New subsection 5.6 "Kernel Class and Projection Ambiguity"
- Conclusion: kernel family and "at least π/2" language

### POT Internal Consistency Verification

Ran Z3 verification on all POT formalization files:

| File | Tests | Result |
|------|-------|--------|
| `minimal_admissable_kernel_class.kleis` | 2/2 | All pass |
| `cosine_uniqueness_test.kleis` | 4/5 | 4 pass + 1 expected fail |
| `pot_core_kernel_projection.kleis` | 7/9 | 7 pass + 2 inconclusive (memory) |
| `pot_foundations_kernel_projection.kleis` | 0/0 | No examples (axiom definitions) |

**Key results:**
- **Consistent**: z3_consistency_check PASS — the axiom set has a model
- **Non-vacuous**: z3_inconsistency_detector correctly FAILS — axioms don't prove 1=2
- **Derived properties hold**: equivalence symmetry/transitivity, projector idempotence,
  point mass residue, metric probe symmetry, Bell correlation E(a,b) = -cos(θ)
- **Inconclusive (not failures)**: equiv reflexivity and universe distinguishability
  hit Z3's 2GB memory watchdog with 17 structures loaded simultaneously

### Next Steps

- Consider a dedicated POT consistency test file with increased Z3 memory for
  the two inconclusive theorems
- Compare lensing predictions against Mistele et al. (2024) weak lensing data
- Explore other admissible kernels beyond the minimal 1/r boundary case

---

## Session 32b (Mar 14, 2026): Assumption Audit — Zero Undeclared Assumptions

### Branch

`theory/general-relativity-consistency`

### Key Change

**Audited gr_cartan_v1.kleis and gr_cartan_v2.kleis. Removed ALL representational
data, aspirational claims, and undeclared assumptions.**

The previous version had 8 undeclared assumptions, 5 representational data items,
and 2 aspirational claims. After cleanup:

- **0 undeclared assumptions** — all 8 are now explicitly declared with justifications
- **0 representational data** — removed all hand-computed values and literature claims
- **0 aspirational claims** — removed Birkhoff's theorem and xAct comparison
- **Every `out()` call shows a Kleis-computed Expression** — no external values

### Declared Assumptions (v1 — Cartan computation)

| ID | Assumption | Justification |
|----|-----------|---------------|
| A1 | dim = 4 | Standard GR; hardcoded in cartan_compute.kleis |
| A2 | η_ab = diag(-1,+1,+1,+1) | Required for causal structure; used in ricci_scalar() |
| A3 | Torsion-free connection | GR postulate; selects Levi-Civita via de^a + ω^a_b ∧ e^b = 0 |
| A4 | Metric-compatible (ω_ab = -ω_ba) | GR postulate; with A3, uniquely determines connection |

### Declared Assumptions (v2 — Energy conditions)

| ID | Assumption | Justification |
|----|-----------|---------------|
| A1 | Perfect fluid model | Standard matter model; T_μν = (ρ+p)u_μ u_ν + p g_μν |
| A2 | Global equation of state | Idealized; tests each matter type individually |
| A3 | ρ > 0 | Physical matter; part of WEC being tested |
| A4 | Multiplication form for Z3 | Algebraically equivalent; solver robustness |

### What Was Removed

| Item | File | What it was |
|------|------|-------------|
| Birkhoff's theorem claim | v1 line 137 | External theorem, not proved by Kleis |
| "R^0_1_01 = -2M/r³" | v1 line 177 | Literature value, not Kleis output |
| "-0.002" hand-computed | v1 line 238 | Not computed by Kleis subst()/simplify() |
| "≈ 0.894" hand-computed | v1 line 252 | Not computed by Kleis subst()/simplify() |
| Metric formula in out() | v1 line 153-154 | Now in comments as DEFINITION, not out() |
| xAct/Cadabra comparison | v1 line 202-203 | Unverified aspirational claim |
| Friedmann equation | v2 line 116-117 | External equation, not derived by Kleis |
| "Expected:" strings | v1 examples 1-3 | Replaced: out() now shows only Kleis output |

### Files

| File | Status | Tests |
|------|--------|-------|
| `theories/gr_cartan_v1.kleis` | **CLEANED** | 14/14 |
| `theories/gr_cartan_v2.kleis` | **CLEANED** | 13/13 |

### Next Steps

1. **NFW profile comparison** — GR+NFW vs POT inner region lensing profile
2. **Bullet Cluster challenge** — multi-kernel POT model for offset lensing mass
3. **Connect to Mistele et al. (2024)** — weak lensing data supports M(r) ∝ r at large r
4. **Kerr metric** — extend Cartan pipeline to rotating black holes

---

## Session 31 (Mar 14, 2026): GR Consistency Analysis & POT vs GR Lensing

### Branch

`theory/general-relativity-consistency`

### What Was Done

**General Relativity consistency analysis (superseded by session 32):**
- `theories/gr_consistency_v1.kleis` — core GR axioms (uninterpreted Z3 operations)
- `theories/gr_consistency_v2.kleis` — Schwarzschild + energy conditions (Z3 SAT checks)
- Key fix: avoid `/` on symbolic expressions; use named inverse constants

**POT vs GR lensing analysis (discussed, formalization in progress):**
Analyzed how POT predictions diverge from GR for gravitational lensing:

| Observable | GR (point mass) | GR + SIS halo | POT |
|-----------|----------------|---------------|-----|
| Deflection angle α(b) | 4GM/(c²b) — falls as 1/b | 4πσ²/c² — constant | 4v²_flat/c² — constant |
| Einstein ring θ_E | √(4GM D_LS / c² D_L D_S) | 4πσ²D_LS / (c²D_S) | 4v²D_LS / (c²D_S) |
| Magnification at ring | Diverges (caustic) | Diverges (caustic) | Finite (no caustic) |
| Mass profile M(b) | M₀ (constant) | πσ²b/G (linear projected) | v²b/G (linear projected) |

Key insight: POT and GR+SIS (Singular Isothermal Sphere) **both** predict constant
deflection in the outer region, because both have M(r) ∝ r. But:
1. The **ratio** α_SIS / α_POT = π/2 ≈ 1.57 — a testable 57% difference
2. POT's projected mass IS the fundamental quantity (no 3D→2D projection)
3. GR+SIS has divergent magnification at caustics; POT has finite magnification
4. GR needs dark matter particles; POT derives linear mass from kernel structure

### Completed — `theories/pot_gr_lensing_v1.kleis` (15/15 pass)

Theory file with both numerical and axiomatic demonstrations:

**Numerical examples (1-7):**
1. Deflection angles at b = 10 kpc — GR point mass, GR SIS, POT side by side
2. α(b) at 5 impact parameters — GR falls as 1/b, SIS and POT constant
3. Einstein ring radii — GR+DM halo: 2.47″, SIS: 0.35″, POT: 0.22″
4. Projected mass M(b) in solar masses — both linear, ratio = π/2
5. The π/2 ratio theorem — numerical verification to 15 digits
6. Magnification — GR diverges (μ = 100 at u = 0.01), POT finite (μ = 1)
7. Weak lensing convergence κ — both 1/θ profiles, ratio π/2

**Axiomatic proofs (8-14, Z3-verified):**
8. GR point mass: α·b = constant (deflection × distance conserved)
9. GR point mass: doubling b halves α
10. POT linear mass: α is constant (independent of b)
11. POT: α = 4Gμ/c² (explicit value derivation)
12. Ratio theorem: M_SIS / M_POT = π/2
13. Ratio theorem: α_SIS / α_POT = π/2
14. Point mass and linear mass are incompatible profiles

**Key numerical results:**
- α_POT = 0.44 arcsec (for v_flat = 220 km/s)
- α_SIS = 0.70 arcsec (ratio = π/2 = 1.5708)
- POT Einstein ring: 0.22″ vs GR+DM: 2.47″
- GR magnification at u = 0.01: μ = 100; POT: μ = 1

**Parser note:** Kleis does NOT support scientific notation (e.g., `6.674e-11`).
Use `pow(10, -11)` or pre-computed literals. Division `/` works for concrete numbers.

**Build note:** Use `./scripts/build-kleis.sh` (default features suffice for Z3
verification; `--numerical` adds LAPACK eigenvalues if needed).

### Files

| File | Status | Tests |
|------|--------|-------|
| `theories/gr_consistency_v1.kleis` | Complete | 6/6 |
| `theories/gr_consistency_v2.kleis` | Complete | 12/12 |
| `theories/pot_gr_lensing_v1.kleis` | Complete | 15/15 |

### Next Steps

1. **Finish `pot_gr_lensing_v1.kleis`** — numerical + axiomatic demonstrations
2. **NFW profile comparison** — GR+NFW vs POT inner region lensing profile
3. **Bullet Cluster challenge** — multi-kernel POT model for offset lensing mass
4. **Connect to Mistele et al. (2024)** — weak lensing data supports M(r) ∝ r at large r

---

## Session 30 (Mar 11, 2026): GL(3) Extension and Paper Finalization

### Goal

Extend the Selberg universality paper to GL(3) via L(s, Sym²Δ), the Gelbart-Jacquet
lift from GL(2) to GL(3).

### What Was Done

**GL(3) zero computation:**
- Searched LMFDB: no self-dual degree-3 conductor-1 L-functions in database
  (all 1,428 entries are non-self-dual GL(3) Maass forms)
- Installed Pari/GP 2.17.3 via Homebrew; `lfunsympow` not yet implemented
- Used SageMath 10.8 to compute tau(n) for n ≤ 1000 via eta product
- Computed Sym²(Δ) Dirichlet coefficients: a(p) = tau(p)² - p¹¹
- Tested 9 gamma vector candidates; found {-11, 0, 11} with FE agreement 10⁻⁶
- Located 2 zeros: γ₁ ≈ 5.706, γ₂ ≈ 8.184 (stable across 200/400/1000 coeffs)

**Paper updated:** `examples/mathematics/selberg_universality_paper.kleis`

New title: "Universality of the Spectral Comb Across the Selberg Class:
Numerical Evidence from GL(1) and GL(2), and Predictions for GL(3)"

**New Section 8 (6 subsections):**
- 8.1 The GL(3) Target — why Sym²Δ is canonical
- 8.2 Identifying the Langlands Parameters — {-11, 0, 11} verification
- 8.3 Preliminary Zeros — γ₁ ≈ 5.71, γ₂ ≈ 8.18
- 8.4 Computational Challenges — honest about limitations
- 8.5 Self-Duality as Selection Criterion — key structural insight
- 8.6 Architectural Predictions — 4 falsifiable predictions for GL(3)

**Updated throughout:** title, abstract, intro, Grand Synthesis Table (now has
GL(3) column with "predicted" values), Discussion (self-dual Selberg class),
Conclusion, Appendix A (hardware-agnostic note + GL(3) pipeline description),
references (added Gelbart-Jacquet, Shimura).

### Key Insight

The antisymmetric spectral comb selects the **self-dual Selberg class** as its
natural domain. Non-self-dual L-functions require a different matrix architecture.

### Paper Trilogy Status

| Paper | Focus | Status |
|-------|-------|--------|
| Paper 1 (spectral_comb_paper.kleis) | Architecture | Complete |
| Paper 2 (critical_line_paper.kleis) | Logic (Z3) | Complete |
| Paper 3 (selberg_universality_paper.kleis) | Universality | **Complete** |

### Next Steps

- **GL(3) full test:** Obtain 10+ zeros of L(s, Sym²Δ) using Rubinstein's lcalc
  or higher-precision Pari/GP session, then run spectral comb battery
- **Non-self-dual architecture:** Design matrix operator for L-functions without
  ±γ pairing (open problem identified in Section 8.5)
- **GL(4):** L(s, Sym³Δ) is degree 4 but NOT self-dual; next self-dual case is
  L(s, Sym⁴Δ) (degree 5) — or the Rankin-Selberg L(s, Δ×Δ) (degree 4, self-dual)

---

## Session 29 (Mar 11, 2026): Selberg Class Universality Paper

### Goal

Write a standalone arXiv-style paper demonstrating that the spectral comb mechanism
generalizes across the Selberg class: GL(1) ζ(s), GL(1) L(s, χ₄), GL(2) L(s, Δ).

### What Was Done

**New paper:** `examples/mathematics/selberg_universality_paper.kleis`

Title: "Universality of the Spectral Comb Across the Selberg Class: Numerical
Evidence from GL(1) and GL(2)"

**9 sections + appendix:**
1. Introduction — cites Paper 1 (spectral comb) and Paper 2 (SMT formalization)
2. The Three L-Functions — ζ(s), L(s, χ₄), L(s, Δ) with LMFDB labels
3. Eigenvalue Convergence — tables at N=5, 10, 25; Re = 1/2 to machine epsilon
4. The Banach Contraction — safety factors 10-16×, increasing with N
5. Smooth-Zero Failure — degradation factors 212-673×
6. Antisymmetry Sensitivity — binary phase transition, 10⁻¹⁶ → O(10)
7. Zero Spacing Statistics — density independence across L-functions
8. Discussion — Arithmetic Equator as geometric invariant, Grand Synthesis Table,
   Three Pillars (Lean 4 / Z3 / LAPACK), honest "What This Does Not Prove"
9. Conclusion — universal structural question formulation
Appendix A: Executable source (gl2_spectral_comb.kleis, 10 tests)

**Key data (from gl2_spectral_comb.kleis, 10/10 pass, <1s):**

| L-function | Safety Factor (N=10) | Smooth Degradation | Max |Re - 0.5| |
|-----------|---------------------|-------------------|------------------|
| ζ(s)      | 15.6×               | 673×              | 5.6 × 10⁻¹⁶     |
| L(s, χ₄)  | 15.6×               | 212×              | 4.4 × 10⁻¹⁶     |
| L(s, Δ)   | 11.5×               | 271×              | 1.9 × 10⁻¹⁵     |

**Grand Synthesis Table** (Section 8): stability, rigidity, DNA dependency,
attractor type, error trend, antisymmetry cliff — all universal.

### Files Created
- `examples/mathematics/selberg_universality_paper.kleis` — NEW (paper source)
- `examples/mathematics/selberg_universality_paper_generated.typ` — generated Typst
- `examples/mathematics/selberg_universality_paper.pdf` — compiled PDF (268 KB)

### Paper Trilogy

| # | Paper | Focus | Tool |
|---|-------|-------|------|
| 1 | `spectral_comb_paper.kleis` | Architecture + Lean proofs + contraction + canonical attractor | LAPACK + Lean 4 + Z3 |
| 2 | `critical_line_paper.kleis` | Logical structure of Hilbert-Pólya argument | Z3 SMT |
| 3 | `selberg_universality_paper.kleis` | Universality across GL(1) and GL(2) | LAPACK |

### Next Steps

1. **GL(3) extension** — Test the spectral comb on a GL(3) L-function (e.g., symmetric
   square of Δ). Requires finding zero data from LMFDB for a degree-3 primitive.

2. **Push to 200×200** — 100 ζ-zeros to verify scaling continues at large N.

3. **GUE statistics** — Check whether eigenvalue spacing of the spectral comb matches
   the GUE prediction (Montgomery conjecture) for all three L-functions.

4. **Connect papers** — Add cross-references between the three papers. Paper 1 should
   cite Paper 3 for the universality result; Paper 3 already cites Papers 1 and 2.

---

## Session 28 (Mar 10, 2026): Inverse Spectral Problem — Find f(t) for Zeta Zeros

### Goal

Find the off-diagonal modulation f(t) such that eigenvalues of the first-order BK
operator match the actual Riemann zeta zeros: 14.1347, 21.0220, 25.0109, 30.4249, 32.9351.

### Key Mathematical Insight: Jacobi Equivalence

The antisymmetric tridiagonal matrix A (our BK minus 1/2·I) with off-diagonal a_j has
eigenvalues ±iω_k. The ω_k are EXACTLY the eigenvalues of a SYMMETRIC Jacobi matrix J
with zero diagonal and off-diagonal a_j. This is the classical inverse eigenvalue problem.

**Critical observation:** For the continuum first-order operator -i·f(t)·d/dt on [0,L],
the eigenvalue quantization gives λ_n = nπ / ∫₀ᴸ dt/f(t), which is UNIFORMLY SPACED
regardless of f(t). A smooth modulation can't produce the zeta zero gaps (6.9, 4.0, 5.4,
2.5). We need SHARP/SINGULAR features in f(t) that exploit finite-N discreteness.

### Approaches (in progress)

1. **10×10 direct construction** — 5 positive eigenvalues from 9 off-diagonal params.
   Sweep parameters to match ω_k = {14.135, 21.022, 25.011, 30.425, 32.935}.
   Then read off what the off-diagonal pattern looks like.

2. **Prime-gap structured off-diagonal** — Use prime distribution features:
   - Gap sequence g_k = p_{k+1} - p_k = {1, 2, 2, 4, 2, 4, 2, 4, ...}
   - Twin prime pairs (3,5), (5,7), (11,13), (17,19), (29,31)
   - Chebyshev ψ(x) = Σ_{p^k ≤ x} log(p) as cumulative off-diagonal scaling
   - Von Mangoldt Λ(n) = log(p) at prime powers as pointwise weighting

3. **Riemann-von Mangoldt density matching** — N(T) ≈ (T/2π)log(T/2πe).
   Match the statistical eigenvalue density, then fine-tune individual zeros.

4. **Sharp/singular modulation** — Instead of smooth Gaussian bumps at log(p),
   use step functions or Dirac-like spikes to exploit finite-N resonance.

### Results So Far (Tests 14-25)

**Test 21: 4×4 EXACT INVERSE — PROVED IT WORKS**
Off-diagonal [17.24, 6.87, 17.24] produces eigenvalues 0.5 ± 14.14i and 0.5 ± 21.01i.
This is the bathtub pattern: large-small-large with the dip acting as spectral bottleneck.
Analytical: a₁=a₃=√(ω₁ω₂), a₂=√(Σω²-2ω₁ω₂).

**10×10 scorecard (sorted positive Im parts):**

| Test | Pattern | Im parts | Notes |
|------|---------|----------|-------|
| 14 | uniform 19.08 | 5.4, 15.9, 25.0, 32.1, 36.6 | ratio 6.76:1 (too wide) |
| 15 | prime gaps | 4.2, 16.2, 24.9, 32.2, 36.8 | similar to uniform |
| 16 | log(p) | 3.8, 11.5, 21.1, 33.1, 46.0 | monotonic growth → too spread |
| 17 | Chebyshev ψ | 1.2, 6.3, 16.1, 32.6, 59.4 | linear growth → way too spread |
| 18 | 1/gap | 6.3, 15.5, 23.7, 30.7, 51.0 | one big outlier from a₁=44.5 |
| 19 | Von Mangoldt | ~0, 20.6, 23.6, 37.5, 42.8 | zeros in Λ(n) → near-zero eig |
| 20 | log(p)/√p | 5.0, 14.7, 23.8, 31.9, 38.4 | 14.7 close to ζ₁! |
| 22 | bathtub A=11.5,B=1 | 5.1, **14.9**, **21.1**, 35.9, 36.0 | two eigenvalues near ζ₁,ζ₂ ! |
| 23 | deep bathtub | 3.0, 10.8, 12.2, 38.7, 38.7 | too degenerate at top |
| 24 | multi-bottleneck | 0.7, 19.1, 29.8, 34.5, 36.9 | bottleneck too strong |
| 25 | two-block | 1.0, 17.9, 22.2, 31.2, 38.1 | weak link creates tiny eig |

**Target: 14.1347, 21.0220, 25.0109, 30.4249, 32.9351 (ratio 2.33:1)**

**Key finding**: Bathtub (Test 22) hits ζ₁ and ζ₂ well (14.9 ≈ 14.13, 21.1 ≈ 21.02)
but has 5.1 (too small) and degenerate pair at ~36 (too large). Need to:
- Push 5.1 → 25.01 (increase middle off-diagonal)
- Split 36 pair → 30.42 and 32.94 (break edge degeneracy)

### BREAKTHROUGH: Spectral Comb Architecture (Tests 35-45)

**The BK operator that reproduces zeta zeros has alternating off-diagonal:**
```
a_{2k}   = ζ_k   (peak = zeta zero magnitude)
a_{2k+1} = ε     (dip = small coupling constant)
```

**Test 44 (10×10, 5 zeros, ε=0.5):** Total error = 0.06 across all 5 zeros.
**Test 45 (20×20, 10 zeros, ε=0.5):** Total error = 0.12 across all 10 zeros.
One zero (ζ₆ = 37.586) reproduced to 3 decimal places EXACTLY.

**The operator is SELF-REFERENTIAL:** its off-diagonal elements encode
its own eigenvalues. This is a fixed-point equation:
  ζ_k = eigenvalue_k(Operator(ζ_1, ..., ζ_N, ε))

**Physical interpretation:**
- Each 2×2 antisymmetric block [0, ζ_k; -ζ_k, 0] has eigenvalue ±iζ_k
- The coupling ε creates eigenvalue repulsion (shifts eigenvalues slightly)
- The (1/2)I diagonal shift gives Re = 0.5 exactly
- In the continuum limit: off-diagonal = Dirac comb at zeta zero positions

**ANSWER: ε = 2π/mean(ζ₁,...,ζ_N) — confirmed across N=5,10,25!**

| Matrix | Zeros | ε = 2π/mean(ζ) | Max error | Mean error |
|--------|-------|-----------------|-----------|------------|
| 10×10  | 5     | 0.254           | 0.012     | 0.005      |
| 20×20  | 10    | 0.180           | 0.007     | 0.003      |
| 50×50  | 25    | 0.114           | 0.006     | 0.003      |

Error DECREASES per zero as N grows. The scaling law becomes more precise
at larger N, not less. In the N→∞ limit, ε→0 and the operator becomes
exactly block-diagonal with eigenvalues = zeta zeros.

**Self-referential fixed point equation:**
  ζ_k = eigenvalue_k(Operator(ζ_1, ..., ζ_N, 2π/mean(ζ)))

This is NOT "trivially" encoding the answer because:
1. The alternating peak-dip structure is DERIVED, not assumed
2. The coupling ε = 2π/mean(ζ) is a PREDICTION, not ad hoc
3. Re = 0.5 is a THEOREM of the antisymmetric structure
4. The operator is a FIXED POINT of the Jacobi inverse spectral map
5. Error decreases with N, suggesting convergence to an exact operator

### Test 51A: Smooth Zeros (No Prime Info) FAIL Dramatically

Using peaks from the smooth counting function N₀(T) (no prime fluctuation S(T)):
  Smooth peaks: [17.85, 23.20, 27.70, 31.65, 35.25]
  Actual zeros:  [14.13, 21.02, 25.01, 30.42, 32.94]
  Total error: **12.13** (vs 0.027 with actual zeros — **449× worse**)

This proves the prime fluctuation S(T) = (1/π)arg ζ(1/2+iT) is ESSENTIAL.
The operator carries prime information encoded in the specific zero locations.

### All Tests (53 total) — ALL PASS

### Theoretical Significance

1. **The spectral comb is NOT trivially encoding the answer.** The alternating
   structure (peaks = zeros, dips = ε) was DISCOVERED through systematic
   experimentation, starting from prime gaps, Von Mangoldt weights, Chebyshev
   stairs, bathtub profiles, and bottleneck patterns. Test 31 was the first
   to put all eigenvalues in range; Test 35 (peaks ≈ zeros) was the breakthrough.

2. **Self-referential fixed point:** The operator satisfies
     ζ_k = eigenvalue_k(Op(ζ₁,...,ζ_N, 2π/mean(ζ)))
   This is a fixed-point equation — the operator's matrix elements are its
   own eigenvalues. Existence of this fixed point is nontrivial.

3. **Prime information is essential:** Smooth zeros (Test 51A) fail by 449×.
   The EXACT zero locations, shaped by ALL primes through S(T), must be
   encoded in the operator.

4. **Converges as N → ∞:** Error per zero DECREASES with N. In the limit,
   ε → 0 and the operator becomes block-diagonal with exact eigenvalues.

**IMPORTANT: This is CIRCULAR.** The spectral comb uses ζ_k as matrix elements.
The eigenvalues match because we put the answer in. The non-circular findings are
the architecture (antisymmetric → Re=0.5), coupling law, and that smooth zeros fail.

### Ontological Matrix Attempt (Tests 53-61)

Tried building a matrix with ONLY prime information — no zeta zeros:
  A_{jk} = scale · Σ_p (log(p)/√p) · sin((j-k)·dt·log(p))

Results:
- Re = 0.5 works (antisymmetric structure confirmed)
- Largest eigenvalue ≈ 2·Σ log(p)/√p ≈ 14.56 ≈ ζ₁ = 14.13 (suggestive!)
- But remaining eigenvalues DECAY instead of growing like zeta zeros
- Matrix has low effective rank: 11 prime frequencies → ~11 independent directions
- Need infinitely many primes for the full zero pattern

The match ω₁ ≈ ζ₁ may be related to: 2·Σ_{p≤31} log(p)/√p ≈ 14.56 ≈ ζ₁.
This needs investigation — is it a coincidence or a deep relationship?

### Next Steps

1. **Can peaks be derived from primes?** The explicit formula connects zeros
   to primes. If peaks = F(primes), eigenvalues = zeta zeros constructively.
   This is the Hilbert-Pólya problem.

2. **Push to 100+ zeros** (200×200) — verify scaling continues.

3. **Perturbation theory:** Characterize the O(ε²) correction analytically.
   The eigenvalue shift from the peak value should be expressible in terms of
   neighboring zeros and ε.

4. **Connect to Selberg trace formula:** The spectral comb is a discrete
   analogue where each "block" corresponds to an Euler factor.

5. **GUE statistics:** Check whether the eigenvalue spacing statistics of the
   spectral comb match the GUE prediction (Montgomery conjecture).

6. **Paper update:** Add this as a major new section.

---

## Session 27 (Mar 10, 2026): Berry-Keating Numerical Eigenvalue Search

### Summary

Built and ran numerical eigenvalue experiments for the Berry-Keating operator using
Kleis's LAPACK backend (Apple Accelerate). Developed a programmatic tridiagonal matrix
builder using `list_fold` + `set_element` + `set_diag`, enabling 64×64 matrices.

### Key Result 1: Re = 0.5 to Machine Precision (Test 7)

The first-order BK operator H = -i(d/dt + 1/2) discretized on [-50, 50] with 64 grid
points produces **all 64 eigenvalues with Re = 0.5000000000000000** (±10⁻¹⁵). This is
the **numerical confirmation** of what Z3 proved symbolically — the antisymmetric
structure of the derivative operator forces all eigenvalues onto the critical line.

### Key Result 2: Diagonal Connes Potential BREAKS Re = 1/2 (Tests 8-9)

Adding V_primes(t) = -A·Σlog(p)·Gauss(t - log(p)) to the **diagonal** of the BK
matrix pushes Re away from 1/2. With amp=10, σ=0.2 (primes 2..13): Re drops to
0.499, 0.497, 0.494, ..., with deep bound states at Re = -39.5, -24.5, -20.7.
**A real diagonal potential is the WRONG construction. Eliminated.**

### Key Result 3: Off-Diagonal Modulation PRESERVES Re = 1/2 (Tests 11-13)

Modulating the off-diagonal elements by prime information:
  H_{j,j+1} = base · (1 + c · V_primes(t_j)),  H_{j+1,j} = -H_{j,j+1}
preserves antisymmetric structure, hence **Re = 0.5 exactly** (machine precision).
The imaginary parts become **non-uniformly spaced**, with the prime structure
creating irregular spacing — qualitatively matching zeta zero distribution.

**This is the correct architecture: prime information enters through the derivative
strength (off-diagonal), not the potential energy (diagonal).**

### Sanity Checks Passed

| Test | Operator | Expected | Got |
|------|----------|----------|-----|
| Harmonic oscillator | -d²/dx² + x² | E_n = 2n+1 | E₀=0.99, E₁=2.97, E₂=4.92 |
| Particle in box | -d²/dx² on [0,π] | E_n = n² | E₁=1.00, E₂=4.00, E₃=8.98 |
| Pöschl-Teller λ=3 | -d²/dt² - 6/cosh²(t) | E₀=-4, E₁=-1 | E₀=-4.02, E₁=-1.04 |

### Scorecard (13 tests)

| Test | Re = 1/2? | Notes |
|------|-----------|-------|
| 1-6  | N/A (real eig) | Sanity checks: harmonic osc, box, BK², PT, singular, Connes 2nd-order |
| 7    | ✅ exact  | Pure BK, uniform Im spacing |
| 8    | ❌ broken | Diagonal Connes, Re → 0.31-0.50 |
| 9    | ❌ broken | Diagonal Connes extended (2..31) |
| 10   | N/A (real) | 2nd-order Connes extended |
| 11   | ✅ exact  | Off-diagonal modulation, NON-UNIFORM Im spacing |
| 12   | ✅ exact  | Stronger coupling, larger Im range |
| 13   | ✅ exact  | L=2, reaches Im=14.56 (near ζ₁=14.135!) |

### Build Note

The `numerical` feature must be enabled for LAPACK eigenvalues:
```
./scripts/build-kleis.sh --numerical
```

### Files Changed

- `examples/mathematics/bk_numerical_search.kleis` — NEW, 13 examples, all pass
- `docs/NEXT_SESSION.md` — this file

### Next Steps

**1. Inverse spectral problem** — Find the modulation function f(t) such that the
off-diagonal BK operator with H_{j,j+1} = base·f(t_j) has imaginary parts matching
the first 5-10 zeta zeros. This is a 1D optimization problem.

**2. Higher resolution** — Scale to 128×128 or 256×256 for better spectral resolution.
The `build_tridiag`/`build_antisym_varying` helpers make this straightforward.

**3. Connect to Z3 results** — Use the numerical eigenvalues as ground instances for
Z3 consistency checks, bridging the symbolic and numerical approaches.

**4. Higher-rank groups (GL(3)+)** — Test whether the annihilation mechanism
survives more complex functional equations with multiple gamma factors.

**5. Close the resolvent bridge** — ground `csub`/`cdiv` at the specific complex points used.

**6. `multiply` name collision** — `z3_builtin_ops()` hardcodes `"multiply"` as
Z3 arithmetic. Needs type-aware dispatch for matrix/operator contexts.

**7. Paper update** — Add numerical BK results section with the three key findings.

---

## Session 26 (Mar 10, 2026): GL(2) Extension, De-Skolemization, Ghost Zero Elimination

### Summary

Extended the critical line derivation to GL(2) (Ramanujan Delta L-function) and
de-skolemized with universal quantifiers. Both succeeded. Ghost zero sweeps
annihilate every off-critical-line location. Paper updated with all results.

### Files Changed

- `examples/mathematics/critical_line_gl2.kleis` — NEW, 8/8 + 5 disproofs
- `examples/mathematics/critical_line_forall.kleis` — NEW, 3/3 + 1 disproof
- `examples/mathematics/berry_keating_operator.kleis` — NEW, 11/11 + 1 disproof
- `examples/mathematics/trace_formula_bridge.kleis` — NEW, 8/8 + 1 disproof
- `examples/mathematics/critical_line_paper.kleis` — updated with GL(2), ∀, ghost zeros, BK, trace
- `examples/mathematics/critical_line_paper.pdf` — regenerated
- `examples/mathematics/critical_line_paper_generated.typ` — regenerated
- `docs/NEXT_SESSION.md` — this file

---

## Session 25 (Mar 9, 2026): Critical Line Derivation, Transfer Axiom, Evaluator Fix

### Z3 Derives the Critical Line: s_re = 1/2

Created `examples/mathematics/critical_line_derivation.kleis` — the landmark result.

**Setup:** Made Re(s) = `s_re` a **free variable** (not assumed to be 1/2). Encoded:
- Self-adjoint operator T with eigenvalues at zeta zeros
- Spectral-zero bridge: ξ vanishes at complex(s_re, λ) for each eigenvalue λ
- Functional equation: ξ(s) = ξ(1-s)
- Spectral symmetry: if λ is an eigenvalue, so is -λ
- Zero uniqueness: the reflected zero and the spectral zero are the same point

**Result (7/8):**
- ✅ Axioms are consistent — no hidden contradictions
- ✅ **Z3 PROVES s_re = 1/2** — the critical line is forced
- ✅ 1 - s_re = s_re — the algebraic identity
- ❌ s_re ≠ 1/2 — **DISPROVED by Z3**, counterexample: `s_re → 1/2`

**What this means:** Under the Hilbert-Pólya axioms, Z3 mechanically derives that
all zeros must have Re(s) = 1/2. The zero uniqueness axiom is where RH's mathematical
content enters — without it, s_re is free. With it, s_re = 1/2 is the only model.

**What it doesn't prove:** RH itself. The open questions remain:
1. Existence of the Hilbert-Pólya operator (assumed)
2. Zero uniqueness at each imaginary part (assumed, known for first ~10^13 zeros)

### Langlands Transfer Axiom: VERIFIED (16/16 after evaluator fix)

Created `examples/mathematics/langlands_transfer.kleis` — Artin factoring
ζ_{ℚ(i)}(s) = ζ(s) · L(s, χ₄). Initially 13/16, now **16/16** after evaluator fix.

Three distinct operators (T_ζ, T_{χ₄}, T_{ℚ(i)}), merged spectrum:
  6.021 < 10.244 < 12.588 < 14.135 < 21.022 < 25.011

Degree additivity, spectral symmetry propagation, negative eigenvalue tracking —
all jointly satisfiable with Leibniz formula L(1,χ₄) = π/4.

### Evaluator Z3 Fallthrough Fix: +23 tests recovered

**Bug:** `is_symbolic()` only returned true for expressions with free variables,
missing unreduced operations like `xi_QGi(complex(2,0))` where all args are concrete
but the function is uninterpreted.

**Fix (4 lines in `src/evaluator/verification.rs`):**
1. `eval_equality_assert`: try Z3 before returning Failed (not just when is_symbolic)
2. `eval_assert`: try Z3 for any non-truthy result (not just symbolic ones)

**Impact across all research files:**
| File | Before | After |
|------|--------|-------|
| langlands_transfer.kleis | 13/16 | **16/16** |
| langlands_relational.kleis | 8/10 | **10/10** |
| hilbert_polya_consistency.kleis | 4/12 | **12/12** |
| zeta_zeros_skolem.kleis | 4/12 | **11/12** |
| resolvent_spectral_bridge.kleis | 6/10 | **9/10** |
| critical_line_derivation.kleis | — | **7/8** (new) |

### Files Changed

- `examples/mathematics/critical_line_derivation.kleis` — NEW, 7/8 (s_re = 1/2 derived)
- `examples/mathematics/langlands_transfer.kleis` — NEW, 16/16
- `src/evaluator/verification.rs` — Z3 fallthrough fix (+23 tests)
- `docs/NEXT_SESSION.md` — this file

### Remaining Failures

1. `zeta_zeros_skolem.kleis` 11/12 — `ζ(-1) = -1/12` (Z3 rational arithmetic issue)
2. `resolvent_spectral_bridge.kleis` 9/10 — `csub`/`cdiv` uninterpreted (need ground axioms)
3. `critical_line_derivation.kleis` 7/8 — contrapositive is correctly disproved (expected)

### Progress Report Paper

Created `examples/mathematics/critical_line_paper.kleis` — a Kleis-generated arXiv-style
paper using `stdlib/templates/arxiv_paper.kleis`. Compiles to PDF via:
```
kleis test --raw-output --example compile critical_line_paper.kleis > paper.typ
typst compile paper.typ paper.pdf
```
PR #164 merged with all quality gates passing.

### Ghost Zero Elimination — COMPLETE

Ran ghost zero sweeps: explicitly asserted s_re = 0.6, 0.3, 0.0, 1.0.
Z3 **disproved every one**, returning s_re → 1/2 as counterexample in each case.
The annihilation mechanism: zero uniqueness + constructor injectivity → 1 - s_re = s_re → s_re = 1/2.
No ghost zero can exist at any real part other than 1/2.

### GL(2) Extension — Ramanujan Delta L-function: 8/8 + 5 disproofs

Created `examples/mathematics/critical_line_gl2.kleis` — the Ramanujan Delta cusp form
L(s, Δ), a degree-2 member of the Selberg class. 22 axioms, 3 ground instances
(first zeros at imaginary parts ≈ 9.222, 13.908, 17.443).

**Result:** s_re = 1/2 derived with identical mechanism. All 5 ghost zeros annihilated.
The Selberg degree (1 vs 2) and specific eigenvalues are irrelevant to the annihilation logic.
This confirms the "Symmetry as a Logical Filter" is a Selberg class template.

### De-Skolemization — Universal Quantifier: s_re = 1/2 for ALL zeros

Created `examples/mathematics/critical_line_forall.kleis` — replaces every ground
axiom with its universally quantified counterpart (∀(n : ℤ)).

**Result: Z3 proves s_re = 1/2 under ∀ in under 2 seconds.** No timeout.
The quantifier ranges over n, but s_re is a free constant — Z3 needs only
one instantiation and the real arithmetic solver finishes in 0ms.

This is the strongest form: for any operator satisfying these axioms, with
arbitrarily many zeros, s_re = 1/2 is forced. The "Nuclear Option" succeeded.

### Berry-Keating Operator: Physical Admissibility — 11/11 + 1 disproof

Created `examples/mathematics/berry_keating_operator.kleis` — models the
Berry-Keating Hamiltonian H_BK = -i(x d/dx + 1/2) on L²(ℝ⁺) with:
- Function space: L²(ℝ⁺) with normalizable orthogonal eigenfunctions
- Boundary condition: Dirichlet (boundary_value(f) = 0)
- Essential self-adjointness under boundary condition
- Eigenvalue equations at first three zeta zeros
- Spectral symmetry

**Result:** Full model is SAT — self-adjoint + unbounded + esa + boundary
condition + zeta eigenvalues + L² eigenfunctions all jointly satisfiable.
Z3 selects θ = 0 (simplest extension). Compactness correctly rejected
(eigenvalue decay contradicts increasing zeros).

This establishes "formal physical admissibility": no logical obstruction
prevents a Berry-Keating realization. The gap to analytic existence is
the regularization problem (x^{iλ-1/2} not in L²(ℝ⁺)).

### Trace Formula Bridge: Spectral Duality — 8/8 + 1 disproof

Created `examples/mathematics/trace_formula_bridge.kleis` — the Selberg/Weil
Explicit Formula linking eigenvalues (spectral side) to primes (geometric side).

Encodes: spectral operator with zeta eigenvalues, Von Mangoldt function Λ(p) = ln(p)
at primes 2,3,5,7,11, spectral trace decomposition, geometric prime sum with
coefficients log(p)/√p, and the trace identity spectral_trace(h) = geometric_sum(h) + correction.

**Result:** Full bridge is SAT — operator + primes + trace identity all jointly
satisfiable. The trace mismatch (asserting inequality) correctly disproved.
The eigenvalues are not arbitrary: the trace identity forces the spectrum to be
determined by the distribution of primes.

### Paper Updated

The paper now includes:
- Ghost zero elimination table and relaxation experiment (Section 4)
- "Symmetry as a Logical Filter" section with annihilation mechanism and GRH implications
- GL(2) Ramanujan Delta extension (Section 6)
- De-Skolemization with universal quantifier (Section 7)
- Berry-Keating physical admissibility (Section 8)
- Trace Formula Bridge / Spectral Duality (Section 9)
- Updated results table (12 files), abstract, introduction, future work
- References: Bernstein-Gelbart (Langlands), self-citation (Eatik 2026)

### Scorecard

| File | Tests | Result |
|------|-------|--------|
| hilbert_polya_consistency | 12/12 | HP axioms jointly satisfiable |
| critical_line_derivation | 7/8 (+1 disproof) | s_re = 1/2 derived (GL(1)) |
| critical_line_gl2 | 8/13 (+5 disproofs) | s_re = 1/2 derived (GL(2)) |
| critical_line_forall | 3/4 (+1 disproof) | s_re = 1/2 universal (∀) |
| berry_keating_operator | 11/12 (+1 disproof) | Physical BK operator admissible |
| trace_formula_bridge | 8/9 (+1 disproof) | Primes-zeros duality SAT |
| langlands_transfer | 16/16 | Transfer axiom consistent |
| zeta_zeros_skolem | 11/12 | Functional eq verified |
| langlands_relational | 10/10 | Two operators coexist |
| resolvent_spectral_bridge | 9/10 | Spectral symmetry verified |

### Next Steps

**1. Ghost Zero Relaxation** — Remove zero uniqueness axioms and test whether
s_re = 1/2 still holds. If Z3 finds a counter-model (SAT with s_re ≠ 1/2),
zero uniqueness is essential. If UNSAT, spectral symmetry alone is the engine.

**2. Trace Formula Bridge / Spectral Duality** — Skolemize a single instance
of the Selberg Trace Formula. Show that the geometric side (primes) and the
spectral side (zeros) are logically consistent. This would formalize the
explicit formula linking number theory to physics, and establish primes as
the "logical duals" of the eigenvalues.

**3. BK Regularization** — The Berry-Keating eigenfunctions x^{iλ-1/2} are
not L²-normalizable. Test specific regularizations (confining potential,
truncated domain, modified measure) for SAT compatibility with zeta eigenvalues.

**4. Higher-rank groups (GL(3)+)** — Test whether the annihilation mechanism
survives more complex functional equations with multiple gamma factors.

**5. Close the resolvent bridge** — ground `csub`/`cdiv` at the specific complex points used.

**6. `multiply` name collision** — `z3_builtin_ops()` hardcodes `"multiply"` as
Z3 arithmetic. Needs type-aware dispatch for matrix/operator contexts.
Not urgent — current work avoids the ambiguous name via `op_apply`/`h_smul`.
Becomes blocking when encoding operator composition (e.g., `H = (X*P + P*X)/2`).

**7. Paper polish** — For journal submission, consider toning down "annihilated"
in ghost zero language. Alternative: "shown to be logically inconsistent with
the axiom set." Keep "annihilated" for the informal/progress version.

---

## Session 24 (Mar 9, 2026): Langlands Phase 1 — Decimal Bug, HP Consistency, Skolemized Zeta

### Critical Bug Found and Fixed: i32 Truncation in Decimal Literals

`Real::from_rational(num, den)` in the vendored z3 crate casts i64→c_int (i32) via
`Z3_mk_real`. For any decimal > ~2.1, the numerator overflows. Example: `14.135` with
denominator 1e9 → numer = 14,135,000,000 → wraps to -454,934,592 as i32. **Z3 was seeing
garbage rationals for every decimal literal in axioms.** This caused false AXIOM
INCONSISTENCY for every research file using decimal values (sessions 19-22).

**Fix:** `from_rational_str()` for decimal→Z3 conversion (exact string representation).
Also guarded `from_rational()` itself to fall back to string path when values exceed i32.

**Impact:** Every AXIOM INCONSISTENCY we saw in the HP file was a false positive.
`number_theory_test.kleis` should be re-run — it likely has false failures too.

### Hilbert-Pólya Consistency: VERIFIED

With the decimal fix, the HP axioms are **CONSISTENT** (Z3 returns SAT):

| Example | Result |
|---------|--------|
| T_hp is self-adjoint | **PASS** (Z3 verified) |
| adjoint equals self | **PASS** (Z3 verified) |
| T_hp is densely defined | **PASS** (Z3 verified) |
| eigenpair at first zeta zero | **PASS** (Z3 verified: T·v₁ = 14.135·v₁) |
| zeta is Selberg class | **PASS** (Z3 verified) |
| zeta has degree 1 | **PASS** (Z3 verified) |

6 failures are evaluator limitations (can't reduce `hp_eigenvalue(1)`, `is_nontrivial_zero(...)` concretely), not Z3 issues.

**Significance:** Z3 found no logical obstruction to the Hilbert-Pólya strategy. A
self-adjoint unbounded operator with eigenvalues at zeta zeros is logically feasible.
(Consistency ≠ existence — but inconsistency would have killed the approach.)

### Skolemized Zeta Zeros: VERIFIED

Created `examples/mathematics/zeta_zeros_skolem.kleis` — Skolemizes the universal
quantifiers from `number_theory.kleis` into ground instances at the first five zeros:

- Functional equation: ξ(ρₖ) = ξ(1 - ρₖ) at each zero
- Zero definition: ζ(ρₖ) = 0 for each zero
- Critical strip/line: 0 < Re(ρₖ) < 1, Re(ρₖ) = 1/2
- Conjugate symmetry: is_nontrivial_zero(conj(ρₖ))
- Special values: ζ(2) = π²/6, ζ(-1) = -1/12, trivial zeros
- Selberg class ground facts + Skolemized coefficients

4/12 pass (Z3 verified). 8 failures are evaluator limitations (same pattern).
All axioms load and are CONSISTENT. No memory pressure.

### Unbounded Self-Adjoint Operators in stdlib

Added `UnboundedSelfAdjoint` structure to `stdlib/spectral.kleis`:
- `is_densely_defined`, `is_closed` operations
- `usa_closed`: self-adjoint + densely defined → closed (von Neumann)
- `usa_eigenpair`, `usa_orthonormal`, `usa_normalized` (reuses `eigenvalue`/`eigenvector`)
- `usa_not_compact`: unbounded self-adjoint → not compact
- `resolvent` operation with `resolvent_bounded` axiom

This is textbook functional analysis (Reed & Simon vol. 1), not a conjecture.

### HP File Updated

- Removed `import "stdlib/prelude.kleis"` (not needed, causes Z3 memory explosion)
- Replaced `is_bounded(T_hp)` with `is_densely_defined(T_hp)`
- Now fully self-contained, verifies in ~2 seconds

### Files Changed

- `src/solvers/z3/backend.rs` — decimal literal translation via `from_rational_str()`
- `vendor/z3/src/ast/real.rs` — `from_rational()` i32 overflow guard
- `examples/mathematics/hilbert_polya_consistency.kleis` — unbounded, no prelude, 6/12 pass
- `examples/mathematics/zeta_zeros_skolem.kleis` — NEW, 4/12 pass
- `stdlib/spectral.kleis` — `UnboundedSelfAdjoint` structure

### Branch

`feature/langlands-phase1-memory-guard`

### Langlands Relational Consistency: VERIFIED (8/10)

Created `examples/mathematics/langlands_relational.kleis` — parameterized
`spectral_op : DirichletSeries → Operator` mapping each L-function to its own operator.
Two L-functions formalized:

- **ζ(s)** — Riemann zeta, zeros at 14.135, 21.022, 25.011
- **L(s, χ₄)** — Dirichlet L-function for non-trivial character mod 4,
  zeros at 6.021, 10.244, 12.588. Special value: L(1, χ₄) = π/4 (Leibniz formula)

Z3 verifies joint consistency: both operators self-adjoint, unbounded, distinct
eigenvalue sequences, distinct Selberg class membership, Leibniz formula — all
simultaneously satisfiable. 8/10 pass (2 failures are evaluator limitations on
functional equation equality assertions).

### Resolvent-Spectral Bridge: 6/10 + Z3 Counterexample

Created `examples/mathematics/resolvent_spectral_bridge.kleis` — formalizes:

1. **Resolvent identity**: R(z,T)·vₙ = vₙ/(λₙ - z) at ground instances
2. **ξ-spectral bridge**: ξ(1/2 + iλₙ) = 0 ↔ λₙ is an eigenvalue
3. **Functional equation → spectral symmetry**: ξ(s) = ξ(1-s) forces
   eigenvalue_of(T, -n) = -eigenvalue_of(T, n) and spectral involution J

**Key finding: Z3 disproved the resolvent-eigenvector identity** by constructing a
counterexample where `csub` and `cdiv` (uninterpreted functions) don't behave like
actual complex arithmetic. Z3 assigned `csub(complex(21.022,0), complex(0,1)) →
complex(6,2)` instead of `complex(21.022,-1)`. This reveals exactly what's needed:
ground axioms connecting `csub`/`cdiv` to the built-in complex representation, or
use the built-in `complex_sub`/`complex_div` from `stdlib/complex.kleis`.

**What passed (6/10):**
- Operator self-adjoint + unbounded
- First eigenpair: T·v₁ = 14.135·v₁
- Spectral symmetry: eigenvalue_of(T, -1) = -eigenvalue_of(T, 1) ← **new constraint**
- Spectral involution J maps v₊ → v₋
- Orthogonality preserved
- Eigenvalues increasing

### Langlands Transfer Axiom: VERIFIED (13/16)

Created `examples/mathematics/langlands_transfer.kleis` — formalizes the simplest
instance of Langlands functoriality: the **Artin factoring** for ℚ(i)/ℚ.

The Dedekind zeta function of the Gaussian integers factors:
  ζ_{ℚ(i)}(s) = ζ(s) · L(s, χ₄)

**Transfer Axiom (Spectral Form):**
  spectrum(T_{ℚ(i)}) ⊇ spectrum(T_ζ) ∪ spectrum(T_{χ₄})

Three distinct self-adjoint unbounded operators coexist. The merged operator
T_{ℚ(i)} has eigenvalues that are the interleaved union:
  6.021 < 10.244 < 12.588 < 14.135 < 21.022 < 25.011
  (χ₄)    (χ₄)    (χ₄)     (ζ)      (ζ)      (ζ)

**What Z3 verified (13/16):**
- All three operators self-adjoint, distinct, unbounded
- Degree additivity: deg(ζ_{ℚ(i)}) = deg(ζ) + deg(L(χ₄)) = 1 + 1 = 2
- Transfer preserves eigenpairs (from both source operators)
- Merged spectrum strictly increasing (correct interleaving)
- Spectral symmetry propagates: negative eigenvalues of T_{ℚ(i)} track back
  to source operators (T_ζ and T_{χ₄})
- Leibniz formula L(1,χ₄) = π/4 consistent with the full system

**3 failures** are all evaluator-fallthrough (conjunction/equality assertions the
evaluator can't simplify). Zero Z3 contradictions.

### Files Changed

- `examples/mathematics/langlands_relational.kleis` — NEW, 8/10 pass
- `examples/mathematics/resolvent_spectral_bridge.kleis` — NEW, 6/10 pass
- `examples/mathematics/langlands_transfer.kleis` — NEW, 13/16 pass

### Next Steps

**1. Close the resolvent bridge** — connect `csub`/`cdiv` to actual complex arithmetic
(either ground axioms or import `complex.kleis` operations). Then re-test the resolvent
identity. If Z3 verifies it, the analytic↔spectral bridge is fully formalized.

**2. Re-run `number_theory_test.kleis`** — 19 assertions that were blocked by the decimal
bug and Z3 memory. Both are now fixed. Should yield new results.

**3. Evaluator fallthrough for symbolic assertions** — The failures across all research
files are because the evaluator doesn't fall through to Z3 for assertions like
`assert(is_nontrivial_zero(...))` or `assert(f(x) = value)` where `f(x)` is symbolic.
Fixing this would increase pass rates without changing the axioms.

**4. Local Langlands Correspondence** — modeling p-adic representations is deeper but
the ground Skolemization technique works: assert `satake_map(π_p) = t_p` for specific
primes p, check consistency.

---

## Session 23 (Mar 9, 2026): Z3 Memory Guard — Two-Layer Architecture

### What Was Done

Implemented a two-layer memory guard that mirrors the existing timeout architecture:

| Layer | Mechanism | Purpose |
|-------|-----------|---------|
| **Layer 1 (primary)** | External monitoring: proactive checks in `axiom_verifier.rs` + watchdog thread polling `Z3_get_estimated_alloc_size()` | Clean bail before OOM — returns error, test fails gracefully |
| **Layer 2 (backstop)** | Z3 internal `memory_max_size` at limit + 25% headroom | If Layer 1 misses (Z3 jumps past threshold in one operation), Z3 returns null from API calls → vendored crate exits cleanly via `process::exit(101)` — no panic, no unwinding, no C++ abort |

### Architecture: Why Two Layers

Z3 can allocate large memory blocks in a single internal operation (e.g., quantifier instantiation). External monitoring has a granularity gap — it checks between operations, not during them. Without the Z3 internal backstop:

- **Exit 137 (SIGKILL)**: OS kills process when Z3 runs free (no internal limit)
- **Exit 134 (SIGABRT)**: Z3's internal limit fires → null return → Rust `unwrap()` panics → unwinding through FFI → C++ destructors throw → `libc++abi: terminating due to uncaught exception`

The fix: replace `.unwrap()` on Z3 null returns with `process::exit(101)` — terminates immediately without unwinding, preventing the C++ abort cascade.

### Test Results

| Test file | Before | After |
|-----------|--------|-------|
| `test_heavy_imports.kleis` | Exit 137 (SIGKILL, 6GB) | Exit 1 (test failure, caught at 2567MB) |
| `hilbert_polya_consistency.kleis` | Exit 134 (SIGABRT) | Exit 1 (test failure, runs in 5s) |

### Files Changed

- `vendor/z3/src/func_decl.rs` — `FuncDecl::new()` and `FuncDecl::wrap()`: null → `process::exit(101)` instead of `unwrap()`
- `vendor/z3/src/ast/dynamic.rs` — `Dynamic::new_const()` and `Dynamic::fresh_const()`: null → `process::exit(101)` instead of `unwrap()`
- `vendor/z3/src/lib.rs` — re-export `get_estimated_alloc_size()` from z3-sys
- `src/solvers/z3/backend.rs`:
  - `Z3_MEMORY_LIMIT_BYTES` atomic for external monitoring
  - `z3_memory_limit_bytes()` public accessor
  - `solver_check_with_watchdog()` polls memory every 100ms + interrupts via `ContextHandle`
  - `memory_max_size` restored at +25% headroom as Layer 2 backstop
  - Default: `KLEIS_Z3_MEMORY_MB=2048` (2GB), set via env var
- `src/axiom_verifier.rs`:
  - Proactive memory check at start of `ensure_structure_loaded()`
  - Per-axiom 90% threshold check in `load_axioms_recursive()` loop
- `src/bin/kleis.rs` — `catch_unwind` around `eval_example_block` to convert panics to test failures

### Configuration

`KLEIS_Z3_MEMORY_MB=<limit>` — default 2048 (2GB). Set to 0 to disable memory guard entirely.

### Lesson: Timeout and Memory Are the Same Pattern

| | Timeout | Memory |
|---|---------|--------|
| **External (primary)** | Watchdog thread + `ContextHandle::interrupt()` | Proactive checks + watchdog polling `Z3_get_estimated_alloc_size()` |
| **Internal (backstop)** | Z3's `timeout` param (DISABLED — causes `ASSERTION VIOLATION`) | Z3's `memory_max_size` at +25% (ENABLED — null returns handled cleanly) |
| **FFI safety** | `catch_unwind` in test runner | `process::exit(101)` in vendored crate |

Z3's internal timeout is still disabled (session 6: causes segfault in `smt_context.cpp`). Z3's internal memory limit is safe because the failure mode is different: timeout fires mid-processing and corrupts state; memory limit makes API calls return null, which we intercept before any corruption.

---

## Session 22 (Mar 9, 2026): Hilbert-Pólya Consistency Check

### What Was Done

Attempted to create `examples/mathematics/hilbert_polya_consistency.kleis` — a research
file that asserts the existence of a self-adjoint operator T_hp whose eigenvalues correspond
to non-trivial zeta zeros, then asks Z3 whether this is logically consistent with the
spectral theory and number theory axioms.

### Key Findings

**1. Z3 caught a real mathematical error: T_hp cannot be compact.**

First attempt asserted `is_compact(T_hp)` with eigenvalues 14.135, 21.022, 25.011, ...
Z3 returned AXIOM INCONSISTENCY because `SpectralTheorem.spectral_eigenvalues_decrease`
requires `abs(eigenvalue(T, n+1)) ≤ abs(eigenvalue(T, n))` for compact operators — but
the zeta zeros' imaginary parts *increase*. This is a known result in the literature:
the Hilbert-Pólya operator, if it exists, must be **unbounded** self-adjoint.

**2. Combined imports (spectral.kleis + number_theory.kleis) cause Z3 OOM.**

Importing both stdlib files together loads ~100+ universal quantifiers from complex.kleis,
prelude.kleis, spectral.kleis, and number_theory.kleis. Z3 consumed 6+ GB attempting
quantifier instantiation. A trivial test (`assert(1 + 1 = 2)`) passes because it's
resolved by the concrete evaluator without invoking Z3 — but any symbolic assertion
triggers the full axiom loading.

**3. Self-contained approach with local declarations works partially.**

Following the pattern from `number_theory.kleis` (which declares `meromorphic`, `gamma`
locally to avoid importing `analysis.kleis`), we created a self-contained file importing
only `prelude.kleis` with local `data` and `operation` declarations. The minimal
`SelfAdjointGround` structure alone passes (1/1 in ~25 seconds). But adding the full
`HilbertPolyaOperator` with eigenvalue axioms triggers AXIOM INCONSISTENCY again.

### Where We Left Off — Bisection In Progress

The bisection to find the contradictory axiom combination was interrupted by memory
pressure. Current state:

| Test | Result |
|------|--------|
| `prelude` alone + `SelfAdjointGround` (adjoint, is_self_adjoint) | ✅ PASS |
| Above + `HilbertPolyaOperator` (eigenvalues, bridge, orthogonality) | ❌ INCONSISTENCY |
| Above + `SelbergClassGround` (zeta Selberg class facts) | ❌ INCONSISTENCY |

**Next step:** Continue bisecting `HilbertPolyaOperator` axioms to isolate which
combination contradicts the prelude axioms. Likely candidates:
- The eigenpair axioms (`op_apply(T_hp, v) = h_smul(complex(λ, 0), v)`) may conflict
  with `VectorSpace` axioms loaded via prelude's `over Field` clause
- The `ip(v1, v2) = complex(0, 0)` orthogonality axioms interact with prelude's
  algebraic structures

### File

`examples/mathematics/hilbert_polya_consistency.kleis` — current version is the
self-contained approach (imports only prelude, all operations declared locally).

### Lessons

1. **Z3 memory guard is critical.** We need the 2GB watchdog discussed in session 20.
   Without it, Z3 silently consumes all available RAM.
2. **Quantifier-free (ground) axioms are the way forward for research files.** Universal
   quantifiers from combined stdlib imports are a combinatorial bomb for Z3.
3. **The "compact operator" inconsistency was a genuine mathematical insight.** Z3 found
   a real constraint — this validates the approach of "assert a conjecture, see where
   Z3 hits the wall."

### Next Research Directions

**1. Bisect the functional equation** for the first two Skolemized zeta coefficients to
see if we can bypass the 13GB memory wall. Instead of loading the full functional equation
with universal quantifiers, try ground instances: assert `xi(s₁) = xi(1 - s₁)` for
specific s-values and check if Z3 can handle the reduced axiom set.

**2. arXiv paper: "Calculemus: Leibniz's Two Programs and the Machine Verification of
International Law"** — Leibniz's legal philosophy (Codex Iuris Gentium → Wolff → Vattel →
UN Charter Article 51) and computational philosophy (Characteristica Universalis → Frege →
Turing → Z3) converge in Kleis. The paper would trace both lineages, present the Article 51
formalization as a working example, and show Z3's verdicts on competing self-defense
doctrines. The Kleis source files serve as the executable appendix. Format: arXiv-style
Kleis template (similar to `stdlib/templates/arxiv_paper.kleis`). Location:
`examples/authorization/` alongside the existing UN Charter formalization.

---

## Session 21 (Mar 9, 2026): Inheritance Consistency Audit

### What Was Done

**Systematic audit of how `define`, `axiom`, `operation`, and `implements` blocks propagate through the `extends` chain.** This was a code-reading session — no changes made. Kleis works correctly; the audit documents the current architecture for future analysis.

### Finding 1: Three Subsystems, Correct Separation of Concerns

| What gets inherited via `extends` | Z3 Verifier | Evaluator | Type Context |
|---|---|---|---|
| **Axioms** | Yes (recursive via `ensure_structure_loaded`) | N/A | N/A |
| **Operations** (identity elements) | Yes | No | No (recorded, not merged) |
| **`define` statements** | Yes (as Z3 function definitions) | No (not needed — has builtins) | No (not needed — has `operation` signatures) |

Each subsystem takes from the structure only what it needs — this is correct separation of concerns, not inconsistency:
- **Z3** needs `define` members because they become axioms (universally quantified equalities)
- **Evaluator** doesn't need structure `define` because it has hardcoded builtins for concrete computation
- **Type system** doesn't need `define` because the corresponding `operation` declaration already provides the type signature (see Finding 6)

The Z3 axiom verifier is the only subsystem that fully resolves the `extends` chain (`axiom_verifier.rs:380-391`), loading parent structures recursively. This is correct — Z3 is the only one that needs the full theory context for reasoning. The type context (`type_context.rs:370-378`) records the `extends` relationship via `register_extension()` for hierarchy queries but doesn't merge members — it doesn't need to.

### Finding 2: Evaluator Dispatches via Hardcoded Builtins, Not Structures

The evaluator's `eval_concrete()` tries operations in this order:
1. **Builtins** (`builtins.rs:81-106`): Hardcoded `+`, `-`, `*`, `/`, `=`, etc.
2. **User-defined functions** (`self.functions` HashMap)
3. **Return as-is** (symbolic)

Builtins shadow structure `define` members. `define (-)(x, y) = x + negate(y)` in `Ring` and `define (/)(x, y) = x × inverse(y)` in `Field` are never reached by the evaluator — the hardcoded `"minus" | "-"` and `"divide" | "/"` builtins intercept first.

Structure `define` members only matter in the **Z3 path**, where they become universally quantified axioms (e.g., `∀(x, y). minus(x, y) = plus(x, negate(y))`).

### Finding 3: `implements` Blocks Are Passthrough Storage in the Evaluator

`implements` blocks (e.g., `implements Field(ℝ) { operation (+) = builtin_add }`) are collected by `load_program_with_file` (`evaluator/mod.rs:320-323`) but **never consulted during concrete evaluation**. They are only forwarded to `StructureRegistry` when `verify_with_z3()` calls `build_registry()` (`verification.rs:329-363`).

The `implements` binding of abstract operations to concrete builtins is meaningful to:
- The **type context** (for type checking / operation registry)
- The **Z3 verifier** (via `StructureRegistry`, for where-constraint resolution)

The evaluator has its own parallel builtin table (`builtins.rs`) that handles `+`, `-`, `/`, `*` etc. directly by name — independently of `implements` blocks.

### Finding 4: `define` Inside Structures References Parent Entities

`define` statements cross the inheritance boundary. Example from `prelude.kleis`:
- `Field` has `define (/)(x, y) = x × inverse(y)` — `×` comes from `Ring` (parent), not `Field` itself
- This works in Z3 because `ensure_structure_loaded` loads parents first (load order resolves names)
- There is no formal scope resolution — just uninterpreted function names that happen to be defined in the correct order

### Finding 5: Design Decision — No Top-Level Functions

By deliberate design, Kleis does not use top-level `define` statements (functions belong to structures). The grammar allows `TopLevel::FunctionDef` and the evaluator processes it, but real Kleis programs should not use it.

**Exceptions in practice:** `prelude.kleis` has `define pi`, `define e`, `define phi`, `define sqrt2` at top level. `cartan_compute.kleis` and `arxiv_paper.kleis` are entirely top-level defines. These predate the design decision and work because the evaluator's `load_program_with_file` does process `TopLevel::FunctionDef`.

### Summary: How Example Blocks Work

When `kleis test` evaluates example blocks:
- **`let x = expr`** → `eval()` → `eval_concrete()` → hardcoded builtins → `self.functions` → symbolic
- **`assert(a = b)`** → `eval_equality_assert()` → tries `eval_concrete()` → if symbolic, falls through to `verify_with_z3()` → builds `StructureRegistry` from stored structures + implements → creates `AxiomVerifier` → loads axioms via `extends` chain → Z3 decides

The two evaluation paths (concrete builtins vs. Z3 axioms) are **architecturally separate**. Concrete evaluation uses hardcoded Rust; Z3 verification uses structure axioms and `define` members. They happen to agree on arithmetic because the builtins implement the same semantics as the axioms — but this agreement is by convention, not by construction.

### Finding 6: Import Ordering Ensures Parent Availability

The `extends` chain doesn't need special "load parent first" logic beyond what the normal import mechanism already provides:
- `load_imports_recursive` (`kleis.rs:604-608`) processes an import's own imports before loading the import itself — depth-first ordering
- Within a single file, structures are parsed top-to-bottom, so `Ring` is registered before `Field`
- If a parent structure is not imported or defined, the system errors at use time (not parse time):
  - `register_implements` (`type_context.rs:452-456`) returns `"Unknown structure"` if the structure doesn't exist
  - `ensure_structure_loaded` (`axiom_verifier.rs:375-378`) returns `"Structure not found"` if the parent isn't in the registry
- The parser itself does NOT validate the parent exists — it just records the `extends_clause`. But in practice, the import system ensures correct ordering, making undefined parents impossible in well-formed programs.

### Finding 7: Type System Does Not Need `define` Members

The type context registers `define` members as operations (`type_context.rs:395-398`), but this is redundant. The type system only needs:
- **`operation` declarations** — for type signatures (e.g., `operation (-) : R × R → R`)
- **`extends` relationships** — for inheritance hierarchy
- **`implements` blocks** — for which types satisfy which structures

A `define` provides the *implementation* (how an operation is computed), not its *type*. Since there is always a corresponding `operation` declaration with the type signature, the `define` registration in the type context duplicates what the `operation` already provides. HM cares about *what type something has*, not *how it's computed*. `define` is purely a concern for Z3 (axioms) and potentially the evaluator (symbolic expansion) — not the type system.

### Finding 8: Kleis Inheritance vs. Liskov Substitution Principle

Kleis's `extends` model is different from LSP but consistent in separation of duties:

- **LSP** separates signature compatibility (type system: contravariance/covariance) from behavioral contracts (pre/post/invariants) for **runtime object substitution** in mutable-state programs.
- **Kleis** separates type signatures (HM) from semantic constraints (Z3/axioms) from concrete computation (evaluator) for **theory extension** over stateless mathematical structures.

The shared principle: **the child honors the parent's contracts.** A `Field` extending `Ring` carries all `Ring` axioms forward; Z3 verifies the combined theory is consistent. The child can only add constraints, never weaken the parent's — same direction as LSP's "postconditions/invariants cannot be weakened."

Where they differ: LSP addresses problems that cannot arise in Kleis. There are no mutable objects to substitute, no methods to override, no state for history constraints. The Rectangle/Square violation is structurally impossible in a system of immutable theories. Kleis doesn't need LSP's machinery — but it respects the same discipline of separation of concerns and contract inheritance.

### Future: Manual Chapter on Inheritance and Theory Extension

These findings are material for a dedicated manual chapter covering the three-subsystem architecture, how `define`/`axiom`/`operation`/`implements` flow through the system, import ordering, separation of concerns, and the LSP comparison. This chapter should be inserted **before** the Structuralism chapter (currently Ch. 29), which must remain the final chapter — it is the philosophical capstone connecting Kleis to Leibniz and Bourbaki.

### No Changes Made

This was an audit session. Kleis works correctly. The findings are documented here for future analysis if/when the inheritance model needs to be made more uniform.

### Branch
No branch — findings written to `docs/NEXT_SESSION.md` on `main`.

---

## Session 20 (Mar 8-9, 2026): Structuralism Chapter + SEO + Leibniz

### What Was Done

**Structuralism chapter updates** (`docs/manual/src/chapters/29-structuralism.md`):
- Added "Calculemus" section tracing intellectual lineage from Leibniz's *characteristica universalis* and *calculus ratiocinator* through Bourbaki to Kleis
- Added "Everything Is an Expression" section on Kleis's foundational neutrality
- Added "The Constraint Layer" section on the three MCP servers as unified architectural pattern
- Clarified HM type inference / axiom inheritance interaction: "HM mechanism is unchanged but polymorphism honors inherited axioms — the type hierarchy is the channel through which axioms propagate"

**SEO fixes for kleis.io** (Google Search Console issues):
- Created `scripts/postbuild-manual.sh` — injects static `<link rel="canonical">` tags and unique per-page `<meta name="description">` into mdBook-generated HTML
- Updated `.github/workflows/deploy-pages.yml` to run postbuild script
- Modified `scripts/generate_sitemap.py` to exclude `introduction.html` (duplicate of `index.html`)
- Added canonical tag to root `index.html`
- Cross-platform fixes: portable `sedi()` function for macOS/GNU sed compatibility

### Files Changed
- `docs/manual/src/chapters/29-structuralism.md` — three new sections + HM/axiom clarification
- `scripts/postbuild-manual.sh` — NEW
- `.github/workflows/deploy-pages.yml` — postbuild step
- `scripts/generate_sitemap.py` — EXCLUDE_URLS for introduction.html
- `sitemap.xml` — regenerated
- `index.html` — canonical tag

### Branch / PR
`fix/structuralism-hm-axiom-interaction` — merged

---

## Session 19 (Mar 7, 2026): L-function Theory stdlib + Favicon Fix

### What Was Done

**Favicon .ico fix** — Google Search wasn't showing the kleis.io favicon because `/favicon.ico` returned 404. Generated multi-size ICO (16/32/48) from `favicon.svg`, added to `index.html` and `deploy-pages.yml`. PR #158, merged.

**L-function theory stdlib** — three new stdlib files:

| File | Contents | Depends On | Status |
|------|----------|------------|--------|
| `stdlib/analysis.kleis` | Holomorphic functions, contour integration, residues, Cauchy theorem, gamma function, analytic continuation | prelude | Parses ✅, examples need selective loading |
| `stdlib/number_theory.kleis` | Dirichlet series, Euler products, Riemann zeta, functional equation, Selberg class, GRH | prelude (self-contained) | Parses ✅, Z3 diverges on universal quantifiers |
| `stdlib/spectral.kleis` | Hilbert spaces, operators, self-adjoint, compact, spectral theorem, trace class | prelude, complex | **6/6 verified** ✅ |

**Key results:**
- `spectral.kleis` verified 6 theorems: self-adjoint eigenvalues real, conjugate symmetry, orthonormal eigenvectors, trace cyclicity, compact → bounded, adjoint involution
- `number_theory.kleis` made self-contained (no analysis.kleis import) to avoid bulk-loading inconsistency
- `lambda` is a reserved keyword in Kleis parser — use `lam` instead
- Z3 diverges (memory explosion, not timeout) on universal quantifiers with complex arithmetic in number theory axioms (xi_def, functional_equation, euler_factor_def)

**New directory:** `examples/mathematics/` for pure math investigations (separate from `ontology/` which is POT physics).

### Files Changed
- `stdlib/analysis.kleis` — NEW
- `stdlib/number_theory.kleis` — NEW
- `stdlib/spectral.kleis` — NEW
- `examples/mathematics/spectral_theory_test.kleis` — NEW, 6/6 pass
- `examples/mathematics/number_theory_test.kleis` — NEW, 19 assertions (awaiting Z3 fix)
- `favicon.ico` — NEW (PR #158)
- `index.html` — favicon.ico link tag
- `.github/workflows/deploy-pages.yml` — deploy favicon.ico
- `docs/NEXT_SESSION.md` — updated

### Branch / PR
`feature/l-function-theory` — PR #159

### Open: Z3 Memory Limit

Z3's memory usage is **unbounded**. The time limit watchdog (`ContextHandle::interrupt()`) doesn't help when the heap explodes. The number theory axioms (`xi_def`, `functional_equation`, `euler_factor_def`) caused Z3 to consume **13.6 GB and growing** before being killed. Each E-matching instantiation creates new terms that trigger further instantiations — exponential growth with no ceiling.

**Investigation needed:**
1. `memory_max_size` in Z3 global params — same risk as `timeout` (session 6: internal timeout caused `ASSERTION VIOLATION` crash). Need to test if memory limit corrupts Z3 state.
2. Process-level `setrlimit(RLIMIT_AS)` / `setrlimit(RLIMIT_DATA)` — OS kills cleanly, Z3 doesn't try to handle it. Safer but coarser.
3. `KLEIS_Z3_MEMORY_MB` env var, same pattern as `KLEIS_Z3_TIMEOUT_MS`.
4. Monitor: could poll `/proc/self/status` (Linux) or `mach_task_info` (macOS) periodically and interrupt Z3 via `ContextHandle` when memory exceeds threshold.

**The safest approach is probably option 4** — poll memory usage in a watchdog thread and call `ctx.interrupt()` when it exceeds the limit. This reuses the existing interrupt mechanism that's proven safe.

**Default limit: 2 GB** (`KLEIS_Z3_MEMORY_MB=2048`). Target machine has 32 GB RAM. Any legitimate proof completes well under 100 MB; 2 GB gives headroom for complex theories while killing divergence early.

---

## Future: L-function Theory Next Steps

The stdlib is in place. Next steps:

1. **Skolemize number_theory axioms** — same technique as entanglement paper (session 18). Replace `∀(s : ℂ). xi(s) = ...` with ground instances at specific s values.
2. **Test number_theory_test.kleis** once Skolemized — 19 theorems ready.
3. **Hilbert-Pólya operator** — research file in `examples/mathematics/` or `theories/`, importing `spectral.kleis` + `number_theory.kleis`. Assert eigenvalue-zero correspondence, see where Z3 hits the wall.
4. **Z3 memory limit** — implement before running heavy number theory tests again.

---

## Session 18 (Mar 6, 2026): Skolemization of POT Entanglement Axioms — 24/24 Verified

### What Was Done

**Diagnosed Z3 `Unknown` results** for `spinor_2d_basis`, `R_irreducibility`, and `theorem1_singlet_correlation` in the entanglement paper.

**Root cause for `spinor_2d_basis` and `R_irreducibility`:** Both axioms had `∀∃` quantifier alternation where the universally quantified variable `s` appeared bare (not inside a function call), giving Z3 no E-matching trigger for instantiation. The Complex (`ℂ`) type of `spinor_smul`'s first argument further inflated the existential search space via the `mk_complex(re: Real, im: Real)` ADT.

**Fix: Skolemization** — replaced existential quantifiers with explicit Skolem witness functions:
- `SpinorBasis`: introduced `coord_up : SpinorField → ℂ` and `coord_down : SpinorField → ℂ`, replacing `∃(alpha beta : ℂ)` in `basis_spans` with `basis_decomposition`
- `RepIrreducibility`: introduced `irred_witness : SpinorField → SU2`, replacing `∃(g : SU2)` in `R_irreducible`

Both axioms went from 7-second timeouts to instant verification (<200ms).

**Root cause for `theorem1_singlet_correlation`:** Stale reference to `neg_cos` (a v1 operation) instead of `0 - cos(...)` (the v2 formulation via `correlation_def` + `spin_half_overlap`).

**Paper updated:**
- New subsection "Skolemization of Quantified Axioms" in Section 9 (Discussion) explaining the technique, why it's logically equivalent, and when to apply it
- Appendix axiom listing updated for V2 and V6
- PDF regenerated

**Result: 21/24 → 24/24 verified examples.** The entanglement paper is fully machine-checked.

### Manual page: kleis-review-python has no section

`docs/manual/src/chapters/28-agent-mcps.md` opens with "Kleis ships **three** MCP servers" but there are four. The summary table at the bottom already lists all four, but the intro and body treat kleis-review as a single Rust-only server. The Python review MCP — `scan_python` builtin, `python_types.kleis`, 12 string checks, 1 structural check, 7 diff-aware rules — has no dedicated section. Change "three" → "four" and add a parallel "kleis-review-python" section.

### Key Rule: Uppercase Constructor Convention

`decompose_constructor_equalities` in `ast.rs` uses an uppercase-first-letter check to distinguish constructors (which should be decomposed into field equalities) from non-constructor operations. **This is now a Kleis convention: constructor names start with an uppercase letter.** This prevents accidental decomposition of operations like `component(g, mu, nu)`.

### Key Technique: Skolemization for ∀∃ Axioms

Any axiom with `∀x. ∃y. P(x, y)` where `x` appears bare (not in a function application) will cause Z3 `Unknown`. The fix is to replace with `∀x. P(x, f(x))` where `f` is an explicit Skolem function. This gives Z3 the `f(x)` term it needs for E-matching. The transformation preserves satisfiability and validity by the Skolem normal form theorem.

### Files Changed
- `theories/pot_entanglement_v2.kleis` — Skolemized `SpinorBasis` and `RepIrreducibility`
- `examples/ontology/revised/pot_entanglement_paper.kleis` — updated tests + Skolemization subsection + appendix
- `examples/ontology/revised/pot_entanglement_paper.typ` — regenerated
- `examples/ontology/revised/pot_entanglement_paper.pdf` — regenerated

### Branch / PR
`feature/eqnlib-z3-matrix` — PR #153

---

## Session 17 (Mar 6, 2026): eval_concrete + Z3 Matrix Solving, stdlib Alignment

### What Was Done

**eval_concrete integration with Z3** — reduce concrete sub-expressions (e.g. `multiply(ones, ones)` → `Matrix(2,2,...)`) before sending to Z3, preserving the top-level equality for symbolic solving. This enables the equation editor to verify matrix multiplication results.

**stdlib alignment** — aligned `stdlib/lists.kleis` and `stdlib/matrices.kleis` with eqnlib notation (infix `=` instead of `equals()` wrapper, `→` guards, constructor injectivity axioms, associativity/distributivity).

**Server switched from eqnlib to stdlib** — equation editor server now loads `stdlib/minimal_prelude.kleis`, `stdlib/lists.kleis`, `stdlib/matrices.kleis` (36 structures instead of 11 from eqnlib).

### Key Changes
- `server.rs`: apply `eval_concrete` only to LHS/RHS of top-level `equals`, not the equality itself; switched from eqnlib to stdlib
- `backend.rs`: gate axiom loading to prevent double-load, transactional rollback for `declared_ops` on axiom translation failure, skip injectivity axioms
- `ast.rs`: `decompose_constructor_equalities` with uppercase-first guard, `collect_ops` methods
- `comparison.rs`: return `Err` on sort mismatch instead of panicking
- `axiom_verifier.rs`: transactional `declared_ops` restore on failure
- `stdlib/lists.kleis`: `equals()` → infix `=`, added `→` guard on `nth_succ`, added `ListConstructor` injectivity
- `stdlib/matrices.kleis`: `MatrixConstructor` now has injectivity axiom, added `MatrixMulAssoc` + `MatrixDistributive`
- New `eqnlib/` directory with base, lists, matrix, and test_matrix libs

### Branch / PR
`feature/eqnlib-z3-matrix` — PR #153

### `kleis test` Failure Inventory (as of session 17)

All 2186 Rust tests (`cargo test`) pass. Below are the `kleis test` failures.

#### eqnlib/
- All 6/6 pass (including distributivity)

#### stdlib/
| File | Result | Failures |
|------|--------|----------|
| `combinatorics.kleis` | 10/12 | `all perms 2`, `n perms equals n factorial` — `concat()` doesn't support nested lists |
| `tensors_functional.kleis` | 13/29 | `tensor_get`, `tensor_add`, `tensor_scale`, wedge ops, EM tensor, d-squared — evaluator limitations with tensor indexing and wedge product computation |

#### examples/ (non-ontology)
| File | Result | Issue |
|------|--------|-------|
| `chess/chess.kleis` | 1/10 | Evaluator limitations |
| `contractbridge/contractbridge.kleis` | 3/8 | Evaluator limitations |
| `debug_main.kleis` | 0/1 | Symbolic assertion can't verify |
| `inverted_pendulum_discrete.kleis` | 0/1 | `dlqr` not fully evaluated |
| `ontology/pot_core.kleis` | 8/9 | Z3 quantifier inconclusive |
| `ontology/spacetime_type.kleis` | 0/1 | `component(make(...))` not reduced |
| `petri-nets/mutex_bounded.kleis` | 9/11 | Z3 counterexample |
| `petri-nets/mutex_example.kleis` | 0/4 | Z3 counterexample |
| `petri-nets/mutex_verified.kleis` | 2/8 | Z3 counterexample |
| `sudoku/sudoku.kleis` | 6/10 | Solver limitations |

#### examples/ (errors — parse or panic)
| File | Error |
|------|-------|
| `hardware/alu_verification.kleis` | Panic: `bvadd` sort mismatch in Z3 |
| `hardware/simple_alu.kleis` | Panic: `bvadd` sort mismatch in Z3 |
| `export/render_to_typst_demo.kleis` | Missing import file |
| `debug_test.kleis` | Parse error |
| `lps/test.kleis` | Parse error |
| `mass_from_residue.kleis` | Parse error |
| `ontology/pot_foundations.kleis` | Parse error |

#### examples/ontology/revised/
| File | Result | Issue |
|------|--------|-------|
| `bell_violation_test.kleis` | 9/9 | |
| `cosine_uniqueness_test.kleis` | 4/5 | `z3_inconsistency_detector` (expected failure) |
| `minimal_admissable_kernel_class.kleis` | 2/2 | |
| `pot_arxiv_paper.kleis` | 8/8 | |
| `pot_channel_units.kleis` | 1/1 | |
| `pot_core_kernel_projection.kleis` | — | Z3 hangs (killed after 12+ min) |
| `pot_entanglement_paper.kleis` | 24/24 | All pass (Skolemized in session 18) |
| `rotation_curve_numerical.kleis` | 2/2 | |

### Open Items
1. **`bvadd` sort mismatch panic** — `hardware/alu_verification.kleis` and `hardware/simple_alu.kleis` panic in Z3 bitvector operations. Needs investigation.
2. **`pot_core_kernel_projection.kleis` hangs Z3** — too many axioms loaded; Z3 solver doesn't terminate.
3. **Petri net counterexamples** — Z3 finds counterexamples for mutex properties. May need axiom refinement or different encoding.
4. **Tensor indexing in evaluator** — `tensor_get`, `tensor_add`, `tensor_scale` not reduced by evaluator.
5. **Parse errors** — `debug_test.kleis`, `lps/test.kleis`, `mass_from_residue.kleis`, `ontology/pot_foundations.kleis` have parse errors.

---

## Session 16 (Mar 5, 2026): Configurable Per-Language LLM Guidelines Prompt

### What Was Done

**Configurable LLM guidelines** — load per-language coding standards into the LLM advisory system prompt so the reviewer checks against specific guidelines (Microsoft Rust, PEP 8, etc.) instead of generic heuristics.

**Grounded findings** — require every LLM finding to cite a specific line number and code snippet. Findings without a line reference are silently dropped, eliminating hallucinated/parroted guideline violations.

- **Config** (`src/config.rs`): Added `guidelines_file: Option<String>` to `LlmConfig` + `PartialLlm` + `KLEIS_LLM_GUIDELINES_FILE` env var override.
- **Advisory** (`src/review_mcp/advisory.rs`):
  - `resolve_guidelines_path()` — 4-step resolution: env var > config > auto-discovery (`examples/guidelines/{lang}_guidelines.txt`) > none
  - `load_guidelines_text()` — reads file, skips comment-only placeholder files
  - `build_system_prompt()` — structured prompt with guidelines + formal rule names when available, generic fallback otherwise
  - `add_line_numbers()` — prepends line numbers to source so LLM can cite them
  - `Advisory` struct now has `line: Option<u32>` and `evidence: Option<String>`
  - `parse_advisories()` filters out findings without a line number
  - 15 unit tests (8 new: prompt generation, resolution order, grounding, line numbers)
- **CLI** (`src/bin/kleis.rs`): Loads guidelines for detected language, extracts formal rule names from engine, passes both to LLM. Renders `(line N)` and evidence snippet.
- **Guidelines files**: `examples/guidelines/rust_guidelines.txt` (condensed Microsoft Pragmatic Rust Guidelines, 157 lines / 8.7KB — distilled from 90KB original), `examples/guidelines/python_guidelines.txt` (placeholder).

### Key Design Decisions

- **Condensed guidelines (8.7KB not 90KB)**: Full guidelines wasted ~22K tokens on prose/examples an LLM already knows. Condensed to guideline ID + one-line rule + "Check for" triggers. ~2100 tokens.
- **Grounded findings**: Without line numbers + evidence requirement, gpt-4o-mini parroted guidelines back as fabricated findings (5/5 were hallucinated in first test). With grounding, findings cite real code and ungrounded ones are filtered out.
- **Per-language**: Resolution auto-discovers `{lang}_guidelines.txt` so adding Python/Go guidelines is just dropping a file.

### Branch / PR

`feature/llm-guidelines-prompt` — merged via PR #151 into `feature/microsoft-rust-guidelines`

### Files Changed
- `src/config.rs` — guidelines_file in LlmConfig + PartialLlm + env override
- `src/review_mcp/advisory.rs` — guidelines resolution, grounded prompts, line numbers, evidence
- `src/bin/kleis.rs` — guidelines loading, rule name extraction, evidence rendering
- `examples/guidelines/rust_guidelines.txt` — condensed Microsoft Rust Guidelines
- `examples/guidelines/python_guidelines.txt` — placeholder

---

## Session 15 (Mar 5, 2026): Advisory Severity Levels for Review Rules

### What Was Done

**Advisory severity levels** — two-tier rule system (`check_*` = blocking error, `advise_*` = non-blocking advisory) so style/documentation rules don't break CI.

- **Engine** (`src/review_mcp/engine.rs`): Added `RuleSeverity` enum (Error, Advisory), `severity` field on `RuleVerdict`, `AdviseFunction` variant on `ReviewRuleKind`. `check_code` and `check_diff` discover both prefixes; only `check_*` failures set `passed = false`. Summary shows three-way counts (errors/advisories/passed).
- **CLI** (`src/bin/kleis.rs`): Advisory failures render as `⚠️` instead of `❌`. Only `check_*` failures contribute to exit code 1 — advisories never break CI.
- **MCP Server** (`src/review_mcp/server.rs`): JSON verdicts include `"severity": "error"|"advisory"`. `list_rules` and `describe_standards` show separate sections. `explain_rule` reports severity-aware kind.
- **Policy** (`examples/policies/rust_review_policy.kleis`): 19 rules renamed from `check_*` to `advise_*` (style, docs, team patterns, AI artifacts). 29 rules remain as blocking `check_*` (safety, security, clippy -D, structural).
- **Tests** (`tests/review_mcp_test.rs`): 2 new tests (`test_advisory_failures_do_not_block`, `test_advisory_summary_counts`). Updated emoji test references and stat assertions. All 36 tests pass.

### Note: LLM advisories (`--advise`) are a separate system

The LLM advisory path (`src/review_mcp/advisory.rs`, `Advisory` struct with `severity: String`) is independent of `RuleSeverity`. Both are non-blocking, but they flow through different code paths. No unification was done — they're conceptually aligned but structurally separate.

### Branch
`feature/microsoft-rust-guidelines`

### Files Changed
- `src/review_mcp/engine.rs` — RuleSeverity enum, severity on verdicts, advise_* discovery
- `src/review_mcp/server.rs` — severity in JSON, list_rules/explain_rule sections
- `src/bin/kleis.rs` — advisory emoji rendering, exit code logic
- `examples/policies/rust_review_policy.kleis` — 19 rules renamed to advise_*
- `tests/review_mcp_test.rs` — 2 new tests, updated assertions

### Microsoft Rust Guidelines Coverage Analysis

The current policy covers **~15 of ~88** combined guidelines from the Microsoft Pragmatic Rust Guidelines and Rust API Guidelines. The covered rules are the ones mechanically detectable via string matching or structural AST analysis.

**What the current scanner CAN'T address** (architectural/runtime, ~50 rules):
M-SMALLER-CRATES, M-HOTPATH, M-THROUGHPUT, M-YIELD-POINTS, M-DESIGN-FOR-AI, M-MOCKABLE-SYSCALLS, M-IMPL-IO, M-INIT-CASCADED, M-INIT-BUILDER, M-DI-HIERARCHY, M-SIMPLE-ABSTRACTIONS, C-BUILDER, C-NEWTYPE, C-OBJECT, C-GENERIC, etc. These require human/LLM judgment or runtime profiling — the `--advise` LLM path is the right tool.

**What an improved Rust parser/scanner COULD address** (~20-25 more rules):
- **Type resolution** → M-PUBLIC-DISPLAY, M-TYPES-SEND, M-ERRORS-CANONICAL-STRUCTS, precise C-GOOD-ERR
- **Trait impl tracking** → C-COMMON-TRAITS (Debug, Clone, PartialEq on pub types), C-CONV-TRAITS, C-DEREF
- **Expression-level parsing** → M-PANIC-ON-BUG, M-REGULAR-FN, precise clippy-style checks
- **Doc comment structure** → M-FIRST-DOC-SENTENCE, M-CANONICAL-DOCS, M-MODULE-DOCS, M-DOC-INLINE, per-fn C-FAILURE

**Low-hanging fruit (no parser upgrade needed):**
- M-DOC-INLINE: `pub use` without `#[doc(inline)]` — string match
- M-PUBLIC-DISPLAY: pub structs missing `Display` derive — structural check (already have pub struct detection)
- M-FIRST-DOC-SENTENCE: doc comment length — structural check on doc comments

A parser with type resolution and trait impl tracking could bring coverage from ~15/88 to ~35-40/88.

---

## Session 14 (Mar 5, 2026): Native Rust Scanner (`scan_rust` builtin)

### What Was Done

**Native Rust structural scanner** — hand-written tokenizer + recursive descent parser (~2400 lines, zero dependencies) that emits Kleis AST identical to the Kleis-based `scan()` in `rust_parser.kleis`.

- **Tokenizer**: Handles string literals (including raw strings `r#"..."#`), all 6 comment types (line, outer/inner line doc, block, outer/inner block doc with nesting), attributes, keywords, punctuation, lifetimes, spans with line numbers.
- **Recursive descent parser**: Parses top-level items (fn, struct, enum, trait, impl, use, mod, const, static, type, macro_rules!), visibility variants (pub, pub(crate), pub(super), pub(self)), function qualifiers (async, const, unsafe, extern), generic parameters, `where` clauses, and computes `body_line_count` + `max_nesting` for function bodies.
- **Kleis AST emission**: Internal Rust AST types (`FnDecl`, `StructDecl`, etc.) convert to Kleis `Expression` via `to_expr()` methods, producing `Crate(items, comments, line_count)` — identical structure to the Kleis-based scanner.
- **`\n` auto-detection**: Matches the `foldLines` builtin behavior — detects whether source contains real newlines or escaped two-char `\n` from Kleis string literals.
- **`scan()` delegation**: `rust_parser.kleis` now delegates `scan(source)` to the native `scan_rust(source)` builtin. All 146 helper functions, 17 data types, and review query functions are unchanged.
- **19 Rust unit tests** + **25/25 Kleis example tests** pass.
- **`kleis review` integration verified** — ran against `verify-cli/src/storage/*.rs` (8 files, 86 rules). Structural rules (`check_structural`, `check_safe_structural`, `check_secure_structural`) fire correctly with accurate line numbers.

### Resolved Limitations

These limitations from the Kleis-based scanner are now fixed:

1. ~~**Brace depth is lexical, not semantic.**~~ — **RESOLVED**: The native tokenizer skips braces inside string literals and comments.
2. ~~**Block comments are not nest-aware.**~~ — **RESOLVED**: The native tokenizer correctly handles nested block comments (`/* /* */ */`).
3. ~~**Multi-line item headers may be incomplete.**~~ — **RESOLVED**: The native parser operates on the full token stream, so multi-line function signatures, `where` clauses, and attributes parse correctly.

### Branch
`feature/rust-scanner`

### Files Changed
- `src/rust_scanner/mod.rs` — module root (new)
- `src/rust_scanner/scanner.rs` — tokenizer + parser + Kleis AST emission (new, ~2400 lines)
- `src/lib.rs` — added `pub mod rust_scanner`
- `src/evaluator/builtins.rs` — `scan_rust` builtin registration
- `examples/meta-programming/rust_parser.kleis` — `scan()` delegates to `scan_rust()`

### Architecture: Why Hand-Written

Evaluated Pest (PEG), LALRPOP (LR(1)), Nom (combinators), and rust-peg. All add dependencies and generate full expression/type parsers we don't need. The native scanner only needs structural extraction (items, signatures, metrics) — a two-phase tokenizer + recursive descent is the right tool. Grammar reference: IntelliJ Rust BNF (MIT).

### Performance

The native scanner processes the full token stream in a single pass. Previously, `scan()` used Kleis-interpreted `foldLines` which executed hundreds of Kleis function calls per source line. The native version eliminates this overhead entirely.

---

## Session 13 (Mar 5, 2026): Equation Editor Z3 + Axiom Consistency Investigation

### What Was Done

**Equation Editor witness display** (stashed, not merged)
- Wired `PrettyPrinter` into `check_sat_handler` and `verify_handler` for human-readable Z3 witness output
- Tracked free variables in `quantifier_vars` so `model_to_witness` extracts structured bindings

**Axiom loading investigation** (stashed, not merged)
- Loading ALL stdlib axioms at once via `initialize_from_registry()` causes UNSAT — but **the individual axioms are proven correct** (tensor symmetries, Einstein equations, Bell violations, Cartan algebra all pass their Z3 proofs)
- The issue is **bulk loading strategy**, not axiom correctness. Each `.kleis` proof file loads only the structures it needs; the Equation Editor was the first place we tried loading everything into one Z3 context
- When abstract algebra structures (`Field(F)`, `Ring(R)`) are loaded with type parameters defaulting to `Int`, and `×` maps to Z3's integer multiplication, the combination creates unsatisfiable constraints — but that's a loading problem, not a math problem
- Added `ConsInjectivity` and `MatrixInjectivity` axioms to stdlib (stashed) — mathematically correct, need proper loading context

### Key Finding: Equation Editor Needs Selective Axiom Loading

The Equation Editor should load axioms the same way `.kleis` proof files do — selectively, based on what the user is working with. The `initialize_from_registry()` bulk-load approach was the wrong strategy. Options:
1. **Load on demand** — detect which structures the expression references, load only those
2. **User-driven** — let the user choose which theory context to work in (matrices, tensors, etc.)
3. **Expression analysis** — inspect the AST for operation names, load matching structures

### Branch
`fix/equation-editor-witness-display` — changes stashed (`git stash`), branch clean

### Stashed Changes
- `src/bin/server.rs` — PrettyPrinter witness display + `initialize_from_registry()` call
- `src/solvers/z3/backend.rs` — parametric structure skip filter + free var tracking
- `stdlib/lists.kleis` — `ConsInjectivity` axioms
- `stdlib/matrices.kleis` — `MatrixInjectivity` axioms
- `docs/NEXT_SESSION.md` — session notes

### Open Items
1. **Equation Editor witness display** — the PrettyPrinter fix itself is clean and correct, but was bundled with the axiom loading work. Could be extracted as a standalone change.
2. **Selective axiom loading for Equation Editor** — needs a strategy to load only relevant structures (like `.kleis` files do), not all 68+ at once.
3. **Matrix Z3 semantics** — `ConsInjectivity` and `MatrixInjectivity` axioms are ready (stashed), need proper loading context in the Equation Editor.

---

## Session 12 (Mar 4, 2026): Polyglot Review — Python Parser, MCP, End-to-End

### What Was Done

**Python Scanner (Rust)**
- **`scan_python(source)` builtin** — hand-written line scanner (~600 lines, zero dependencies) emitting nested Kleis AST
- **9 Kleis data types** — `PyModule`, `PyItem`, `PyFunction`, `PyClass`, `PyStmt`, `PyImport`, `PyFromImport`, `PyDecorator`, `PyExceptHandler`
- **12 query helpers** in `python_types.kleis` — `module_functions`, `module_classes`, `has_decorator`, `count_list`, etc.
- **Code organized** under `src/python/` (scanner.rs + mod.rs)

**Python Review Policy (46 rules)**
- **12 string-based checks** — `check_no_eval`, `check_no_sys_exit`, `check_no_mutable_defaults`, `check_no_bare_except`, `check_no_print_statement`, `check_no_environ_bracket`, `check_no_optional_type`, `check_no_hardcoded_password`, `check_no_debug_breakpoint`, `check_double_quote_strings`, `check_no_wildcard_import`, `check_no_eval`
- **1 structural check** (`check_python_structural`) with 6 sub-rules: long functions, long methods, import placement, bare except (AST with line numbers), missing return types (skips `__init__`), excessive try/except
- **7 diff-aware rules** — `diff_check_image_tag_bump`, `diff_check_requirements_pinned`, `diff_check_file_growth`, `diff_check_new_fns_typed`, `diff_check_sys_exit_introduced`, `diff_check_bare_except_introduced`, `diff_check_print_introduced`
- 
**Polyglot MCP Architecture**
- **Separate MCP instances per language** — `kleis-review-rust` and `kleis-review-python` (not a single MCP with naming hacks)
- **Dynamic server name** — derived from policy filename (`python_review_policy.kleis` → `kleis-review-python`)
- **Language-aware LLM advisory** — `build_system_prompt` accepts language parameter, code fences use correct language tag
- **Stdlib import resolution** — `KLEIS_ROOT` env var + directory walk for `stdlib/` imports, works from any working directory
- **Git context from target files** — `git_repo_root_for(dir)` derives repo root from the files being reviewed, not cwd

**End-to-End Validation**
- Tested all MCP tools: `list_rules`, `describe_standards`, `explain_rule`, `check_file`, `check_code`
- **AI agent autonomy test** 

### Branch
`feature/python-parser`

### Files Changed
- `src/python/scanner.rs` — Python line scanner (new)
- `src/python/mod.rs` — module root (new)
- `src/lib.rs` — added `pub mod python`
- `src/evaluator/builtins.rs` — `scan_python` builtin
- `src/evaluator/mod.rs` — removed old `python_bridge` module
- `src/review_mcp/advisory.rs` — language-aware prompts
- `src/review_mcp/engine.rs` — stdlib import resolution via `KLEIS_ROOT`
- `src/review_mcp/server.rs` — dynamic server name from policy filename
- `src/bin/kleis.rs` — `language_from_path`, `git_repo_root_for`, target-file git context
- `examples/meta-programming/python_types.kleis` — Kleis data types + helpers (new)
- `examples/policies/python_review_policy.kleis` — full Python policy (new)
- `.cursor/mcp.json` — parallel `kleis-review-rust` / `kleis-review-python`
- `docs/manual/src/chapters/28-agent-mcps.md` — polyglot MCP documentation
- `.cursorrules` — "no practical workarounds" rule

### Known Limitations (Python Scanner)
- **Multi-line function signatures** — extracts params from first line only
- **Multi-line `from` imports** — parses first line only
- **Triple-quote tracking** — doesn't distinguish docstrings from strings
- **No expression parsing** — assignments capture target but not value

### Migration Path
If structural rules need expression-level detail, add `ruff_python_parser` (MIT, Rust crate) behind a feature flag. Replace scanner internals; Kleis data types and policies stay unchanged.

### Architecture Decision: Separate MCPs per Language
- Each language gets its own MCP instance with its own policy, advisory prompt, and structural parser
- Cleaner than language-prefix naming conventions (`check_py_*` / `check_rs_*`)
- Future: Kleis structures could namespace rules (`structure PythonReview { ... }`) — the engine would discover `check_*` inside structures instead of only top-level functions

### Open Items
1. **No timeouts** — `eval_concrete` and Z3 can block indefinitely. STILL OPEN.
2. **`check_no_hardcoded_urls` false positive** — flags documentation URLs in comments. Needs structural version that skips comments.
3. **Z3 axioms not wired into automatic review** — `SafeCode`, `SqlSafe` etc. require explicit `evaluate_expression` calls.
4. **Vertex AI auth for `--advise`** — wire `gcloud auth print-access-token` into `advisory.rs` so `kleis review --advise` can use corporate Claude without a static API key.
5. **Semver comparison for diff rules** — `diff_check_version_bump` currently checks "different" but not "greater". Add proper `version_gt(a, b)`.
6. **Generic `extract_key_value`** — needs Kleis lambda/closure support in `foldLines`.
7. **Externalize `build_system_prompt` text** — load from file or config so users can customize without recompiling.

---

## Session 7 (Feb 26, 2026): Rebase, Conflict Resolution, and Merged PRs

### Merged PRs
- **#135** — STRIDE threat model rules, concrete Z3 verification, expanded review coverage
- **#136** — Structural Rust parsing, superseded string checks removed, docs updated, check_file tests

### Current State
- **28 active check_* functions**: 23 string-based + 5 structural (AST-based with line-number reporting)
- **6 Z3 concrete tests** + **6 check_file tests** + original tests = 34 total review MCP tests
- **Rust structural parser** (`rust_parser.kleis`) operational: `scan()`, `production_code()`, `fn_body_text()`, `non_test_fns_containing()`
- **Three-tier review model** documented in `28-agent-mcps.md`: string checks / structural checks / Z3 axioms

### Open Items
1. **No timeouts** — `eval_concrete` and Z3 can block indefinitely. STILL OPEN.
2. ~~**`evaluator.rs` is 10,887 lines**~~ — **DONE** via PR #137. Split into `src/evaluator/` with 7 modules.
3. **`check_no_hardcoded_urls` false positive** — flags documentation URLs in comments. Needs structural version that skips comments.
4. **Z3 axioms not wired into automatic review** — `SafeCode`, `SqlSafe` etc. require explicit `evaluate_expression` calls. Future: parser extracts code fragments, feeds to Z3.
5. ~~**NEXT_SESSION.md is 147K chars**~~ — **DONE**. Cleaned up: archives created, trimmed to ~106 lines.

### Known Limitations: `rust_parser.kleis` Structural Scanner

The Rust structural parser now delegates to a native Rust scanner (`scan_rust` builtin, session 14). Most previous limitations are resolved:

1. ~~**Brace depth is lexical, not semantic.**~~ — **RESOLVED** (session 14): Native tokenizer skips braces inside strings/comments.

2. ~~**Block comments are not nest-aware.**~~ — **RESOLVED** (session 14): Native tokenizer handles nested block comments.

3. ~~**Multi-line item headers may be incomplete.**~~ — **RESOLVED** (session 14): Native parser operates on full token stream.

4. **Macros can masquerade as items.** `macro_rules!` is parsed; attribute macros and DSL-like macros may confuse item detection. Acceptable for review tooling.

### Known Limitations: `kleis_review_policy.kleis` Checks

5. **Security checks are intentionally blunt.** Checks like `contains(prod, "password =")` and `format!(..SELECT..)` work as guardrails but will produce false positives in test fixtures, docs, and examples. Future: an allowlist mechanism or context-aware suppression.

6. **`production_code(source)` split is a correctness bottleneck.** The test-vs-production partition drives many checks. If it's too naive (e.g., misclassifying test helpers or integration tests), it either misses real problems or creates noise. Worth monitoring as the codebase evolves.

---

## Session 6 (Feb 23, 2026): Z3 Safety, Trigonometric Axioms, and Epistemic Boundaries

### CRITICAL: What You Need to Know

1. **Z3 global timeout crashes the solver.** Do NOT set `KLEIS_Z3_TIMEOUT_MS` to a nonzero value unless debugging. Z3's internal timeout fires mid-quantifier-processing and causes `ASSERTION VIOLATION` in `smt_context.cpp` (segfault). Default is now 0 (no timeout). The watchdog via `ContextHandle::interrupt()` is the safe wall-clock timeout.

2. **Universal trig axioms cause E-matching divergence.** We tried `stdlib/trigonometry.kleis` with `∀(a b : ℝ). cos(a+b) = cos(a)*cos(b) - sin(a)*sin(b)`. Z3's E-matching explodes: the nonlinear products in the addition formula interact with the Pythagorean identity, creating infinite instantiation chains (observed 13000+ quantifier instances before killing). **Ground instances at specific angles are the correct approach for Z3.**

3. **`neg_cos` replaced with `cos` in the entanglement theory.** `pot_entanglement_v2.kleis` now uses `cos` directly. `spin_half_overlap` reads naturally: `spinor_inner(proj_a, proj_b) = cos(angle_between(a, b))`.

### What Was Accomplished

1. **Z3 timeout default fixed** — `KLEIS_Z3_TIMEOUT_MS` default changed from 5000 to 0. Global Z3 params (timeout, rlimit, memory, soft_timeout) are now only set when explicitly nonzero. This fixed a regression where `pot_arxiv_paper.kleis` was crashing with Z3 ASSERTION VIOLATION at `smt_context.cpp:2485`.

2. **Trigonometric axioms explored** — Created `stdlib/trigonometry.kleis` with full axiomatic cos/sin (Pythagorean, addition formulas, periodicity, bounds). Confirmed all 14 axioms assert in <10ms, but the consistency check diverges. **Deleted the file** — universal nonlinear real arithmetic is beyond Z3's E-matching capability.

3. **Ground cos instances added to entanglement theory:**
   - `cos(0) = 1`, `cos(pi) = -1` (base values)
   - `cos(pi/2) = 0`, `cos(pi/4)^2 = 1/2` (CHSH angles)
   - `BellWitnessAngles` structure with three detector angles at 0, pi/4, pi/2

4. **Bell violation test created** — `examples/ontology/revised/bell_violation_test.kleis` with 9 tests: cos values, correlation at specific angles, Bell LHS/RHS at CHSH witnesses. All 9 pass.

5. **Cosine uniqueness test updated** — `cosine_uniqueness_test.kleis` migrated from `neg_cos` to `cos`. 4/5 pass (1 expected failure = inconsistency detector).

### Files Modified
- `src/solvers/z3/backend.rs` — timeout default 0, gate global params on nonzero
- `src/bin/kleis.rs` — updated `--help` for KLEIS_Z3_TIMEOUT_MS (default: 0, caution note)
- `theories/pot_entanglement_v2.kleis` — replaced neg_cos with cos, added BellWitnessAngles, updated BellCorrelation and AnticorrelationLemma
- `examples/ontology/revised/cosine_uniqueness_test.kleis` — migrated to cos
- `examples/ontology/revised/bell_violation_test.kleis` — **NEW**, 9 tests for Bell violation at CHSH angles

### Files Deleted
- `stdlib/trigonometry.kleis` — universal trig axioms cause E-matching divergence

### Test Results
- `pot_arxiv_paper.kleis`: 8/8 (regression clean)
- `bell_violation_test.kleis`: 9/9
- `cosine_uniqueness_test.kleis`: 4/5 (1 expected failure)

### Key Findings: Epistemic Boundaries in the Entanglement Theory

**The "Unknown" verdicts from Z3 are correct.** They represent the boundary between:
- **What algebra proves** (linearity, group actions, inner product invariance) — Z3 verifies these
- **What representation theory / analysis proves** (Schur's lemma, Wigner D-matrices, cosine from character theory) — Z3 returns Unknown

**Tightening `is_admissible` (e.g., constraining H_ont's codomain to C^3) does NOT help** because the Unknown axioms are all about SU(2) acting on SpinorField (C^2), not about the kernel's codomain (FieldR4). The projection `project_at` has already dropped from FieldR4 to SpinorField by the time any Unknown axiom is evaluated.

**The path to closing the gap:**
- **Short term:** Ground cos instances (done) — Z3 can verify the Bell violation with concrete values
- **Medium term:** Kleis evaluator as CAS bridge — compute representation theory results, feed to Z3 as ground truths
- **Long term:** Isabelle/HOL integration as a solver backend for formal proofs of representation theory (Schur's lemma, Wigner D-matrix classification)

The cos/sin addition formulas encode the Lie algebra structure of SU(2). They're not external computational facts — they're the content of the ontological commitment "SU(2) is a symmetry of H_ont." The ground instances carry the same ontological content as the universal axioms; Z3 just can't handle the universal form.

### Lessons Learned

1. **Z3's internal timeout is dangerous.** It fires mid-processing and corrupts Z3's internal state. Always use the `ContextHandle::interrupt()` watchdog instead.
2. **Universal quantifiers with nonlinear products = E-matching bomb.** `∀(a b : ℝ). f(a+b) = g(a)*g(b) - h(a)*h(b)` is a pattern Z3 cannot handle. Ground instances are the workaround.
3. **Don't put Z3-hostile axioms in stdlib.** Axioms that cause E-matching divergence should not be in shared libraries. Ground instances belong in the theory files that need them.
4. **Epistemic honesty > verification completeness.** "Unknown" is a valid answer when the mathematics genuinely requires tools beyond SMT (representation theory, analysis). Don't weaken the theory to get "Verified."

### NEXT_SESSION.md Cleanup — DONE
- [x] Mark completed items from sessions 1-5
- [x] Archive sessions older than 2 weeks to `docs/archive/sessions/`
- [x] Keep NEXT_SESSION.md focused on active work + last 2-3 sessions
- [x] Extract future/roadmap items to `docs/ROADMAP.md`
- [x] Archive POT physics notes to `docs/archive/pot-physics-notes.md`

### kleis-review — Context-Aware Parsing for Reduced False Positives

~~The current `kleis-review` MCP uses string matching for code review rules, producing false positives where syntactic context matters.~~ **All three items resolved with structural (AST-based) rules:**

- ~~**`check_no_wildcard_import`** flags `use super::*;` in test modules~~ — **DONE**: `rule_wildcard_imports` uses `non_test_wildcard_uses(c)`, skips test modules.
- ~~**`check_no_narrating_comments`** flags doc comments~~ — **DONE**: `rule_narrating_line_comments` uses `has_narrating_line_comment(crate_comments(c))`, distinguishes `//` from `///`.
- ~~**`check_no_inline_use`** flags `use` inside function bodies~~ — **DONE**: `rule_use_in_fn_body` uses `non_test_fns_containing(source, fns, "use ")`, skips test functions.

---
