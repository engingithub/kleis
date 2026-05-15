# ADR-037: Graph Editor with Domain-Agnostic Routing and Verification

**Status:** Accepted & Implemented (Phases 1–8)
**Date:** 2026-04-27 (original), 2026-05-14 (Phase 8 + theory-driven simulation)
**Relates to:** ADR-005 (Visual Authoring), ADR-022 (Z3 Integration), ADR-023
(Template Externalization), ADR-035 (Multi-Domain Template Compiler), ADR-036
(Multi-Domain Template Generality)

## Context

ADR-036 identified three levels of circuit support and classified "Level 3:
Graph editor — KiCad-like 2D canvas with drag-and-drop wiring" as requiring a
graph AST and deferred it indefinitely. Investigation and prototyping revealed
that graph editing is achievable without a new AST type — graphs are represented
as **incidence matrices** within the existing Kleis AST, and a standalone Graph
Editor application handles the visual editing.

The key insight: the Equation Editor's tree AST and the Graph Editor's graph
model are not competing representations. A graph's topology (incidence matrix)
and its component list are **values inside the AST**, serialized as a `graph()`
operation:

```json
{
  "Operation": {
    "name": "graph",
    "args": [
      { "Operation": { "name": "SparseMatrix", "args": [V, P, [triples...]] } },
      { "List": [component operations...] },
      { "List": [net labels...] },
      { "List": [port labels...] }
    ]
  }
}
```

This representation flows through the existing `editor_node_to_expression`
translation, type inference, and Z3 verification pipelines.

A second insight: different graph domains (electronics, bond graphs, Petri nets,
molecular graphs) share the same incidence-matrix topology model but require
fundamentally different visual behavior — routing algorithms, edge decorations,
junction styles, and directionality. Hardcoding these per domain in JavaScript
would prevent end-user extensibility.

## Decision

### 1. Separate Graph Editor Application

The Graph Editor is a standalone web application (`static/graph_editor.html`,
`static/js/graphEditorMain.js`) that is a sibling to the Equation Editor, not an
extension of it. This separation:

- Avoids shoehorning graph-type UI into equation-type UI
- Allows dedicated interaction paradigms (pan/zoom, wire routing, port
  connections)
- Prevents regressions in the existing Equation Editor
- Enables independent evolution of both editors

### 2. Signed Sparse Port-Based Incidence Matrix

Graphs are represented as **port-based** incidence matrices within the Kleis
AST. The matrix rows correspond to nets and columns to individual **ports**
(not components). A transistor with 3 ports (base, collector, emitter) occupies
3 columns, so port identity is preserved by column position.

Entries are **signed integers**:

| Value | Meaning |
|-------|---------|
| `+1`  | First port of component connected to this net (source / positive) |
| `-1`  | Non-first port connected to this net (sink / negative) |
| `+n`/`-n` | Weighted connection (e.g., double bond in molecular graphs) |
| `0`   | No connection (not stored in sparse format) |

The sign convention follows standard algebraic graph theory: each net assigns
`+1` to its "source" end and `-1` to its "sink" end. For undirected domains the
signs still provide algebraic orientation, enabling axioms like "each row sums
to zero for a simple two-port net." For directed domains (bond graphs, Petri
nets), the signs carry physical meaning.

**Storage format:** COO (Coordinate List) — only non-zero entries are stored as
`(net_index, port_index, value)` triples, flattened into a single list:

```
graph(
  SparseMatrix(V, P, [net0, port0, val0, net1, port1, val1, ...]),
  [component operations...],     // component types with parameters
  [net labels...],               // net names
  [port labels...]               // "componentIdx:portName" per column
)
```

The dense V x P matrix can be materialized from COO when needed (e.g., for
display or axiom evaluation), but the canonical AST representation is sparse.
This is efficient for large, sparsely connected graphs and maps directly to
Z3 assertions without iterating over zero entries.

This representation is complete: given the sparse matrix + component list + port
labels, the full port-level connectivity can be reconstructed. It is universal
across domains: electronic circuits, bond graphs, Petri nets, and molecular
graphs all reduce to signed sparse port-based incidence matrices.

**Why port-based, not component-based:** A component-level matrix (nets x
components) with +1/-1 polarity encoding cannot distinguish which port of a
multi-port component (3+ ports) is connected to a given net. Port-based columns
eliminate this ambiguity — the column IS the port.

**Why signed:** Unsigned binary entries cannot represent edge direction (needed
for bond graphs, Petri nets) or bond order (needed for molecular graphs). Signed
integers handle direction, weight, and algebraic orientation in a single entry.
The `domainConfig.edge_direction` field guides interpretation: in `"undirected"`
domains the signs are algebraic bookkeeping; in `"directed"` domains they encode
physical flow direction.

### 3. Domain Configuration via `.kleist` Metadata

Domain-specific routing and rendering behavior is declared in `.kleist` template
files using the `@template __domain_<name>` naming convention. The existing
`/api/templates` endpoint carries this metadata to the client without server
changes.

```kleist
@template __domain_electronics {
    pattern: "__domain_electronics()"
    category: "__domain"
    routing_mode: "orthogonal"
    junction_style: "dot"
    multi_port_strategy: "trunk_branch"
    edge_decoration: "none"
    edge_direction: "undirected"
}
```

Configuration fields:

| Field | Values | Purpose |
|-------|--------|---------|
| `routing_mode` | `"orthogonal"`, `"direct"`, `"curved"` (future) | Wire routing algorithm |
| `junction_style` | `"dot"`, `"none"`, `"bar"` | Visual marker at T-junctions |
| `multi_port_strategy` | `"trunk_branch"`, `"star"`, `"bus"` (future) | How 3+ port nets are routed |
| `edge_decoration` | `"none"`, `"arrow"`, `"half_arrow"`, `"inhibitor"` | Edge visual style |
| `edge_direction` | `"undirected"`, `"directed"` | Whether connections have directionality |

Defaults (when no `__domain_` template exists): `orthogonal`, `dot`,
`trunk_branch`, `none`, `undirected`.

### 4. Trunk+Branch Routing for Multi-Port Nets

Multi-port nets (3+ connections) use a trunk+branch algorithm instead of the
previous star topology:

1. **Pick trunk pair**: The two ports with greatest spatial distance become trunk
   endpoints, maximizing the wire length available for branches.
2. **Route trunk**: Use the standard exit-direction-aware waypoint algorithm
   (`computeDefaultWaypoints`) for the main wire.
3. **Route branches**: For each remaining port, project its position onto the
   trunk polyline to find the nearest point. Route a perpendicular stub from the
   port to that junction point, respecting the port's exit direction.
4. **Render**: Draw the trunk as a single `<path>`, each branch as a short
   `<path>`, and a junction marker (configurable via `junction_style`) at each
   branch point.

The algorithm is generic — it works for any domain using `orthogonal` routing
mode. Domains using `direct` routing get straight-line connections (no
waypoints). The `multi_port_strategy` field selects between `trunk_branch` and
`star` (centroid) strategies.

### 5. Parameterized Components

Components can accept domain-specific parameters (e.g., resistance for a
resistor, token count for a Petri net place). Parameters are declared in
`.kleist` template metadata:

```kleist
@template pn_place {
    params: "tokens:int:0"
    componentType: "Place"
}

@template resistor {
    params: "R:real:1000"
    componentType: "Passive"
}
```

Format: `"name:type:default"`, comma-separated for multiple params. The Graph
Editor parses this metadata, initializes component state with defaults, and
renders a property panel when a component is selected. Parameter values flow into
the AST as operation arguments:

```json
{ "Operation": { "name": "resistor", "args": [{ "Const": "1000" }] } }
```

### 6. Domain-Agnostic Verification Architecture

The VERIFY button triggers a two-stage verification pipeline:

**Stage 1: Client-side structural checks (JavaScript)**

Generic checks driven by `verify_*` metadata in the `__domain_` template:

| Metadata Key | Check |
|-------------|-------|
| `verify_bipartite` | Every arc crosses two different component roles |
| `verify_no_isolated` | No component lacks connections |
| `verify_requires_type` | At least one component of each required type exists |
| `verify_all_connected` | Graph is connected |
| `verify_exactly_one` | Exactly one component of a given type |

These checks are fast, run entirely in the browser, and provide immediate
feedback. They use no domain-specific JavaScript logic — the check names map to
generic graph analysis functions.

**Stage 2: Z3-based deep verification (server-side)**

If structural checks pass and `verify_theory` is defined in the domain config,
the client POSTs to `POST /api/verify_graph` with graph data (components,
incidence matrix, port labels, domain name).

The server then:

1. **Generates a domain-agnostic preamble** from the graph data
2. **Loads the companion `.kleis` theory** file (`std_template_lib/{domain}.kleis`)
3. **Concatenates** preamble + theory source and evaluates with Z3
4. **Returns** per-example pass/fail results

### 7. Domain-Agnostic Graph Preamble

**Critical architectural constraint: `server.rs` contains zero domain-specific
code.** The server does not know what a "place," "transition," "resistor," or
"bond" is. It emits only generic graph primitives:

```kleis
// Counts
operation graph_nc : ℤ          // number of components
operation graph_nn : ℤ          // number of nets

// Per-component type codes (integer-coded, auto-assigned)
operation graph_ctype : ℤ → ℤ   // component index → type code
operation graph_param : ℤ × ℤ → ℤ  // (component, param_index) → value

// Component-level incidence matrix (aggregated from port-level)
operation graph_inc : ℤ × ℤ → ℤ    // (net, component) → signed value
```

The preamble emits a `GraphData` structure with:

- **Component counts** and **type code assignments**
- **Parameter values** (positional, from the component's `params` map)
- **Component-level incidence** (port-level entries aggregated per component)
- **Closed-world axioms** for `graph_ctype`, `graph_inc`, and `graph_param`
  (preventing Z3 from inventing values for unconstrained inputs)
- **Distinctness axioms** for all TYPE codes (preventing Z3 from equating
  different component roles)

**Theory-declared type codes:** TYPE codes (e.g., `TYPE_Place`,
`TYPE_Resistor`) are **declared by the companion `.kleis` theory**, not by the
server. The theory uses standard Kleis `operation` declarations:

```kleis
// In petri_net.kleis — the theory declares its own requirements
operation TYPE_Place : ℤ
operation TYPE_SourcePlace : ℤ
operation TYPE_SinkPlace : ℤ
operation TYPE_Transition : ℤ
```

The preamble generator **parses the theory** as a Kleis program, extracts all
`OperationDecl` items whose name starts with `TYPE_`, and assigns each a unique
integer code. This is AST-based extraction, not text scanning — only properly
parsed `operation` declarations are recognized:

```rust
if let Ok(theory_program) = parse_kleis_program(theory_source) {
    for op in theory_program.operations() {
        if let Some(suffix) = op.name.strip_prefix("TYPE_") { ... }
    }
}
```

The preamble then provides **axiom values** for each TYPE code inside the
`GraphData` structure, without re-declaring the operations (the theory already
declared them):

```kleis
structure GraphData {
    axiom type_Place: TYPE_Place = 1
    axiom type_Transition: TYPE_Transition = 2
    // ...
}
```

**Type code assignment rules:**

1. Types present in the graph's components are assigned codes 1, 2, 3, ...
   (in insertion order)
2. Types declared by the theory but absent from the graph are assigned higher
   codes that no component has — this ensures Z3 doesn't confuse an absent type
   with a present one
3. If a component in the request has a type the theory didn't declare, the
   preamble emits an `operation TYPE_X : ℤ` declaration for it (fallback for
   forward compatibility)

This architecture means the **theory is the contract**: it declares exactly what
type codes it expects, the preamble fills in the values, and Z3 enforces
consistency. A theory that references `TYPE_Foo` without declaring it will
produce a parse error, not a silent Z3 misinterpretation.

### 8. Companion Theory Files

Each domain's verification logic lives entirely in a companion `.kleis` file.
The theory has three layers:

**Layer 1 — Type declarations:** The theory declares the type codes it requires
from the preamble. These are `operation` declarations with no axioms — the
preamble provides concrete values at verification time.

**Layer 2 — Domain interpretation:** `define` statements map generic graph
primitives to domain concepts. These are pure abbreviations — no Z3 cost.

**Layer 3 — Verification assertions:** `example` blocks express properties that
Z3 must prove from the graph data. Each example is a named, independently
checked assertion.

Example — `std_template_lib/petri_net.kleis`:

```kleis
// Layer 1: type declarations (preamble fills in values)
operation TYPE_Place : ℤ
operation TYPE_SourcePlace : ℤ
operation TYPE_SinkPlace : ℤ
operation TYPE_Transition : ℤ

// Layer 2: domain interpretation of generic graph primitives
define is_place(c) =
    graph_ctype(c) = TYPE_Place ∨
    graph_ctype(c) = TYPE_SourcePlace ∨
    graph_ctype(c) = TYPE_SinkPlace

define is_transition(c) = graph_ctype(c) = TYPE_Transition
define tokens(c) = graph_param(c, 0)

// Layer 3: structural verification via Z3
example "INITIAL MARKING: some component has tokens" {
    assert(∃(c : ℤ). c ≥ 0 ∧ c < graph_nc ∧ is_place(c) ∧ tokens(c) ≥ 1)
}

example "BIPARTITE: every arc crosses place/transition boundary" {
    assert(∀(n : ℤ). ∀(c1 : ℤ). ∀(c2 : ℤ).
        (n ≥ 0 ∧ n < graph_nn ∧ c1 ≥ 0 ∧ c1 < graph_nc ∧
         c2 ≥ 0 ∧ c2 < graph_nc ∧ ¬(c1 = c2) ∧
         ¬(graph_inc(n, c1) = 0) ∧ ¬(graph_inc(n, c2) = 0))
        → ¬(is_place(c1) ∧ is_place(c2)) ∧
          ¬(is_transition(c1) ∧ is_transition(c2)))
}
```

The three layers form a clear contract:

| Layer | What it says | Who provides it |
|-------|-------------|-----------------|
| Type declarations | "I need `TYPE_Place`, `TYPE_Transition`, ..." | Theory declares, preamble fills values |
| Domain interpretation | "`is_place(c)` means `graph_ctype(c) = TYPE_Place ∨ ...`" | Theory defines |
| Verification assertions | "Every arc must cross a place/transition boundary" | Theory asserts, Z3 proves |

**Adding verification for a new domain** requires only writing a companion
`.kleis` file that follows this three-layer pattern. No Rust changes, no
JavaScript changes.

### 9. Shared Primitives Across Theories

All companion theories share the same set of preamble-provided primitives:

| Primitive | Type | Meaning |
|-----------|------|---------|
| `graph_nc` | `ℤ` | Number of components |
| `graph_nn` | `ℤ` | Number of nets |
| `graph_ctype(c)` | `ℤ → ℤ` | Type code for component `c` |
| `graph_param(c, j)` | `ℤ × ℤ → ℤ` | j-th parameter of component `c` |
| `graph_inc(n, c)` | `ℤ × ℤ → ℤ` | Incidence matrix entry (net `n`, component `c`) |

These primitives are always available. Domain-specific type codes (`TYPE_X`) are
declared by each theory and filled by the preamble. This means theories are
composable in principle — a theory that imports another can reuse its type
declarations and domain definitions.

### 10. How to Add a New Graph Domain

Adding a new domain to the Graph Editor requires **only data files** — no Rust,
no JavaScript, no recompilation. The following steps are sufficient:

#### Step 1: Create SVG assets

Place component SVG files in `static/svg/<domain>/`. Each SVG should be a clean
symbol with consistent viewBox dimensions. The SVG filename must match the
template name.

#### Step 2: Create the `.kleist` template file

Create `std_template_lib/<domain>.kleist` with:

**a) Domain configuration block** — declares routing and verification behavior:

```kleist
@template __domain_<name> {
    pattern: "__domain_<name>()"
    category: "__domain"
    routing_mode: "orthogonal"       // or "direct", "curved"
    junction_style: "dot"            // or "none", "bar"
    multi_port_strategy: "trunk_branch"  // or "star", "bus"
    edge_decoration: "none"          // or "arrow", "half_arrow", "inhibitor"
    edge_direction: "undirected"     // or "directed"
    verify_no_isolated: "true"       // generic structural checks
    verify_theory: "<domain>"        // enables Z3 verification
}
```

**b) Component templates** — one per component type, with ports and metadata:

```kleist
@template my_component {
    pattern: "my_component()"
    category: "<domain>_<group>"
    svg: "/static/svg/<domain>/my_component.svg"
    ports: "in:left:0:50,out:right:100:50"
    component_type: "MyType"
    params: "value:real:100"
    typst: "#my_typst_rendering()"
}
```

Port format: `"name:side:x:y"` (comma-separated, coordinates relative to SVG).
Params format: `"name:type:default"` (comma-separated for multiple).

#### Step 3: (Optional) Create a companion `.kleis` theory

Create `std_template_lib/<domain>.kleis` following the three-layer pattern:

```kleis
// Layer 1: declare required type codes
operation TYPE_MyType : ℤ
operation TYPE_OtherType : ℤ

// Layer 2: domain interpretation
define is_my_type(c) = graph_ctype(c) = TYPE_MyType

// Layer 3: verification assertions
example "MY CHECK: at least one MyType component" {
    assert(∃(c : ℤ). c ≥ 0 ∧ c < graph_nc ∧ is_my_type(c))
}
```

**The `component_type` values in `.kleist` must match the TYPE declaration
suffixes in `.kleis`.** If a template has `component_type: "Resistor"`, the
theory must declare `operation TYPE_Resistor : ℤ`.

#### Step 4: Test

Open `http://localhost:3000/static/graph_editor.html?domain=<domain>`. The
palette should show component categories, components should be placeable, and
the VERIFY button should run both structural and Z3 checks.

#### Summary of files

| File | Purpose |
|------|---------|
| `static/svg/<domain>/*.svg` | Component artwork |
| `std_template_lib/<domain>.kleist` | Templates, ports, domain config |
| `std_template_lib/<domain>.kleis` | Verification theory (optional) |

No other files need to be created or modified.

## Implementation Status

### Phase 1 (Implemented)

- Graph Editor PoC with component placement, rotation, and deletion
- Port-based connection model with orthogonal (Manhattan) wire routing
- Exit-direction-aware waypoint generation for 2-port nets
- Trunk+branch routing for multi-port nets with junction dots
- Interactive waypoint manipulation (drag segments, add/remove waypoints)
- Collinear segment merging and auto-rerouting on component drag
- SVG `viewBox`-based pan and zoom
- Domain configuration loading from `__domain_` templates
- Routing mode dispatch (`orthogonal` vs `direct`)
- Typst schematic export with proper scaling and rotation
- **Signed sparse incidence matrix** in COO format (JS-based)
  - Entries are signed integers: +1 (first port), -1 (non-first port)
  - AST topology node changed from `Matrix(V,P,[dense])` to
    `SparseMatrix(V,P,[net,port,val,...])`
  - `renderMatrixHTML` materializes dense view from COO for display

### Phase 2 (Implemented): Edge Decorations and Direction

- SVG marker definitions (arrowhead, half-arrow, causal bar)
- `marker-start`/`marker-end` attributes based on `edge_decoration`
- Direction encoding in incidence matrix entries for directed domains
- Data-driven edge decoration via `.kleist` domain config

### Phase 3 (Implemented): Bond Graph Templates

- `bond_graph.kleist` with effort/flow sources, R/C/I elements, transformers,
  gyrators, 0-junctions, and 1-junctions
- Direct routing mode with half-arrow decorations
- Causal stroke rendering on bond connections
- `componentType` metadata for causality analysis
- SVG assets in `static/svg/bond_graph/`

### Phase 4 (Implemented): Petri Net Templates

- `petri_net.kleist` with place, transition, source place, and sink place
  templates
- Orthogonal routing with arrow decorations for directed arcs
- `componentType` metadata (Place, Transition, SourcePlace, SinkPlace)
- SVG assets in `static/svg/petri_net/`

### Phase 5 (Implemented): Parameters and Verification

- **Parameterized components**: `params` metadata in `.kleist` templates with
  type/default declarations; property panel in Graph Editor for editing values
- **Client-side structural verification**: generic `verifyGraph()` function
  driven by `verify_*` metadata flags
- **Server-side Z3 verification**: `POST /api/verify_graph` endpoint with
  domain-agnostic preamble generation and companion theory evaluation
- **Domain-agnostic preamble**: `build_graph_preamble()` emits generic graph
  primitives (counts, type codes, params, incidence, closed-world axioms)
- **Theory type extraction**: preamble parses companion theory AST to discover
  declared `TYPE_X` operations and assign integer codes
- **Companion theory**: `petri_net.kleis` derives domain semantics entirely from
  generic primitives
- **23 tests** covering preamble structure, Z3 verification (pass and fail
  cases), missing theories, and multi-domain scenarios

### Phase 6 (Partial): Extended Verification

- Token visualization in Petri net places — **not yet implemented**
- Arc weight support (weighted edges in incidence matrix) — **not yet implemented**
- ~~Companion theories for electronics and bond graph domains~~ — **done in Phase 7**
- Simulation integration with existing ODE solver — **not yet implemented**

### Phase 7 (Implemented): Causal Network Verification Theories

- **Bond graph companion theory** (`std_template_lib/bond_graph.kleis`):
  9 type codes (EffortSource, FlowSource, Resistor, Capacitor, Inertia,
  Transformer, Gyrator, Junction0, Junction1), 7 Z3 assertions covering
  source existence, 1-port existence, effort/flow conflict on junctions,
  port connectivity, and junction connectivity
- **Electronics companion theory** (`std_template_lib/electronics.kleis`):
  7 type codes (VoltageSource, CurrentSource, Ground, Passive, Active,
  Connector, Measurement), 5 Z3 assertions covering ground existence, source
  existence, load existence, parallel voltage source detection, and series
  current source detection
- **Refined `component_type` values** in `.kleist` templates: split generic
  "Source" into domain-specific types (e.g., `EffortSource` vs `FlowSource`
  for bond graphs; `VoltageSource` vs `CurrentSource` for electronics)
- **Explicit type declarations**: theory files declare `operation TYPE_X : ℤ`
  for every type they reference; preamble extracts these from the parsed AST
  instead of scanning theory text
- **Always-run Z3 verification**: client-side `verifyGraph()` no longer
  short-circuits — Z3 checks run even when structural checks fail
- **33 Rust tests** covering preamble structure, Z3 verification (pass and
  fail cases), theory loading, and the new type declaration extraction

### Phase 8 (Implemented): Theory-Driven Simulation

Simulation was initially hardcoded in `server.rs` with Petri-net-specific logic
(enabled detection, token transfer, halt classification). This violated the
domain-agnostic principle established for verification. Phase 8 refactors
simulation to mirror the verification architecture: domain-specific semantics
live entirely in `.kleis` theory files, executed via `eval_concrete`.

**Architecture:** Two preamble modes in the same theory file:

| Mode | Preamble Style | Execution Engine | Purpose |
|------|---------------|-----------------|---------|
| Verification | `operation` + `axiom` in `GraphData` | Z3 | Symbolic ∀/∃ proofs |
| Simulation | `define` with `nth`-based lookup | `eval_concrete` | Concrete step-by-step execution |

**Simulation preamble** (`build_sim_preamble`): generates concrete `define`
statements instead of Z3 operations/axioms:

```kleis
define sim_state = [5, 0, 0, 0, 0]         // current state (updated per step)
define graph_nc_val = 5                      // component count
define graph_nn_val = 4                      // net count
define graph_ctype_val(c) = nth([1,2,3,2,4], c)   // type codes as list
define graph_inc_val(n, c) = nth(nth([...], n), c) // incidence as nested lists
define TYPE_Place = 3                        // concrete type code values
define TYPE_Transition = 2
```

**Theory simulation interface** (contract that any simulation-enabled theory
must implement):

| Function | Returns | Semantics |
|----------|---------|-----------|
| `sim_enabled(t)` | `true`/`false` | Is component `t` enabled to fire? |
| `sim_fire(t, c)` | integer | New state value for component `c` after `t` fires |
| `sim_halted()` | `true`/`false` | Are no components enabled? |
| `sim_halt_reason()` | `"completed"`/`"deadlock"` | Why did simulation stop? |

**Recursive enumeration:** Since `eval_concrete` has no loop construct, the
theory uses bounded recursion over `graph_nc_val` and `graph_nn_val` to
enumerate components and nets. For example, `sim_enabled(t)` recursively
checks all nets via `sim_all_nets_ok(t, graph_nn_val - 1)`, and each net
recursively scans all places via `sim_net_has_source(t, n, graph_nc_val - 1)`.

**Performance optimization for multi-step:** The server parses the full
preamble + theory once, then for each step only re-parses a single-line
`define sim_state = [...]` update that overrides the previous state in the
evaluator's function table. This avoids full re-parse overhead during `Run`
actions.

**Bug fix:** Added `"logical_and"` and `"logical_or"` aliases to
`eval_concrete` builtins. The parser generates these names for `∧`/`∨`
operators, but only `"and"`/`"or"` and Unicode symbols were previously handled.

**Test results:** 10 simulation tests pass, including a 5-token pipeline that
correctly processes all tokens through a linear workflow (Source → T0 → Place
→ T1 → Sink) in 10 steps with round-robin transition selection.

### WASM Status

WASM was initially prototyped for graph logic but removed from the active code
path due to overhead. All graph computations (incidence matrix construction,
AST generation, preamble generation) are currently implemented in JavaScript
(client-side) and Rust (server-side, synchronous). WASM may be revisited for
computationally intensive operations (e.g., large graph layout, ODE simulation).

## Consequences

### Positive

1. **Level 3 graph editing is no longer deferred.** ADR-036 marked it as "out of
   scope"; this ADR delivers it.
2. **The incidence matrix representation is universal.** Electronics, bond
   graphs, Petri nets, and molecular graphs all use the same data model.
3. **Domain extensibility is data-driven.** New graph domains require only
   `.kleist` files, SVG assets, and optional `.kleis` theory files. The routing
   engine and verification pipeline are generic.
4. **The existing AST is unchanged.** Graphs are values inside the tree AST, not
   a competing representation. All existing Kleis infrastructure (type inference,
   Z3, evaluation) continues to work.
5. **Separation of editors prevents regressions.** The Equation Editor is
   untouched by graph editing work.
6. **Server is domain-agnostic.** `server.rs` contains zero domain-specific
   code. All domain semantics live in `.kleis` theory files, interpreted by Z3.
7. **Closed-world axioms prevent Z3 exploitation.** The preamble constrains all
   uninterpreted functions so Z3 cannot invent values for unconstrained inputs.
8. **Theory declarations bridge preamble and theory.** Theories explicitly
   declare their required type codes as `operation TYPE_X : ℤ`. The preamble
   extracts these from the parsed AST and assigns concrete values. Types
   referenced by the theory but absent from the graph get unused codes,
   preventing Z3 from equating undefined TYPE constants with existing ones.
   The contract is explicit and parser-checked.

### Negative

1. **Two editors to maintain.** The Graph Editor and Equation Editor share no
   rendering code. Future changes to common patterns (e.g., template loading)
   must be applied in both places.
2. ~~**No bounded model checking.**~~ **Partially resolved by Phase 8.** The
   domain-agnostic simulation architecture enables step-by-step state space
   exploration via theory-defined `sim_enabled`/`sim_fire` functions. Full
   reachability analysis (e.g., all reachable markings of a Petri net) would
   require a generic BFS framework as a future extension.
3. **No persistent trunk waypoints for multi-port nets.** Trunk routing is
   recomputed each time; users cannot manually adjust trunk segments of
   multi-port nets (only 2-port nets have persistent draggable waypoints).

### Neutral

1. **The `__domain_` naming convention avoids parser changes** but is less clean
   than a dedicated `@graph_domain` syntax. A future parser enhancement could
   recognize a first-class `@graph_domain` block.
2. **Pan/zoom uses SVG `viewBox`**, which is performant and well-supported but
   limits future layering (e.g., HTML overlays would not pan with the SVG).
3. ~~Theory scanning is string-based.~~ **Resolved.** Type discovery now uses
   AST extraction via `parse_kleis_program()`. Theories explicitly declare
   `operation TYPE_X : ℤ`; the preamble reads parsed `OperationDecl` items.
   Comments and strings are no longer a concern.

## Files

| File | Role |
|------|------|
| `static/graph_editor.html` | Graph Editor HTML structure |
| `static/js/graphEditorMain.js` | Core editor logic: interaction, routing, rendering, verification |
| `src/bin/server.rs` | `/api/verify_graph` + `/api/simulate_graph` endpoints, domain-agnostic preamble generators |
| **Template files (`.kleist`)** | |
| `std_template_lib/electronics.kleist` | Electronics templates + `__domain_electronics` config |
| `std_template_lib/bond_graph.kleist` | Bond graph templates + `__domain_bond_graph` config |
| `std_template_lib/petri_net.kleist` | Petri net templates + `__domain_petri_net` config |
| `std_template_lib/graph_theory.kleist` | Abstract graph theory templates + `__domain_graph_theory` config |
| **Companion theories (`.kleis`)** | |
| `std_template_lib/electronics.kleis` | Electronics verification (KVL/KCL structural checks) |
| `std_template_lib/bond_graph.kleis` | Bond graph verification (SCAP structural checks) |
| `std_template_lib/petri_net.kleis` | Petri net verification + simulation (bipartiteness, marking, sim_enabled/fire/halt) |
| `std_template_lib/graph_theory.kleis` | Abstract graph theory (degree parity, Eulerian path) |
| **SVG assets** | |
| `static/svg/electronics/` | SVG assets for electronic components |
| `static/svg/bond_graph/` | SVG assets for bond graph elements |
| `static/svg/petri_net/` | SVG assets for Petri net elements |

## References

- ADR-022: Z3 Integration for Axiom Verification — the verification backend
  used by companion theories
- ADR-036: Multi-Domain Template Generality — established the Level 1/2/3
  classification; this ADR implements Level 3
- ADR-035: Multi-Domain Template Compiler — engine fixes for data-driven
  extensibility
- ADR-023: Template Externalization — `.kleist` file format and metadata model
- KiCad schematic architecture — studied for component/netlist model
- draw.io/diagrams.net — studied for interactive connector patterns
