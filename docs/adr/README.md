# Architecture Decision Records (ADRs)

This directory contains all Architecture Decision Records for the Kleis project.

---

## Core Language Design

### ADR-001: Scalar Multiply
**File:** `adr-001-scalar-multiply.md`  
**Status:** Accepted  
**Summary:** Scalar multiplication semantics and operator precedence

### ADR-002: Eval vs Simplify
**File:** `adr-002-eval-vs-simplify.md`  
**Status:** Accepted  
**Summary:** Symbolic evaluation strategy (simplify, not evaluate to numbers)

### ADR-003: Self-Hosting Strategy
**File:** `adr-003-self-hosting.md`  
**Status:** In Progress  
**Summary:** Path to defining Kleis in Kleis itself  
**Current:** Level 1 complete (data types), Level 2 partial (simple functions)

---

## Type System

### ADR-014: Hindley-Milner Type System
**File:** `adr-014-hindley-milner-type-system.md`  
**Status:** Accepted & Implemented  
**Summary:** Constraint-based type inference with unification

### ADR-015: Text as Source of Truth
**File:** `adr-015-text-as-source-of-truth.md`  
**Status:** Accepted  
**Summary:** Kleis source files are canonical, not internal representations  
**Validation:** See `adr-015-validation-report.md`

### ADR-016: Operations in Structures
**File:** `adr-016-operations-in-structures.md`  
**Status:** Accepted & Implemented  
**Summary:** Type operations defined in structures, not hardcoded in type checker

### ADR-019: Dimensional Type Checking
**File:** `adr-019-dimensional-type-checking.md`  
**Status:** Proposed  
**Summary:** Physical dimensions and units as part of type system

### ADR-020: Metalanguage for Type Theory
**File:** `adr-020-metalanguage-for-type-theory.md`  
**Status:** Accepted  
**Summary:** Type-level vs value-level distinction

### ADR-021: Algebraic Data Types
**File:** `adr-021-algebraic-data-types.md`  
**Status:** Accepted & Implemented  
**Summary:** User-defined data types with pattern matching

### ADR-022: Z3 Integration for Axiom Verification
**File:** `adr-022-z3-integration-for-axiom-verification.md`  
**Status:** Accepted & Implemented  
**Summary:** Using Z3 SMT solver for verifying axioms in structures

### ADR-028: Dimension Expressions
**File:** `ADR-028-Dimension-Expressions.md`  
**Status:** Accepted  
**Summary:** Type-level dimension expressions (e.g., `Matrix(2*n, 2*n)`) for expressing relationships between matrix dimensions

---

## User Interface & Editing

### ADR-004: Input Visualization
**File:** `adr-004-input-visualization.md`  
**Status:** Accepted  
**Summary:** Visual feedback during mathematical input

### ADR-005: Visual Authoring
**File:** `adr-005-visual-authoring.md`  
**Status:** Proposed  
**Summary:** Visual tools for mathematical authoring

### ADR-009: WYSIWYG Structural Editor
**File:** `adr-009-wysiwyg-structural-editor.md`  
**Status:** Accepted  
**Summary:** Direct manipulation of mathematical structures

### ADR-010: Inline Editing
**File:** `adr-010-inline-editing.md`  
**Status:** Accepted  
**Summary:** Edit mathematics inline with rendered output

### ADR-011: Notebook Environment
**File:** `adr-011-notebook-environment.md`  
**Status:** Accepted  
**Summary:** Interactive computational notebook interface

### ADR-012: Document Authoring
**File:** `adr-012-document-authoring.md`  
**Status:** Implemented (Jan 2026)  
**Summary:** Documents as Kleis programs with templates in `stdlib/templates/`

### ADR-017: Vite + PatternFly Frontend
**File:** `adr-017-vite-patternfly-frontend.md`  
**Status:** Accepted  
**Summary:** Modern web frontend technology choices

### ADR-023: Template Externalization with .kleist Files
**File:** `ADR-023-kleist-template-externalization.md`  
**Status:** Accepted  
**Summary:** Equation editor templates externalized to `.kleist` files for extensibility

### ADR-024: Kleis Notebook Monaco Editor
**File:** `adr-024-kleis-notebook-monaco-editor.md`  
**Status:** Accepted  
**Summary:** Using Monaco editor in Jupyter notebook for Kleis editing

### ADR-025: Debugger Shared Context
**File:** `adr-025-debugger-shared-context.md`  
**Status:** Accepted & Implemented  
**Summary:** Sharing evaluation context between REPL and debugger

---

## Grammar & Parsing

### ADR-006: Template-Grammar Duality
**File:** `adr-006-template-grammar-duality.md`  
**Status:** Accepted  
**Summary:** Templates and grammar as dual representations

### ADR-007: Bootstrap Grammar
**File:** `adr-007-bootstrap-grammar.md`  
**Status:** Accepted  
**Summary:** Minimal grammar for bootstrapping the system

### ADR-008: Bootstrap Grammar Boundary
**File:** `adr-008-bootstrap-grammar-boundary.md`  
**Status:** Accepted  
**Summary:** Defining what's in vs out of bootstrap grammar

### ADR-027: Named Arguments as Parser-Level Sugar
**File:** `ADR-027-Named-Arguments-Parser-Sugar.md`  
**Status:** Implemented  
**Summary:** Named/keyword arguments for plotting functions without complicating the type system

---

## Formalism & Theory

### ADR-013: Paper Scope Hierarchy
**File:** `adr-013-paper-scope-hierarchy.md`  
**Status:** Accepted  
**Summary:** Scoping rules for mathematical papers and documents

### ADR-018: Universal Formalism
**File:** `adr-018-universal-formalism.md`  
**Status:** Proposed  
**Summary:** Unified formalism for mathematics across domains

### ADR-026: Self-Hosted Differential Forms
**File:** `adr-026-self-hosted-differential-forms.md`  
**Status:** Proposed  
**Summary:** Defining differential forms and exterior algebra in Kleis

### ADR-029: LAPACK Integration
**File:** `ADR-029-LAPACK-Integration.md`  
**Status:** Accepted & Implemented  
**Summary:** LAPACK-level linear algebra via ndarray-linalg with platform-specific backends (Apple Accelerate on macOS, OpenBLAS on Linux/Windows)

---

## Agent & MCP Architecture

### ADR-030: MCP Agent Reasoning Partner
**File:** `ADR-030-MCP-Agent-Reasoning-Partner.md`  
**Status:** Accepted  
**Summary:** Policy engine as an MCP-based reasoning partner for AI agents

### ADR-031: Theory MCP Interactive Theory Building
**File:** `ADR-031-Theory-MCP-Interactive-Theory-Building.md`  
**Status:** Accepted  
**Summary:** Interactive theory building via MCP tools

### ADR-032: Intent-Aware Code Review
**File:** `ADR-032-Intent-Aware-Code-Review.md`  
**Status:** Accepted  
**Summary:** MCP-based code review that understands architectural intent

---

## Multi-Domain Extensibility

### ADR-033: Musical Score Notation
**File:** `ADR-033-Musical-Score-Notation.md`  
**Status:** Accepted  
**Summary:** Music notation as a non-mathematical domain extension

### ADR-034: Egyptian Hieroglyph Editor
**File:** `ADR-034-Egyptian-Hieroglyph-Editor.md`  
**Status:** Accepted  
**Summary:** Egyptian hieroglyphs as a test of the multi-domain template architecture

### ADR-035: Multi-Domain Template Compiler
**File:** `ADR-035-Multi-Domain-Template-Compiler.md`  
**Status:** Proposed  
**Summary:** Engine fixes to make domain extension purely data-driven (no Rust/JS — only `.kleist` templates and assets)

### ADR-036: Multi-Domain Template Generality
**File:** `ADR-036-Multi-Domain-Template-Generality.md`  
**Status:** Accepted  
**Summary:** Validation that the atom-molecule template decomposition generalizes across mathematics, hieroglyphs, circuits, and chemistry

### ADR-037: Graph Editor with Domain-Agnostic Routing and Verification
**File:** `ADR-037-Graph-Editor-Domain-Agnostic-Routing.md`  
**Status:** Accepted & Implemented (Phases 1–5)  
**Summary:** Standalone graph editor with incidence-matrix AST representation, domain-configurable routing (trunk+branch, star), parameterized components, and domain-agnostic Z3 verification via companion `.kleis` theory files. Extensible for electronics, bond graphs, Petri nets, and molecular graphs via `.kleist`/`.kleis` data files only — `server.rs` contains zero domain-specific code.

---

## Implementation Notes

### ADR-015 Validation Report
**File:** `adr-015-validation-report.md`  
**Type:** Validation Document  
**Summary:** Evidence that text-as-source-of-truth works in practice

---

## Status Legend

- **Proposed:** Under consideration
- **Accepted:** Decision made, may not be implemented yet
- **Implemented:** Fully implemented in codebase
- **Accepted & Implemented:** Both decided and built
- **In Progress:** Partially implemented
- **Superseded:** Replaced by newer ADR

---

## Naming Convention

ADRs follow the pattern: `adr-NNN-short-title.md`

- Numbers are sequential
- Titles use kebab-case
- Some historical ADRs use different capitalization (ADR-016)

---

## Adding New ADRs

When creating a new ADR:

1. Use next available number
2. Follow template structure (problem, decision, consequences)
3. Add entry to this README
4. Update status as implementation progresses

---

**Total ADRs:** 37  
**Last Updated:** May 14, 2026

