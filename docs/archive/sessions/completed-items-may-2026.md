# Completed Items Archive — May 2026

Archived from `docs/NEXT_SESSION.md` on May 18, 2026.
These sections contain detailed implementation narratives for completed work.

---

## Graph Editor — Completed Phase Details (Phases 1–8)

### What's built (Phase 1)

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

### Domain config fields

| Field | Values | Default |
|-------|--------|---------|
| `routing_mode` | `"orthogonal"`, `"direct"`, `"curved"` (future) | `"orthogonal"` |
| `junction_style` | `"dot"`, `"none"`, `"bar"` | `"dot"` |
| `multi_port_strategy` | `"trunk_branch"`, `"star"`, `"bus"` (future) | `"trunk_branch"` |
| `edge_decoration` | `"none"`, `"arrow"`, `"half_arrow"`, `"inhibitor"` | `"none"` |
| `edge_direction` | `"undirected"`, `"directed"` | `"undirected"` |

### To add a new graph domain

1. Create `std_template_lib/<domain>.kleist` with component templates
   (each needs `ports:`, `graph_width:`, `graph_height:` metadata and an SVG)
2. Add `@template __domain_<name>` block with routing preferences
3. Place SVG assets in `static/svg/<domain>/`
4. No JavaScript, no Rust, no recompilation

### Files

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

### Phase 2: Edge decorations and direction — DONE
- SVG marker definitions (arrowhead, half-arrow, inhibitor, causal bar)
- `marker-start`/`marker-end` attributes based on `edge_decoration`
- Typst export emits `#polygon` arrowheads for directed edges
- `DECORATION_MARKER_ID` lookup table maps config names to marker IDs

### Phase 3: Bond graph templates + direct routing — DONE
- `std_template_lib/bond_graph.kleist` with 9 elements (Se, Sf, R, C, I, TF, GY, 0-junction, 1-junction)
- `__domain_bond_graph` config: `routing_mode: "direct"`, `edge_decoration: "half_arrow"`, `edge_direction: "directed"`
- 9 SVG assets under `static/svg/bond_graph/`
- `?domain=X` URL parameter filters palette and domain config (e.g. `?domain=bond`, `?domain=electronics`)
- Direct routing fixes: segment drag, dbl-click split, waypoint drag disabled; cursor classes corrected
- Per-net causal stroke: `net.causal = 'start' | 'end' | null`, toggled with `K` key
- `causality_type` metadata on each template for future SCAP algorithm
- Typst export includes causal bar rendering

### Phase 4: Petri net / workflow net templates — DONE
- `std_template_lib/petri_net.kleist` with 4 component types + `__domain_petri_net` config
- Source place, Regular place, Sink place, Transition
- `edge_decoration: "arrow"`, `edge_direction: "directed"`, `routing_mode: "orthogonal"`
- SVGs: `static/svg/petri_net/{place,source_place,sink_place,transition}.svg`
- component_type metadata: `SourcePlace`, `Place`, `SinkPlace`, `Transition`

**Petri net ↔ BPMN mapping:**

| BPMN Element   | Workflow Net Element           | Kleis Template       |
|----------------|-------------------------------|----------------------|
| Start event    | Source place (initial token)   | `pn_source_place`    |
| End event      | Sink place                    | `pn_sink_place`      |
| Activity/Task  | Transition                    | `pn_transition`      |
| Condition      | Place                         | `pn_place`           |
| XOR split      | Place with multiple out-arcs  | (topology, not type) |
| AND split      | Transition with multiple outs | (topology, not type) |
| Sequence flow  | Directed arc                  | (wire with arrow)    |

### Phase 5: Component parameters + structural VERIFY — DONE
- `params` metadata in `.kleist`: `"name:type:default"` semicolon-separated (like `ports`)
- Petri nets: `pn_source_place(tokens:int:1)`, `pn_place(tokens:int:0)`, `pn_sink_place(tokens:int:0)`
- Electronics: `resistor(R:real:1000)`, `capacitor(C:real:1e-6)`, etc.
- Property panel: editable inputs for each param when component selected
- AST encoding: `resistor(1000)` not `resistor()` — params become operation args
- **VERIFY button**: generic data-driven structural checks from `verify_*` domain metadata

**Parameter type system (data-driven, no domain JS):**

| Type | Example | UI widget | AST encoding |
|------|---------|-----------|-------------|
| `real` | R=1000 | number input | operation arg |
| `int` | tokens=3 | number input (step=1) | operation arg |
| `enum` | element=C | dropdown (future) | operation arg |
| `ref` | model=2N2222 | dropdown from server (future) | operation arg (name) |

### Phase 6: Z3 verification via companion `.kleis` theory — DONE
- Companion `.kleis` file convention: `std_template_lib/petri_net.kleis` next to
  `petri_net.kleist`. Domain config references it via `verify_theory: "petri_net"`.
- **CRITICAL ARCHITECTURAL DECISION: `server.rs` contains ZERO domain-specific code.**
- Server `POST /api/verify_graph` endpoint with domain-agnostic preamble
- Domain-agnostic preamble (`build_graph_preamble`) emits: `graph_nc`, `graph_nn`,
  `graph_ctype(c)`, `graph_param(c, j)`, `graph_inc(net, comp)`, `TYPE_X` constants,
  closed-world axioms, theory scanning for `TYPE_X` references
- 23 Rust tests covering preamble structure and Z3 pass/fail cases
- BMC removed (contained domain-specific code)

### Phase 7 (planned): Causal Network Verification Theories

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

### Phase 8 (DONE): Theory-Driven Simulation

- `petri_net.kleis` contains simulation functions: `sim_enabled(t)`,
  `sim_fire(t, c)`, `sim_halted()`, `sim_halt_reason()`
- `build_sim_preamble()` generates concrete `define` statements
- Server `simulate_graph_core()` calls `eval_concrete` on theory-defined functions
- 9 tests pass including 5-token pipeline completing in 10 steps
- **To add simulation for a new domain:** implement `sim_enabled`/`sim_fire`/
  `sim_halted`/`sim_halt_reason` in the companion `.kleis` theory file. No Rust
  or JS changes needed.

---

## Electronics MNA Theory — Completed Phase Details (Phases 1–10)

### What we proved (session May 14, 2026)

1. **Symbolic Jacobian works.** `diff()` from `stdlib/symbolic_diff.kleis` computes
   exact Jacobian entries for diode (Shockley), BJT (Ebers-Moll), and MOSFET (Level 1).

2. **No bridge function needed.** Native Kleis arithmetic handles all numeric computation.

3. **All builtins exist.** `solve`/`linsolve` (LAPACK) for J·Δx = -F, `matrix` for
   assembly, `eigenvalues` for stability, `fft`/`dft` for frequency analysis.

4. **Constitutive equations verified at concrete operating points:**

   | Component | Jacobian entry | Value at operating point |
   |-----------|---------------|-------------------------|
   | Diode g_d (0.7V forward) | `(Is/nVt)·exp(Vd/nVt)` | 0.287 S |
   | Diode g_d (-1V reverse) | same formula | 1.43×10⁻¹⁷ S |
   | BJT g_m (0.65V active) | `(Is/Vt)·exp(Vbe/Vt)` | symbolic exp(25.15) |
   | MOSFET g_m (3V sat) | `Kp·(Vgs-Vth)` | exactly 0.002 S |

### Phase 1: eval_concrete setup interface — DONE
- `sim_state_count`, `sim_state_map(i)`, `sim_input_map(k)`, `sim_connected_net(c)`
- `sim_initial_state(i)`, `sim_input_value(k)` — same recursive walker pattern

### Phase 2: Constitutive equations — DONE
- `diode_current(Vd, Is, nd, Vt)` and `diode_conductance(Vd, Is, nd, Vt)`
- Voltage limiting via `diode_vcrit(nd, Vt) = 10*nd*Vt`

### Phase 3: MNA Jacobian assembly — DONE
- `stamp_2port_J(np, nm_, g, n, m)` — generic symmetric ±g pattern for 2-port
- Per-type wrappers, port-order-based terminal identification
- `mna_build_J(v)` and `mna_build_F(v)` assembled via `list_fold` over components

### Phase 4: Newton-Raphson sim_step — DONE
- `nr_iterate(v, iter)` — recursive NR with convergence check (tol = 1e-9, max 50)
- `sim_step(i) = extract_state(i, nr_iterate(nr_initial_guess, 0))`

### Phase 5: Z3 verification — EXISTING (structural only)
- Ground exists, source exists, load exists, no parallel V sources, no series I sources

### Phase 6: Server integration — DONE (zero domain-specific code)
- `server.rs` stays domain-agnostic. Electronics uses `sim_mode: "continuous"`.
- `graph_port_net_val(p)` and `graph_comp_port0_val(c)` added to preamble

### Phase 7: Adaptive timestep — DEFERRED

### Phase 8: BJT support + multivibrator test — DONE
- 3-terminal net helpers, simplified Ebers-Moll model
- NPN Jacobian stamp: 3x3 sub-pattern at (base, coll, emit) nodes
- Tests: `multivibrator_setup_ok`, `electronics_multivibrator_oscillates`

**Multivibrator test findings:**
- Circuit: 2x NPN cross-coupled via capacitors (R1=R2=1k, R3=R4=10k, C1=C2=10uF, Vcc=5V)
- After 100 steps (dt=0.1ms): C1=0.92V, C2=0.85V — physically correct initial transient

### Phase 9: Sparse stamp optimization — DONE

- `assemble_matrix(n, entries)` and `assemble_vector(n, entries)` builtins

**Performance results:**

| Test                     | Before (dense) | After (sparse) | Speedup |
|--------------------------|---------------|----------------|---------|
| Rectifier (100 steps)    | 5.7s          | 1.24s          | 4.6x    |
| Multivibrator (100 steps)| 128s          | 12.0s          | 10.7x   |

**Scaling path for IC-scale circuits:**

| Scale             | Assembly                     | Solver                           |
|-------------------|-----------------------------|---------------------------------|
| Small (< 50 nodes)| `assemble_matrix` (dense)   | LAPACK `dgesv` (current)        |
| Medium (50-5000)  | CSC build from entries       | KLU sparse direct               |
| Large (5000+)     | CSC build from entries       | Iterative (GMRES + precond.)    |

### Phase 10: Graph Save/Load — DONE

- `POST /api/save_graph`, `GET /api/load_graph`, `GET /api/list_graphs`
- Save format: valid Kleis program with `define` statements

**Seed files:**
- `examples/petri-nets/graph-editor/linear.kleis`
- `examples/electronics/graph-editor/rectifier.kleis`
- `examples/electronics/graph-editor/multivibrator.kleis`
- `examples/bond-graph/graph-editor/rc_circuit.kleis`

**Fine-grained component types in electronics.kleist:**
- Resistor, Capacitor, Inductor, Diode, LED, ZenerDiode, NPN, PNP, NMOS, PMOS,
  OpAmp, VoltageSource, ACVoltageSource, CurrentSource, Ground, Connector, Measurement

---

## ES Module Extraction — DONE

**Branch:** `refactor/es-module-extraction`
**PRs:** eatikrh/kleis#64, engingithub/kleis#62

Extracted ~3,100 lines of inline JavaScript from `static/index.html` into
17 ES module files under `static/js/`. The HTML file is now 1,232 lines
(markup + CSS only), loading `<script type="module" src="/static/js/main.js">`.

Modules: `state.js`, `astUtils.js`, `render.js`, `undoRedo.js`,
`inlineEdit.js`, `slotHandlers.js`, `palette.js`, `egyptian.js`,
`matrixBuilder.js`, `piecewiseBuilder.js`, `debug.js`, `verify.js`,
`modeConvert.js`, `jupyter.js`, `keyboard.js`, `gallery.js`, `main.js`.

---

## K-Q Seven-Paper Inventory

| # | Paper | Theory file | Paper file | Results |
|---|-------|-------------|------------|---------|
| 1 | φ⁴ one-loop | pot_phi4_oneloop.kleis | pot_phi4_oneloop_paper.kleis | 18 worked |
| 2 | QED vacuum pol. | pot_qed_vacuum_polarization.kleis | pot_qed_vacuum_polarization_paper.kleis | 15 worked |
| 3 | Yang-Mills | pot_ym_vacuum_polarization.kleis | pot_ym_vacuum_polarization_paper.kleis | 14 worked |
| 4 | Ghost theorem | pot_ghost_activity_theorem.kleis | pot_ghost_activity_theorem_paper.kleis | 17 Z3 |
| 5 | Gauge dependence | pot_gauge_dependence_ghost.kleis | pot_gauge_dependence_ghost_paper.kleis | 16 Z3 |
| 6 | Structural atlas | pot_ker_q_atlas.kleis | pot_ker_q_atlas_paper.kleis | 24 Z3 |
| 7 | Abstract K-Q framework | pot_abstract_kq_framework.kleis | pot_abstract_kq_framework_paper.kleis | 24 Z3 |
