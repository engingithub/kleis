# ADR-037: Graph Editor with Domain-Agnostic Routing and Verification

**Status:** Accepted & Implemented (Phases 1–5)
**Date:** 2026-04-27 (original), 2026-05-14 (updated)
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

// Type code constants (one per unique component_type string)
operation TYPE_X : ℤ            // e.g., TYPE_Place, TYPE_Transition
```

The preamble emits a `GraphData` structure with:

- **Component counts** and **type code assignments**
- **Parameter values** (positional, from the component's `params` map)
- **Component-level incidence** (port-level entries aggregated per component)
- **Closed-world axioms** for `graph_ctype`, `graph_inc`, and `graph_param`
  (preventing Z3 from inventing values for unconstrained inputs)
- **Distinctness axioms** for all TYPE codes (preventing Z3 from equating
  different component roles)

**Theory scanning for absent types:** The preamble scans the companion theory
text for `TYPE_X` references and assigns unused codes to any types the theory
needs but the graph doesn't contain. This ensures every TYPE constant gets a
concrete value. For absent types, the assigned code won't match any actual
component (codes start at 1 for present types; absent types get higher codes
that no component has). This is fully domain-agnostic — it's string scanning,
not domain interpretation.

### 8. Companion Theory Files

Each domain's verification logic lives entirely in a companion `.kleis` file.
The theory interprets the generic graph primitives in domain-specific terms.

Example — `std_template_lib/petri_net.kleis`:

```kleis
// Domain interpretation of generic graph data
define is_place(c) =
    graph_ctype(c) = TYPE_Place ∨
    graph_ctype(c) = TYPE_SourcePlace ∨
    graph_ctype(c) = TYPE_SinkPlace

define is_transition(c) = graph_ctype(c) = TYPE_Transition
define tokens(c) = graph_param(c, 0)

// Structural verification via Z3
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

**Adding verification for a new domain** requires only writing a companion
`.kleis` file that maps generic primitives to domain semantics. No Rust changes.

### 9. Future Domain Support Without Code Changes

New domains require only:

1. A `.kleist` file with component templates (SVGs, ports, metadata, params)
2. A `@template __domain_<name>` block declaring routing/verify preferences
3. SVG assets in `static/svg/<domain>/`
4. *(Optional)* A companion `.kleis` theory for Z3 verification

No JavaScript, no Rust, no recompilation.

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
- **Theory scanning**: preamble scans theory text for `TYPE_X` references to
  assign codes to absent types
- **Companion theory**: `petri_net.kleis` derives domain semantics entirely from
  generic primitives
- **23 tests** covering preamble structure, Z3 verification (pass and fail
  cases), missing theories, and multi-domain scenarios

### Phase 6 (Planned): Extended Verification

- Token visualization in Petri net places
- Arc weight support (weighted edges in incidence matrix)
- Companion theories for electronics and bond graph domains
- Simulation integration with existing ODE solver

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
8. **Theory scanning bridges preamble and theory.** Types referenced by the
   theory but absent from the graph get assigned concrete unused codes, preventing
   Z3 from equating undefined TYPE constants with existing ones.

### Negative

1. **Two editors to maintain.** The Graph Editor and Equation Editor share no
   rendering code. Future changes to common patterns (e.g., template loading)
   must be applied in both places.
2. **No bounded model checking.** The domain-agnostic architecture precludes
   server-side state space exploration (e.g., Petri net reachability analysis).
   Verification is limited to what Z3 can prove from the incidence matrix and
   component parameters directly. Future work could add a generic BFS framework
   as an opt-in extension.
3. **No persistent trunk waypoints for multi-port nets.** Trunk routing is
   recomputed each time; users cannot manually adjust trunk segments of
   multi-port nets (only 2-port nets have persistent draggable waypoints).

### Neutral

1. **The `__domain_` naming convention avoids parser changes** but is less clean
   than a dedicated `@graph_domain` syntax. A future parser enhancement could
   recognize a first-class `@graph_domain` block.
2. **Pan/zoom uses SVG `viewBox`**, which is performant and well-supported but
   limits future layering (e.g., HTML overlays would not pan with the SVG).
3. **Theory scanning is string-based.** It finds `TYPE_X` patterns via simple
   text splitting, not AST analysis. This is fragile if `TYPE_` appears in
   comments or strings, but sufficient for the current use case.

## Files

| File | Role |
|------|------|
| `static/graph_editor.html` | Graph Editor HTML structure |
| `static/js/graphEditorMain.js` | Core editor logic: interaction, routing, rendering, verification |
| `src/bin/server.rs` | `/api/verify_graph` endpoint, domain-agnostic preamble generator |
| `std_template_lib/electronics.kleist` | Electronics templates + `__domain_electronics` config |
| `std_template_lib/bond_graph.kleist` | Bond graph templates + `__domain_bond_graph` config |
| `std_template_lib/petri_net.kleist` | Petri net templates + `__domain_petri_net` config |
| `std_template_lib/petri_net.kleis` | Petri net companion theory (Z3 verification) |
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
