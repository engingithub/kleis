//! Symbolic Evaluator for Kleis (Wire 3: Self-Hosting)
//!
//! This module provides symbolic evaluation of Kleis expressions, including
//! user-defined functions (`define` statements).
//!
//! **Key Concept**: Kleis is a symbolic math system, not a computational interpreter.
//! "Evaluation" means symbolic manipulation: substituting variables and simplifying expressions.
//!
//! ## Capabilities
//!
//! 1. **Function Storage**: Store function definitions as closures
//! 2. **Function Application**: Apply functions via symbolic substitution
//! 3. **Pattern Matching**: Delegate to PatternMatcher for match expressions
//!
//! ## Examples
//!
//! ```ignore
//! let mut eval = Evaluator::new();
//!
//! // Load function: define double(x) = x + x
//! eval.load_function("double", vec!["x"],
//!     Expression::Operation {
//!         name: "plus",
//!         args: vec![Object("x"), Object("x")]
//!     });
//!
//! // Apply: double(5)
//! let result = eval.apply_function("double", vec![Const("5")])?;
//! // result = plus(5, 5) (symbolic, not computed to 10)
//! ```
use crate::ast::Expression;
use crate::debug::{DebugHook, SourceLocation};
use crate::kleis_ast::{FunctionDef, Program, TopLevel};
use crate::kleis_parser::SourceSpan;
use crate::pattern_matcher::PatternMatcher;
use crate::solvers::backend::Witness;
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;

/// Result of evaluating an example block
#[derive(Debug, Clone)]
pub struct ExampleResult {
    /// Name of the example
    pub name: String,
    /// Whether the example passed (all assertions succeeded)
    pub passed: bool,
    /// Error message if failed
    pub error: Option<String>,
    /// Number of assertions that passed
    pub assertions_passed: usize,
    /// Total number of assertions
    pub assertions_total: usize,
    /// Witness from the last Z3-verified assertion (for ∃ queries)
    pub witness: Option<Witness>,
}

/// Result of a single assertion
#[derive(Debug, Clone)]
pub enum AssertResult {
    /// Assertion passed (concrete equality)
    Passed,
    /// Assertion verified by Z3 (symbolic proof).
    /// For existential quantifiers, `witness` contains a satisfying assignment.
    Verified { witness: Option<Witness> },
    /// Assertion failed with concrete values (boxed to reduce enum size)
    Failed {
        expected: Box<Expression>,
        actual: Box<Expression>,
    },
    /// Assertion disproved by Z3 — the `witness` contains variable assignments
    /// (as Kleis expressions) that violate the property.
    Disproved { witness: Witness },
    /// Assertion couldn't be evaluated (symbolic)
    Unknown(String),
    /// Loaded axioms are mutually inconsistent — any assertion would be vacuously true
    InconsistentAxioms,
}

/// Represents a user-defined function as a closure
#[derive(Debug, Clone)]
pub struct Closure {
    /// Parameter names
    pub params: Vec<String>,

    /// Function body (expression to evaluate)
    pub body: Expression,

    /// Captured environment (for closures - not used yet in Wire 3)
    pub env: HashMap<String, Expression>,

    /// Source location where this function is defined
    pub span: Option<SourceSpan>,

    /// File path where this function is defined (for cross-file debugging)
    pub file: Option<PathBuf>,
}

/// Symbolic evaluator for Kleis expressions
pub struct Evaluator {
    /// Loaded function definitions
    functions: HashMap<String, Closure>,

    /// Variable bindings from :let command
    /// Maps variable names to their evaluated values
    bindings: HashMap<String, Expression>,

    /// Last evaluation result (for `it` magic variable)
    /// Stores the result of the most recent :eval command
    last_result: Option<Expression>,

    /// Pattern matcher for match expressions
    matcher: PatternMatcher,

    /// ADT constructor names (nullary constructors like TCP, UDP, ICMP)
    /// These are values that should be recognized as constants, not variables
    adt_constructors: std::collections::HashSet<String>,

    /// All ADT constructor names (including those with fields like Some, Cons, Atom)
    /// Used by eval_concrete to recognize constructor calls as values
    all_constructors: std::collections::HashSet<String>,

    /// Loaded data type definitions (for export)
    data_types: Vec<crate::kleis_ast::DataDef>,

    /// Loaded structure definitions (for export)
    structures: Vec<crate::kleis_ast::StructureDef>,

    /// Top-level operation declarations (for Z3 type signatures)
    toplevel_operations: HashMap<String, crate::kleis_ast::TypeExpr>,

    /// Implements blocks (for registry - where constraints and concrete bindings)
    implements_blocks: Vec<crate::kleis_ast::ImplementsDef>,

    /// Type aliases (for registry - type abbreviations)
    type_aliases: Vec<crate::kleis_ast::TypeAlias>,

    /// Optional debug hook for step-through debugging
    /// When set, eval() calls hook methods at key points
    /// Uses RefCell for interior mutability (hook needs &mut self)
    debug_hook: RefCell<Option<Box<dyn DebugHook + Send>>>,

    /// Cached axiom consistency result (avoids re-checking per assertion).
    /// None = not yet checked, Some(true) = consistent, Some(false) = inconsistent.
    /// "Unknown" from Z3 is stored as None (re-check not attempted — too expensive).
    axiom_consistency_cache: RefCell<Option<Option<bool>>>,
}

/// Evaluate an expression to a numeric value (no evaluator state needed).
///
/// Handles: constants, arithmetic (+, -, *, /), power, trig (sin, cos),
/// exp, sqrt, negation. Returns None for symbolic/unevaluable expressions.
///
/// This is a pure function - it doesn't need Evaluator state, so it can be
/// called from closures (like the ODE solver dynamics function).
pub fn eval_numeric(expr: &Expression) -> Option<f64> {
    match expr {
        Expression::Const(s) => s.parse().ok(),
        Expression::Operation { name, args, .. } => {
            let vals: Option<Vec<f64>> = args.iter().map(eval_numeric).collect();
            let vals = vals?;
            match name.as_str() {
                "plus" | "add" => Some(vals.iter().sum()),
                "minus" | "sub" => match vals.len() {
                    1 => Some(-vals[0]),
                    2 => Some(vals[0] - vals[1]),
                    _ => None,
                },
                "times" | "mul" => Some(vals.iter().product()),
                "div" | "divide" => {
                    if vals.len() == 2 && vals[1] != 0.0 {
                        Some(vals[0] / vals[1])
                    } else {
                        None
                    }
                }
                "pow" | "power" => vals.first().zip(vals.get(1)).map(|(a, b)| a.powf(*b)),
                "sin" => vals.first().map(|v| v.sin()),
                "cos" => vals.first().map(|v| v.cos()),
                "tan" => vals.first().map(|v| v.tan()),
                "exp" => vals.first().map(|v| v.exp()),
                "ln" | "log" => vals.first().map(|v| v.ln()),
                "sqrt" => vals.first().map(|v| v.sqrt()),
                "abs" => vals.first().map(|v| v.abs()),
                "neg" | "negate" => vals.first().map(|v| -v),
                _ => None,
            }
        }
        _ => None,
    }
}

impl Evaluator {
    /// Create a new evaluator
    pub fn new() -> Self {
        Evaluator {
            functions: HashMap::new(),
            bindings: HashMap::new(),
            last_result: None,
            matcher: PatternMatcher,
            adt_constructors: std::collections::HashSet::new(),
            all_constructors: std::collections::HashSet::new(),
            data_types: Vec::new(),
            structures: Vec::new(),
            toplevel_operations: HashMap::new(),
            implements_blocks: Vec::new(),
            type_aliases: Vec::new(),
            debug_hook: RefCell::new(None),
            axiom_consistency_cache: RefCell::new(None),
        }
    }

    /// Basic unescape for common sequences (\n, \t, \", \\)
    /// Note: since session 15, the parser unescapes at parse time, so this is
    /// rarely needed. Kept for edge cases (external string sources).
    #[allow(dead_code)]
    fn unescape_basic(&self, s: &str) -> String {
        let mut out = String::with_capacity(s.len());
        let mut chars = s.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '\\' {
                match chars.next() {
                    Some('n') => out.push('\n'),
                    Some('t') => out.push('\t'),
                    Some('\\') => out.push('\\'),
                    Some('"') => out.push('"'),
                    Some(other) => {
                        out.push('\\');
                        out.push(other);
                    }
                    None => out.push('\\'),
                }
            } else {
                out.push(c);
            }
        }
        out
    }

    /// Set a variable binding (used by :let command)
    pub fn set_binding(&mut self, name: String, value: Expression) {
        self.bindings.insert(name, value);
    }

    /// Get a variable binding
    pub fn get_binding(&self, name: &str) -> Option<&Expression> {
        self.bindings.get(name)
    }

    /// Get all variable bindings (for debugger variable inspection)
    pub fn get_all_bindings(&self) -> impl Iterator<Item = (&String, &Expression)> {
        self.bindings.iter()
    }

    /// Set the last evaluation result (for `it` magic variable)
    pub fn set_last_result(&mut self, value: Expression) {
        self.last_result = Some(value);
    }

    /// Get the last evaluation result
    pub fn get_last_result(&self) -> Option<&Expression> {
        self.last_result.as_ref()
    }

    /// List all bindings (for :env command)
    pub fn list_bindings(&self) -> Vec<(&String, &Expression)> {
        self.bindings.iter().collect()
    }

    /// Load function definitions from a parsed program (Wire 3: Self-hosting)
    ///
    /// Processes `define` statements and stores them as closures.
    ///
    /// Example:
    /// ```ignore
    /// let code = "define double(x) = x + x";
    /// let program = parse_kleis_program(code)?;
    /// evaluator.load_program(&program)?;
    /// // Now 'double' is available for application
    /// ```
    pub fn load_program(&mut self, program: &Program) -> Result<(), String> {
        self.load_program_with_file(program, None)
    }

    /// Load a program with file path for cross-file debugging
    ///
    /// This is the preferred method when loading files, as it enables
    /// the debugger to track source locations across file boundaries.
    pub fn load_program_with_file(
        &mut self,
        program: &Program,
        file: Option<PathBuf>,
    ) -> Result<(), String> {
        for item in &program.items {
            if let TopLevel::FunctionDef(func_def) = item {
                self.load_function_def_with_file(func_def, file.clone())?;
            }
        }

        // Extract ADT constructor names and store data definitions
        for data_type in program.data_types() {
            // Store the full data definition for export
            self.data_types.push(data_type.clone());

            for variant in &data_type.variants {
                // Track ALL constructors for eval_concrete
                self.all_constructors.insert(variant.name.clone());

                // Nullary constructors (no fields) are values/constants
                if variant.fields.is_empty() {
                    self.adt_constructors.insert(variant.name.clone());
                }
            }
        }

        // Store structure definitions for export
        for structure in program.structures() {
            self.structures.push(structure.clone());
        }

        // Store implements blocks for registry
        for item in &program.items {
            if let TopLevel::ImplementsDef(impl_def) = item {
                self.implements_blocks.push(impl_def.clone());
            }
        }

        // Store type aliases for registry
        for item in &program.items {
            if let TopLevel::TypeAlias(type_alias) = item {
                self.type_aliases.push(type_alias.clone());
            }
        }

        // Store top-level operation declarations for Z3 type lookup
        for item in &program.items {
            if let TopLevel::OperationDecl(op_decl) = item {
                self.toplevel_operations
                    .insert(op_decl.name.clone(), op_decl.type_signature.clone());
            }
        }

        Ok(())
    }

    /// Get the set of ADT constructor names (nullary constructors)
    pub fn get_adt_constructors(&self) -> &std::collections::HashSet<String> {
        &self.adt_constructors
    }

    /// Get all loaded data type definitions
    pub fn get_data_types(&self) -> &[crate::kleis_ast::DataDef] {
        &self.data_types
    }

    /// Get all loaded structure definitions
    pub fn get_structures(&self) -> &[crate::kleis_ast::StructureDef] {
        &self.structures
    }

    /// Load function definitions from structure members (Grammar v0.6)
    ///
    /// Processes `define` statements inside structures and makes them available
    /// for symbolic expansion.
    ///
    /// Example:
    /// ```ignore
    /// structure Ring(R) {
    ///   operation (-) : R × R → R
    ///   define (-)(x, y) = x + negate(y)
    /// }
    /// // Now (-) is available for expansion: a - b → a + negate(b)
    /// ```
    pub fn load_structure_functions(
        &mut self,
        structure: &crate::kleis_ast::StructureDef,
    ) -> Result<(), String> {
        self.load_structure_functions_recursive(&structure.members)
    }

    /// Recursively load functions from structure members
    fn load_structure_functions_recursive(
        &mut self,
        members: &[crate::kleis_ast::StructureMember],
    ) -> Result<(), String> {
        use crate::kleis_ast::StructureMember;

        for member in members {
            match member {
                StructureMember::FunctionDef(func_def) => {
                    // Load function for symbolic expansion
                    self.load_function_def(func_def)?;
                }
                StructureMember::NestedStructure { members, .. } => {
                    // Recursively load from nested structures
                    self.load_structure_functions_recursive(members)?;
                }
                _ => {
                    // Operation, Field, Axiom - not functions
                }
            }
        }
        Ok(())
    }

    /// Load a single function definition
    pub fn load_function_def(&mut self, func_def: &FunctionDef) -> Result<(), String> {
        self.load_function_def_with_file(func_def, None)
    }

    /// Load a single function definition with file path for cross-file debugging
    pub fn load_function_def_with_file(
        &mut self,
        func_def: &FunctionDef,
        file: Option<PathBuf>,
    ) -> Result<(), String> {
        let closure = Closure {
            params: func_def.params.clone(),
            body: func_def.body.clone(),
            env: HashMap::new(), // Empty environment for now
            span: func_def.span.clone(),
            file,
        };

        self.functions.insert(func_def.name.clone(), closure);
        Ok(())
    }

    /// Get the full source location of a function (line, column, file)
    pub fn get_function_location(&self, name: &str) -> Option<SourceLocation> {
        self.functions.get(name).and_then(|c| {
            c.span.clone().map(|span| {
                // Use SourceLocation::from_span which extracts file from span.file
                let mut loc = SourceLocation::from_span(&span);
                // Fall back to Closure.file if span.file is None (legacy support)
                if loc.file.is_none()
                    && let Some(ref file) = c.file
                {
                    loc = loc.with_file(file.clone());
                }
                loc
            })
        })
    }

    /// Get the source location of a function
    pub fn get_function_span(&self, name: &str) -> Option<SourceSpan> {
        self.functions.get(name).and_then(|c| c.span.clone())
    }

    /// Apply a user-defined function to arguments (symbolic substitution)
    ///
    /// This performs symbolic substitution: replaces parameters with arguments in the body.
    ///
    /// Example:
    /// ```ignore
    /// // Given: define double(x) = x + x
    /// // Call: double(5)
    /// // Result: 5 + 5 (symbolic)
    /// ```
    pub fn apply_function(&self, name: &str, args: Vec<Expression>) -> Result<Expression, String> {
        self.apply_function_internal(name, args, 0)
    }

    /// Internal apply_function with depth tracking for debugging
    fn apply_function_internal(
        &self,
        name: &str,
        args: Vec<Expression>,
        depth: usize,
    ) -> Result<Expression, String> {
        let closure = self
            .functions
            .get(name)
            .ok_or_else(|| format!("Function '{}' not defined", name))?;

        if args.len() != closure.params.len() {
            return Err(format!(
                "Function '{}' expects {} arguments, got {}",
                name,
                closure.params.len(),
                args.len()
            ));
        }

        // Build substitution map: param_name -> argument_value
        let mut subst = HashMap::new();
        for (param, arg) in closure.params.iter().zip(args.iter()) {
            subst.insert(param.clone(), arg.clone());
        }

        // Notify debug hook about parameter bindings
        {
            let mut hook_ref = self.debug_hook.borrow_mut();
            if let Some(ref mut hook) = *hook_ref {
                for (param, arg) in &subst {
                    hook.on_bind(param, arg, depth);
                }
            }
        }

        // Substitute parameters in body
        Ok(self.substitute(&closure.body, &subst))
    }

    /// Evaluate an expression (symbolic evaluation)
    ///
    /// This resolves function applications and match expressions symbolically.
    /// It does NOT perform arithmetic computation.
    ///
    /// If a debug hook is set (via `set_debug_hook`), this method will call
    /// hook methods at key evaluation points, enabling step-through debugging.
    pub fn eval(&self, expr: &Expression) -> Result<Expression, String> {
        self.eval_internal(expr, 0)
    }

    /// Internal evaluation with depth tracking for debugging
    fn eval_internal(&self, expr: &Expression, depth: usize) -> Result<Expression, String> {
        // Call debug hook ONLY if expression has a valid span
        // Leaf expressions (Const, Object) without spans should NOT trigger stops
        if let Some(span) = expr.get_span() {
            let location = SourceLocation::from_span(span);
            let mut hook_ref = self.debug_hook.borrow_mut();
            if let Some(ref mut hook) = *hook_ref {
                let _action = hook.on_eval_start(expr, &location, depth);
                // Hook handles pausing internally if needed
            }
        }

        // Evaluate based on expression type
        let result = match expr {
            // Check if this is a function application
            Expression::Operation { name, args, span } => {
                if self.functions.contains_key(name) {
                    // Get the function's full source location (span + file) for debugging
                    // Fallback to expression span if function location not found
                    let expr_location = span
                        .as_ref()
                        .map(SourceLocation::from_span)
                        .unwrap_or_else(|| SourceLocation::new(1, 1));
                    let func_location = self.get_function_location(name).unwrap_or(expr_location);

                    crate::logging::log(
                        "DEBUG",
                        "eval",
                        &format!(
                            "Entering function '{}' with location: line={}, file={:?}",
                            name, func_location.line, func_location.file
                        ),
                    );

                    // Call debug hook for function entry with correct location
                    {
                        let mut hook_ref = self.debug_hook.borrow_mut();
                        if let Some(ref mut hook) = *hook_ref {
                            hook.on_function_enter(name, args, &func_location, depth);
                        }
                    }

                    // It's a user-defined function - apply it
                    let eval_args: Result<Vec<_>, _> = args
                        .iter()
                        .map(|arg| self.eval_internal(arg, depth + 1))
                        .collect();
                    let eval_args = eval_args?;

                    let func_result = self.apply_function_internal(name, eval_args, depth + 1);

                    // Report the result's location (span has line/column/file from source)
                    if let Ok(ref result_expr) = func_result
                        && let Some(span) = result_expr.get_span()
                    {
                        let result_location = SourceLocation::from_span(span);
                        let mut hook_ref = self.debug_hook.borrow_mut();
                        if let Some(ref mut hook) = *hook_ref {
                            let _action = hook.on_eval_start(result_expr, &result_location, depth);
                        }
                    }

                    // Call debug hook for function exit
                    // Note: on_function_exit already calls pop_frame internally
                    {
                        let mut hook_ref = self.debug_hook.borrow_mut();
                        if let Some(ref mut hook) = *hook_ref {
                            hook.on_function_exit(name, &func_result, depth);
                        }
                    }

                    func_result
                } else if name == "eval" || name == "reduce" {
                    // Special built-in: evaluate ground terms via Z3 simplify
                    // eval(expr) → concrete value if expr has no free variables
                    if args.len() != 1 {
                        return Err("eval() takes exactly 1 argument".to_string());
                    }

                    // First evaluate the argument
                    let arg_eval = self.eval_internal(&args[0], depth + 1)?;

                    // Check if the expression is symbolic (has free variables)
                    if self.is_symbolic(&arg_eval) {
                        return Err("eval() cannot evaluate expressions with free variables. \
                             Use assert() with Z3 for symbolic verification instead."
                            .to_string());
                    }

                    // Use Z3 simplify to reduce the ground term
                    self.eval_ground_term(&arg_eval)
                } else {
                    // Built-in operation - just evaluate arguments
                    let eval_args: Result<Vec<_>, _> = args
                        .iter()
                        .map(|arg| self.eval_internal(arg, depth + 1))
                        .collect();
                    let eval_args = eval_args?;

                    Ok(Expression::Operation {
                        name: name.clone(),
                        args: eval_args,
                        span: span.clone(),
                    })
                }
            }

            // Match expressions - delegate to PatternMatcher
            Expression::Match {
                scrutinee, cases, ..
            } => {
                let eval_scrutinee = self.eval_internal(scrutinee, depth + 1)?;
                let result = self.matcher.eval_match(&eval_scrutinee, cases)?;
                self.eval_internal(&result, depth + 1)
            }

            // Lists - evaluate elements
            Expression::List(elements) => {
                let eval_elements: Result<Vec<_>, _> = elements
                    .iter()
                    .map(|elem| self.eval_internal(elem, depth + 1))
                    .collect();
                Ok(Expression::List(eval_elements?))
            }

            // Quantifiers - not evaluated (used in axioms)
            Expression::Quantifier { .. } => {
                // Quantifiers are for axioms, not runtime evaluation
                Ok(expr.clone())
            }

            // Conditionals - evaluate condition and select branch
            Expression::Conditional {
                condition,
                then_branch,
                else_branch,
                span,
            } => {
                let eval_cond = self.eval_internal(condition, depth + 1)?;

                if let Some(cond_bool) = self.as_boolean(&eval_cond) {
                    if cond_bool {
                        return self.eval_internal(then_branch, depth + 1);
                    }
                    return self.eval_internal(else_branch, depth + 1);
                }

                Ok(Expression::Conditional {
                    condition: Box::new(eval_cond),
                    then_branch: then_branch.clone(),
                    else_branch: else_branch.clone(),
                    span: span.clone(),
                })
            }

            // Let bindings - evaluate value and substitute into body
            // Grammar v0.8: supports pattern destructuring
            Expression::Let {
                pattern,
                value,
                body,
                ..
            } => {
                // Evaluate the value
                let eval_value = self.eval_internal(value, depth + 1)?;

                // Match pattern against value and collect bindings
                let mut subst = std::collections::HashMap::new();
                self.match_pattern_to_bindings(pattern, &eval_value, &mut subst)?;

                // Call debug hook for each binding
                {
                    let mut hook_ref = self.debug_hook.borrow_mut();
                    if let Some(ref mut hook) = *hook_ref {
                        for (name, val) in &subst {
                            hook.on_bind(name, val, depth);
                        }
                    }
                }

                let substituted_body = self.substitute(body, &subst);
                self.eval_internal(&substituted_body, depth + 1)
            }

            // Type ascription - evaluate inner expression, discard type annotation
            // (type checking happens at type-check time, not evaluation time)
            Expression::Ascription { expr: inner, .. } => self.eval_internal(inner, depth),

            // Lambda - return as a value (closures are values)
            Expression::Lambda { .. } => Ok(expr.clone()),

            // Atoms - return as-is (except Object which may need binding lookup)
            Expression::Const(_) | Expression::String(_) | Expression::Placeholder { .. } => {
                Ok(expr.clone())
            }

            Expression::Object(name) => {
                // Check bindings first (from example blocks / let statements)
                if let Some(bound) = self.bindings.get(name) {
                    // Recursively evaluate the bound value
                    self.eval_internal(bound, depth + 1)
                } else {
                    // Not bound, return as symbolic object
                    Ok(expr.clone())
                }
            }
        };

        // Call debug hook after evaluation
        {
            let mut hook_ref = self.debug_hook.borrow_mut();
            if let Some(ref mut hook) = *hook_ref {
                hook.on_eval_end(expr, &result, depth);
            }
        }

        result
    }

    /// Get a function definition (for inspection/testing)
    pub fn get_function(&self, name: &str) -> Option<&Closure> {
        self.functions.get(name)
    }

    /// Check if a function is defined
    pub fn has_function(&self, name: &str) -> bool {
        self.functions.contains_key(name)
    }

    /// List all defined function names
    pub fn list_functions(&self) -> Vec<String> {
        self.functions.keys().cloned().collect()
    }

    /// Verify a proposition (public wrapper around eval_assert).
    ///
    /// Tries concrete evaluation first; falls back to Z3 for quantified
    /// or symbolic expressions. This is the same pipeline that `assert()`
    /// uses inside example blocks.
    ///
    /// Returns the `AssertResult` which may be:
    /// - `Passed` — concrete evaluation confirmed truth
    /// - `Verified` — Z3 proved the proposition
    /// - `Failed` — concrete evaluation returned false
    /// - `Disproved { witness }` — Z3 found a counterexample (Kleis expressions)
    /// - `Unknown(msg)` — could not determine
    pub fn verify_proposition(&self, condition: &Expression) -> AssertResult {
        self.eval_assert(condition)
    }

    /// Remove a function by name
    /// Returns true if the function was found and removed
    pub fn remove_function(&mut self, name: &str) -> bool {
        self.functions.remove(name).is_some()
    }

    /// Remove a data type by name, including all its constructors
    /// Returns true if the data type was found and removed
    pub fn remove_data_type(&mut self, name: &str) -> bool {
        // Find the data type and get its constructors
        let mut found_idx = None;
        let mut constructors_to_remove = Vec::new();

        for (idx, data_type) in self.data_types.iter().enumerate() {
            if data_type.name == name {
                found_idx = Some(idx);
                for variant in &data_type.variants {
                    constructors_to_remove.push(variant.name.clone());
                }
                break;
            }
        }

        if let Some(idx) = found_idx {
            // Remove the data type
            self.data_types.remove(idx);

            // Remove its constructors from both sets
            for ctor in constructors_to_remove {
                self.adt_constructors.remove(&ctor);
                self.all_constructors.remove(&ctor);
            }
            true
        } else {
            false
        }
    }

    /// Remove a structure by name
    /// Returns true if the structure was found and removed
    pub fn remove_structure(&mut self, name: &str) -> bool {
        let initial_len = self.structures.len();
        self.structures.retain(|s| s.name != name);
        self.structures.len() < initial_len
    }

    /// Clear all definitions (for :reset command)
    /// Removes all functions, data types, structures, and bindings
    pub fn reset(&mut self) {
        self.functions.clear();
        self.bindings.clear();
        self.last_result = None;
        self.adt_constructors.clear();
        self.all_constructors.clear();
        self.data_types.clear();
        self.structures.clear();
    }

    /// Get counts for status display
    pub fn definition_counts(&self) -> (usize, usize, usize, usize) {
        (
            self.functions.len(),
            self.data_types.len(),
            self.structures.len(),
            self.bindings.len(),
        )
    }

    /// Set a debug hook for step-through debugging
    /// When set, eval() will call hook methods at key evaluation points
    pub fn set_debug_hook(&self, hook: Box<dyn DebugHook + Send>) {
        *self.debug_hook.borrow_mut() = Some(hook);
    }

    /// Remove the debug hook (return to normal evaluation)
    pub fn clear_debug_hook(&self) {
        *self.debug_hook.borrow_mut() = None;
    }

    /// Check if debugging is active
    pub fn is_debugging(&self) -> bool {
        self.debug_hook.borrow().is_some()
    }

    /// Evaluate an expression to a concrete value
    ///
    /// Unlike `eval` which is symbolic, this actually computes:
    /// - Arithmetic: 2 + 3 → 5
    /// - String operations: concat("a", "b") → "ab"
    /// - Conditionals: if true then x else y → x
    /// - Recursion: fib(5) → 5
    pub fn eval_concrete(&self, expr: &Expression) -> Result<Expression, String> {
        match expr {
            // Constants and strings are already values
            Expression::Const(s) => Ok(Expression::Const(s.clone())),
            Expression::String(s) => Ok(Expression::String(s.clone())),

            // Variables: check bindings, `it`, functions, ADT constructors
            Expression::Object(name) => {
                // 1. Check REPL bindings (from :let command)
                if let Some(value) = self.bindings.get(name) {
                    return Ok(value.clone());
                }

                // 2. Check `it` magic variable (last eval result)
                if name == "it" {
                    if let Some(value) = &self.last_result {
                        return Ok(value.clone());
                    }
                    // `it` not set yet - return as unbound
                    return Ok(expr.clone());
                }

                // 3. Check defined functions
                if let Some(closure) = self.functions.get(name) {
                    if closure.params.is_empty() {
                        // It's a constant (define pi = 3.14)
                        self.eval_concrete(&closure.body)
                    } else {
                        // It's a function, return as-is
                        Ok(expr.clone())
                    }
                } else if self.adt_constructors.contains(name) {
                    // It's a nullary constructor (True, False, None, etc.)
                    Ok(expr.clone())
                } else {
                    // Unbound variable
                    Ok(expr.clone())
                }
            }

            // Operations: evaluate args then apply built-in or user-defined function
            Expression::Operation { name, args, .. } => {
                // First, evaluate all arguments
                // First, evaluate all arguments
                let eval_args: Result<Vec<_>, _> =
                    args.iter().map(|a| self.eval_concrete(a)).collect();
                let eval_args = eval_args?;

                // Fast-path for raw Typst helpers to ensure they always collapse
                if name == "typst_raw" {
                    return self.builtin_typst_raw(&eval_args).map(|res| {
                        res.unwrap_or(Expression::Operation {
                            name: name.clone(),
                            args: eval_args.clone(),
                            span: None,
                        })
                    });
                }
                if name == "concat" {
                    return self.builtin_concat(&eval_args).map(|res| {
                        res.unwrap_or(Expression::Operation {
                            name: name.clone(),
                            args: eval_args.clone(),
                            span: None,
                        })
                    });
                }

                // Check if this is a data constructor (e.g., Atom, List, Cons, Some)
                // Constructors are values - return them with evaluated args
                if self.all_constructors.contains(name) || self.is_constructor_name(name) {
                    return Ok(Expression::Operation {
                        name: name.clone(),
                        args: eval_args,
                        span: None,
                    });
                }

                // Try built-in operations first
                if let Some(result) = self.apply_builtin(name, &eval_args)? {
                    return Ok(result);
                }

                // Try user-defined functions
                if self.functions.contains_key(name) {
                    let applied = self.apply_function(name, eval_args)?;
                    return self.eval_concrete(&applied);
                }

                // Unknown operation - return as-is with evaluated args
                Ok(Expression::Operation {
                    name: name.clone(),
                    args: eval_args,
                    span: None,
                })
            }

            // Conditionals: evaluate condition and branch
            Expression::Conditional {
                condition,
                then_branch,
                else_branch,
                ..
            } => {
                let eval_cond = self.eval_concrete(condition)?;
                match &eval_cond {
                    Expression::Object(s) if s == "true" || s == "True" => {
                        self.eval_concrete(then_branch)
                    }
                    Expression::Object(s) if s == "false" || s == "False" => {
                        self.eval_concrete(else_branch)
                    }
                    Expression::Const(s) if s == "true" || s == "True" => {
                        self.eval_concrete(then_branch)
                    }
                    Expression::Const(s) if s == "false" || s == "False" => {
                        self.eval_concrete(else_branch)
                    }
                    _ => {
                        // Condition didn't evaluate to a boolean - return symbolic
                        Ok(Expression::Conditional {
                            condition: Box::new(eval_cond),
                            then_branch: Box::new(self.eval_concrete(then_branch)?),
                            else_branch: Box::new(self.eval_concrete(else_branch)?),
                            span: None,
                        })
                    }
                }
            }

            // Let bindings: evaluate value, bind, evaluate body
            Expression::Let {
                pattern,
                value,
                body,
                ..
            } => {
                let eval_value = self.eval_concrete(value)?;
                let mut subst = HashMap::new();
                self.match_pattern_to_bindings(pattern, &eval_value, &mut subst)?;
                let substituted_body = self.substitute(body, &subst);
                self.eval_concrete(&substituted_body)
            }

            // Match expressions
            Expression::Match {
                scrutinee, cases, ..
            } => {
                let eval_scrutinee = self.eval_concrete(scrutinee)?;
                let result = self.matcher.eval_match(&eval_scrutinee, cases)?;
                self.eval_concrete(&result)
            }

            // Lambda - return as value
            Expression::Lambda { .. } => Ok(expr.clone()),

            // Lists - evaluate elements
            Expression::List(elements) => {
                let eval_elements: Result<Vec<_>, _> =
                    elements.iter().map(|e| self.eval_concrete(e)).collect();
                Ok(Expression::List(eval_elements?))
            }

            // Ascription - evaluate inner, discard type
            Expression::Ascription { expr: inner, .. } => self.eval_concrete(inner),

            // Quantifiers - not for concrete evaluation
            Expression::Quantifier { .. } => Ok(expr.clone()),

            // Placeholder - return as-is
            Expression::Placeholder { .. } => Ok(expr.clone()),
        }
    }
}

mod builtins;
mod helpers;
mod plotting;

mod substitute;
mod verification;

#[cfg(feature = "numerical")]
mod lapack;

#[cfg(feature = "numerical")]
mod tensor;

impl Default for Evaluator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::LambdaParam;
    use crate::kleis_parser::parse_kleis_program;

    #[test]
    fn test_load_simple_function() {
        let mut eval = Evaluator::new();

        let code = "define pi = 3.14159";
        let program = parse_kleis_program(code).unwrap();
        eval.load_program(&program).unwrap();

        assert!(eval.has_function("pi"));
    }

    #[test]
    fn test_load_function_with_params() {
        let mut eval = Evaluator::new();

        let code = "define double(x) = x + x";
        let program = parse_kleis_program(code).unwrap();
        eval.load_program(&program).unwrap();

        assert!(eval.has_function("double"));
        let closure = eval.get_function("double").unwrap();
        assert_eq!(closure.params.len(), 1);
        assert_eq!(closure.params[0], "x");
    }

    #[test]
    fn test_apply_function() {
        let mut eval = Evaluator::new();

        let code = "define double(x) = x + x";
        let program = parse_kleis_program(code).unwrap();
        eval.load_program(&program).unwrap();

        // Apply: double(5)
        let result = eval
            .apply_function("double", vec![Expression::Const("5".to_string())])
            .unwrap();

        // Should get: 5 + 5 (symbolic)
        match result {
            Expression::Operation { name, args, .. } => {
                assert_eq!(name, "plus");
                assert_eq!(args.len(), 2);
                assert!(matches!(args[0], Expression::Const(ref s) if s == "5"));
                assert!(matches!(args[1], Expression::Const(ref s) if s == "5"));
            }
            _ => panic!("Expected Operation"),
        }
    }

    #[test]
    fn test_apply_function_two_params() {
        let mut eval = Evaluator::new();

        let code = "define add(x, y) = x + y";
        let program = parse_kleis_program(code).unwrap();
        eval.load_program(&program).unwrap();

        // Apply: add(3, 7)
        let result = eval
            .apply_function(
                "add",
                vec![
                    Expression::Const("3".to_string()),
                    Expression::Const("7".to_string()),
                ],
            )
            .unwrap();

        // Should get: 3 + 7 (symbolic)
        match result {
            Expression::Operation { name, args, .. } => {
                assert_eq!(name, "plus");
                assert_eq!(args.len(), 2);
            }
            _ => panic!("Expected Operation"),
        }
    }

    #[test]
    fn test_higher_order_function_basic() {
        // Test that functions can be passed as arguments and called
        let mut eval = Evaluator::new();

        let code = r#"
            define double(x) = x + x
            define apply_fn(f, x) = f(x)
            define test = apply_fn(double, 5)
        "#;
        let program = parse_kleis_program(code).unwrap();
        eval.load_program(&program).unwrap();

        let result = eval.eval_concrete(&Expression::Object("test".to_string()));
        assert!(result.is_ok(), "HOF should work: {:?}", result);

        // Result should be double(5) = 5 + 5 = 10
        let expr = result.unwrap();
        assert_eq!(expr, Expression::Const("10".to_string()));
    }

    #[test]
    fn test_higher_order_function_apply_twice() {
        // Test nested HOF calls: apply_twice(f, x) = f(f(x))
        let mut eval = Evaluator::new();

        let code = r#"
            define inc(x) = x + 1
            define apply_twice(f, x) = f(f(x))
            define test = apply_twice(inc, 10)
        "#;
        let program = parse_kleis_program(code).unwrap();
        eval.load_program(&program).unwrap();

        let result = eval.eval_concrete(&Expression::Object("test".to_string()));
        assert!(result.is_ok(), "Nested HOF should work: {:?}", result);
    }

    #[test]
    fn test_higher_order_function_with_pattern_match() {
        // Test HOF with pattern matching (the original use case for is_t/is_r)
        let mut eval = Evaluator::new();

        let code = r#"
            define is_one(x) = match x { 1 => 1 | _ => 0 }
            define check(f, x) = f(x)
            define test1 = check(is_one, 1)
            define test2 = check(is_one, 2)
        "#;
        let program = parse_kleis_program(code).unwrap();
        eval.load_program(&program).unwrap();

        let result1 = eval.eval_concrete(&Expression::Object("test1".to_string()));
        assert!(result1.is_ok());
        assert_eq!(
            result1.unwrap(),
            Expression::Const("1".to_string()),
            "is_one(1) should be 1"
        );

        let result2 = eval.eval_concrete(&Expression::Object("test2".to_string()));
        assert!(result2.is_ok());
        assert_eq!(
            result2.unwrap(),
            Expression::Const("0".to_string()),
            "is_one(2) should be 0"
        );
    }

    #[test]
    fn test_expression_span_is_source_of_truth_for_debugger() {
        use std::path::PathBuf;
        use std::sync::Arc;

        // The span on an Expression IS the source location for the debugger.
        // No lookup, no searching - just read expression.span.

        // Create a file path wrapped in Arc (cheap to clone)
        let file = Arc::new(PathBuf::from("test_file.kleis"));

        // Create an expression with a known span (line 42, column 10, file)
        let span = SourceSpan::new(42, 10).with_file(Arc::clone(&file));
        let expr = Expression::Operation {
            name: "plus".to_string(),
            args: vec![
                Expression::Const("1".to_string()),
                Expression::Const("2".to_string()),
            ],
            span: Some(span),
        };

        // This is what the debugger does: read the span from the current expression
        if let Expression::Operation { span, .. } = &expr {
            // Verify we get the correct location including file
            assert!(span.is_some(), "Expression should have a span");
            let loc = span.as_ref().unwrap();
            assert_eq!(loc.line, 42, "Debugger should report line 42");
            assert_eq!(loc.column, 10, "Debugger should report column 10");
            assert!(loc.file.is_some(), "Debugger should have file path");
            assert_eq!(
                loc.file.as_ref().unwrap().to_string_lossy(),
                "test_file.kleis"
            );
        } else {
            panic!("Expected Operation expression");
        }
    }

    #[test]
    fn test_eval_function_application() {
        let mut eval = Evaluator::new();

        let code = "define double(x) = x + x";
        let program = parse_kleis_program(code).unwrap();
        eval.load_program(&program).unwrap();

        // Evaluate: double(5)
        let expr = Expression::Operation {
            name: "double".to_string(),
            args: vec![Expression::Const("5".to_string())],
            span: None,
        };

        let result = eval.eval(&expr).unwrap();

        // Should get: 5 + 5
        match result {
            Expression::Operation { name, .. } => {
                assert_eq!(name, "plus");
            }
            _ => panic!("Expected Operation"),
        }
    }

    #[test]
    fn test_load_multiple_functions() {
        let mut eval = Evaluator::new();

        let code = r#"
            define pi = 3.14159
            define double(x) = x + x
            define add(x, y) = x + y
        "#;
        let program = parse_kleis_program(code).unwrap();
        eval.load_program(&program).unwrap();

        assert!(eval.has_function("pi"));
        assert!(eval.has_function("double"));
        assert!(eval.has_function("add"));
    }

    #[test]
    fn test_function_not_found() {
        let eval = Evaluator::new();

        let result = eval.apply_function("undefined", vec![]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not defined"));
    }

    #[test]
    fn test_function_wrong_arity() {
        let mut eval = Evaluator::new();

        let code = "define double(x) = x + x";
        let program = parse_kleis_program(code).unwrap();
        eval.load_program(&program).unwrap();

        // Try to call with wrong number of arguments
        let result = eval.apply_function(
            "double",
            vec![
                Expression::Const("1".to_string()),
                Expression::Const("2".to_string()),
            ],
        );

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("expects 1 arguments, got 2"));
    }

    #[test]
    fn test_beta_reduce_identity() {
        // (λ x . x)(5) → 5
        let eval = Evaluator::new();

        let lambda = Expression::Lambda {
            params: vec![LambdaParam::new("x")],
            body: Box::new(Expression::Object("x".to_string())),
            span: None,
        };

        let result = eval
            .beta_reduce(&lambda, &Expression::Const("5".to_string()))
            .unwrap();

        assert!(matches!(result, Expression::Const(ref s) if s == "5"));
    }

    #[test]
    fn test_beta_reduce_simple_arithmetic() {
        // (λ x . x + 1)(5) → 5 + 1
        let eval = Evaluator::new();

        let lambda = Expression::Lambda {
            params: vec![LambdaParam::new("x")],
            body: Box::new(Expression::Operation {
                name: "plus".to_string(),
                args: vec![
                    Expression::Object("x".to_string()),
                    Expression::Const("1".to_string()),
                ],
                span: None,
            }),
            span: None,
        };

        let result = eval
            .beta_reduce(&lambda, &Expression::Const("5".to_string()))
            .unwrap();

        match result {
            Expression::Operation { name, args, .. } => {
                assert_eq!(name, "plus");
                assert!(matches!(args[0], Expression::Const(ref s) if s == "5"));
                assert!(matches!(args[1], Expression::Const(ref s) if s == "1"));
            }
            _ => panic!("Expected Operation, got {:?}", result),
        }
    }

    #[test]
    fn test_beta_reduce_partial_application() {
        // (λ x y . x + y)(3) → λ y . 3 + y
        let eval = Evaluator::new();

        let lambda = Expression::Lambda {
            params: vec![LambdaParam::new("x"), LambdaParam::new("y")],
            body: Box::new(Expression::Operation {
                name: "plus".to_string(),
                args: vec![
                    Expression::Object("x".to_string()),
                    Expression::Object("y".to_string()),
                ],
                span: None,
            }),
            span: None,
        };

        let result = eval
            .beta_reduce(&lambda, &Expression::Const("3".to_string()))
            .unwrap();

        // Should be λ y . 3 + y
        match result {
            Expression::Lambda { params, body, .. } => {
                assert_eq!(params.len(), 1);
                assert_eq!(params[0].name, "y");

                match *body {
                    Expression::Operation { name, ref args, .. } => {
                        assert_eq!(name, "plus");
                        assert!(matches!(args[0], Expression::Const(ref s) if s == "3"));
                        assert!(matches!(args[1], Expression::Object(ref s) if s == "y"));
                    }
                    _ => panic!("Expected Operation in body"),
                }
            }
            _ => panic!("Expected Lambda, got {:?}", result),
        }
    }

    #[test]
    fn test_beta_reduce_full_application() {
        // (λ x y . x + y)(3)(4) → 7 (symbolically: 3 + 4)
        let eval = Evaluator::new();

        let lambda = Expression::Lambda {
            params: vec![LambdaParam::new("x"), LambdaParam::new("y")],
            body: Box::new(Expression::Operation {
                name: "plus".to_string(),
                args: vec![
                    Expression::Object("x".to_string()),
                    Expression::Object("y".to_string()),
                ],
                span: None,
            }),
            span: None,
        };

        // Apply first argument
        let partial = eval
            .beta_reduce(&lambda, &Expression::Const("3".to_string()))
            .unwrap();

        // Apply second argument
        let result = eval
            .beta_reduce(&partial, &Expression::Const("4".to_string()))
            .unwrap();

        match result {
            Expression::Operation { name, args, .. } => {
                assert_eq!(name, "plus");
                assert!(matches!(args[0], Expression::Const(ref s) if s == "3"));
                assert!(matches!(args[1], Expression::Const(ref s) if s == "4"));
            }
            _ => panic!("Expected Operation, got {:?}", result),
        }
    }

    #[test]
    fn test_beta_reduce_multi() {
        // Apply multiple args at once: (λ x y . x * y)(2, 3) → 2 * 3
        let eval = Evaluator::new();

        let lambda = Expression::Lambda {
            params: vec![LambdaParam::new("x"), LambdaParam::new("y")],
            body: Box::new(Expression::Operation {
                name: "times".to_string(),
                args: vec![
                    Expression::Object("x".to_string()),
                    Expression::Object("y".to_string()),
                ],
                span: None,
            }),
            span: None,
        };

        let result = eval
            .beta_reduce_multi(
                &lambda,
                &[
                    Expression::Const("2".to_string()),
                    Expression::Const("3".to_string()),
                ],
            )
            .unwrap();

        match result {
            Expression::Operation { name, args, .. } => {
                assert_eq!(name, "times");
                assert!(matches!(args[0], Expression::Const(ref s) if s == "2"));
                assert!(matches!(args[1], Expression::Const(ref s) if s == "3"));
            }
            _ => panic!("Expected Operation, got {:?}", result),
        }
    }

    #[test]
    fn test_free_variables() {
        let eval = Evaluator::new();

        // x + y has free variables x, y
        let expr = Expression::Operation {
            name: "plus".to_string(),
            args: vec![
                Expression::Object("x".to_string()),
                Expression::Object("y".to_string()),
            ],
            span: None,
        };

        let free = eval.free_variables(&expr);
        assert!(free.contains("x"));
        assert!(free.contains("y"));
        assert_eq!(free.len(), 2);
    }

    #[test]
    fn test_free_variables_in_lambda() {
        let eval = Evaluator::new();

        // λ x . x + y has only y free (x is bound)
        let lambda = Expression::Lambda {
            params: vec![LambdaParam::new("x")],
            body: Box::new(Expression::Operation {
                name: "plus".to_string(),
                args: vec![
                    Expression::Object("x".to_string()),
                    Expression::Object("y".to_string()),
                ],
                span: None,
            }),
            span: None,
        };

        let free = eval.free_variables(&lambda);
        assert!(!free.contains("x")); // x is bound
        assert!(free.contains("y")); // y is free
        assert_eq!(free.len(), 1);
    }

    #[test]
    fn test_bound_variables() {
        let eval = Evaluator::new();

        // λ x . λ y . x + y has bound variables x, y
        let lambda = Expression::Lambda {
            params: vec![LambdaParam::new("x")],
            body: Box::new(Expression::Lambda {
                params: vec![LambdaParam::new("y")],
                body: Box::new(Expression::Operation {
                    name: "plus".to_string(),
                    args: vec![
                        Expression::Object("x".to_string()),
                        Expression::Object("y".to_string()),
                    ],
                    span: None,
                }),
                span: None,
            }),
            span: None,
        };

        let bound = eval.bound_variables(&lambda);
        assert!(bound.contains("x"));
        assert!(bound.contains("y"));
    }

    #[test]
    fn test_alpha_conversion() {
        let eval = Evaluator::new();

        // λ y . y → λ y' . y' (when we need to rename y)
        let lambda = Expression::Lambda {
            params: vec![LambdaParam::new("y")],
            body: Box::new(Expression::Object("y".to_string())),
            span: None,
        };

        let converted = eval.alpha_convert(&lambda, "y", "z");

        match converted {
            Expression::Lambda { params, body, .. } => {
                assert_eq!(params[0].name, "z");
                assert!(matches!(*body, Expression::Object(ref s) if s == "z"));
            }
            _ => panic!("Expected Lambda"),
        }
    }

    #[test]
    fn test_variable_capture_avoidance() {
        let eval = Evaluator::new();

        // (λ x . λ y . x + y)(y) should NOT produce λ y . y + y
        // It should alpha-convert to avoid capture: λ y' . y + y'
        let outer_lambda = Expression::Lambda {
            params: vec![LambdaParam::new("x")],
            body: Box::new(Expression::Lambda {
                params: vec![LambdaParam::new("y")],
                body: Box::new(Expression::Operation {
                    name: "plus".to_string(),
                    args: vec![
                        Expression::Object("x".to_string()),
                        Expression::Object("y".to_string()),
                    ],
                    span: None,
                }),
                span: None,
            }),
            span: None,
        };

        // Apply with 'y' as argument (potential capture)
        let result = eval
            .beta_reduce(&outer_lambda, &Expression::Object("y".to_string()))
            .unwrap();

        // Result should be λ y' . y + y' (or similar fresh name)
        match result {
            Expression::Lambda { params, body, .. } => {
                let inner_param = &params[0].name;
                // The inner param should NOT be 'y' anymore (renamed to avoid capture)
                assert_ne!(inner_param, "y");

                match *body {
                    Expression::Operation { ref args, .. } => {
                        // First arg should be 'y' (the argument we passed)
                        assert!(matches!(args[0], Expression::Object(ref s) if s == "y"));
                        // Second arg should be the renamed parameter
                        assert!(matches!(args[1], Expression::Object(ref s) if s == inner_param));
                    }
                    _ => panic!("Expected Operation in body"),
                }
            }
            _ => panic!("Expected Lambda, got {:?}", result),
        }
    }

    #[test]
    fn test_reduce_named_lambda_function() {
        // define f = λ x . x + 1
        // f(5) → 5 + 1
        let mut eval = Evaluator::new();

        let code = "define f = λ x . x + 1";
        let program = parse_kleis_program(code).unwrap();
        eval.load_program(&program).unwrap();

        // Create expression f(5)
        let expr = Expression::Operation {
            name: "f".to_string(),
            args: vec![Expression::Const("5".to_string())],
            span: None,
        };

        let result = eval.reduce_to_normal_form(&expr).unwrap();

        match result {
            Expression::Operation { name, args, .. } => {
                assert_eq!(name, "plus");
                assert!(matches!(args[0], Expression::Const(ref s) if s == "5"));
                assert!(matches!(args[1], Expression::Const(ref s) if s == "1"));
            }
            _ => panic!("Expected Operation, got {:?}", result),
        }
    }

    #[test]
    fn test_reduce_curried_function() {
        // define add = λ x y . x + y
        // add(3)(4) → 3 + 4
        let mut eval = Evaluator::new();

        let code = "define add = λ x y . x + y";
        let program = parse_kleis_program(code).unwrap();
        eval.load_program(&program).unwrap();

        // For curried application, we need nested operations
        // First: add(3) → λ y . 3 + y
        let partial = Expression::Operation {
            name: "add".to_string(),
            args: vec![Expression::Const("3".to_string())],
            span: None,
        };

        let reduced_partial = eval.reduce_to_normal_form(&partial).unwrap();

        // Should be a lambda
        assert!(matches!(reduced_partial, Expression::Lambda { .. }));
    }

    #[test]
    fn test_reduction_fuel_limit() {
        let eval = Evaluator::new();

        // Create a simple expression that would take many steps
        let expr = Expression::Let {
            pattern: crate::ast::Pattern::Variable("x".to_string()),
            type_annotation: None,
            value: Box::new(Expression::Const("1".to_string())),
            body: Box::new(Expression::Object("x".to_string())),
            span: None,
        };

        // Should complete within fuel limit
        let result = eval.reduce_with_fuel(&expr, 10);
        assert!(result.is_ok());
    }

    #[test]
    fn test_typst_raw_unescape_concat() {
        let mut eval = Evaluator::new();
        // Strings are already unescaped (as the parser now produces them)
        let expr = Expression::Operation {
            name: "typst_raw".to_string(),
            args: vec![Expression::Operation {
                name: "concat".to_string(),
                args: vec![
                    Expression::String("a\n".to_string()),
                    Expression::String("b\"c".to_string()),
                ],
                span: None,
            }],
            span: None,
        };
        let result = eval.eval_concrete(&expr).unwrap();
        match result {
            Expression::Object(s) => assert_eq!(s, "a\nb\"c"),
            other => panic!("Expected Object, got {:?}", other),
        }
    }

    #[test]
    fn test_typst_raw_accepts_const_and_object() {
        let mut eval = Evaluator::new();

        // Const input
        let const_expr = Expression::Operation {
            name: "typst_raw".to_string(),
            args: vec![Expression::Const("foo".to_string())],
            span: None,
        };
        let const_result = eval.eval_concrete(&const_expr).unwrap();
        assert!(matches!(const_result, Expression::Object(ref s) if s == "foo"));

        // Object input
        let obj_expr = Expression::Operation {
            name: "typst_raw".to_string(),
            args: vec![Expression::Object("bar".to_string())],
            span: None,
        };
        let obj_result = eval.eval_concrete(&obj_expr).unwrap();
        assert!(matches!(obj_result, Expression::Object(ref s) if s == "bar"));
    }

    #[test]
    fn test_eval_concrete_arithmetic() {
        let eval = Evaluator::new();

        // 2 + 3 → 5
        let expr = Expression::Operation {
            name: "plus".to_string(),
            args: vec![
                Expression::Const("2".to_string()),
                Expression::Const("3".to_string()),
            ],
            span: None,
        };
        let result = eval.eval_concrete(&expr).unwrap();
        assert!(matches!(result, Expression::Const(ref s) if s == "5"));

        // 10 * 5 → 50
        let expr = Expression::Operation {
            name: "times".to_string(),
            args: vec![
                Expression::Const("10".to_string()),
                Expression::Const("5".to_string()),
            ],
            span: None,
        };
        let result = eval.eval_concrete(&expr).unwrap();
        assert!(matches!(result, Expression::Const(ref s) if s == "50"));

        // 7 - 3 → 4
        let expr = Expression::Operation {
            name: "minus".to_string(),
            args: vec![
                Expression::Const("7".to_string()),
                Expression::Const("3".to_string()),
            ],
            span: None,
        };
        let result = eval.eval_concrete(&expr).unwrap();
        assert!(matches!(result, Expression::Const(ref s) if s == "4"));
    }

    #[test]
    fn test_eval_concrete_string_ops() {
        let eval = Evaluator::new();

        // concat("hello", " world") → "hello world"
        let expr = Expression::Operation {
            name: "concat".to_string(),
            args: vec![
                Expression::String("hello".to_string()),
                Expression::String(" world".to_string()),
            ],
            span: None,
        };
        let result = eval.eval_concrete(&expr).unwrap();
        assert!(matches!(result, Expression::String(ref s) if s == "hello world"));

        // strlen("kleis") → 5
        let expr = Expression::Operation {
            name: "strlen".to_string(),
            args: vec![Expression::String("kleis".to_string())],
            span: None,
        };
        let result = eval.eval_concrete(&expr).unwrap();
        assert!(matches!(result, Expression::Const(ref s) if s == "5"));

        // hasPrefix("(define fib)", "(define") → true
        let expr = Expression::Operation {
            name: "hasPrefix".to_string(),
            args: vec![
                Expression::String("(define fib)".to_string()),
                Expression::String("(define".to_string()),
            ],
            span: None,
        };
        let result = eval.eval_concrete(&expr).unwrap();
        assert!(matches!(result, Expression::Object(ref s) if s == "true"));

        // contains("hello world", "wor") → true
        let expr = Expression::Operation {
            name: "contains".to_string(),
            args: vec![
                Expression::String("hello world".to_string()),
                Expression::String("wor".to_string()),
            ],
            span: None,
        };
        let result = eval.eval_concrete(&expr).unwrap();
        assert!(matches!(result, Expression::Object(ref s) if s == "true"));
    }

    #[test]
    fn test_eval_concrete_comparison() {
        let eval = Evaluator::new();

        // gt(5, 3) → true
        let expr = Expression::Operation {
            name: "gt".to_string(),
            args: vec![
                Expression::Const("5".to_string()),
                Expression::Const("3".to_string()),
            ],
            span: None,
        };
        let result = eval.eval_concrete(&expr).unwrap();
        assert!(matches!(result, Expression::Object(ref s) if s == "true"));

        // lt(2, 10) → true
        let expr = Expression::Operation {
            name: "lt".to_string(),
            args: vec![
                Expression::Const("2".to_string()),
                Expression::Const("10".to_string()),
            ],
            span: None,
        };
        let result = eval.eval_concrete(&expr).unwrap();
        assert!(matches!(result, Expression::Object(ref s) if s == "true"));

        // eq(5, 5) → true
        let expr = Expression::Operation {
            name: "eq".to_string(),
            args: vec![
                Expression::Const("5".to_string()),
                Expression::Const("5".to_string()),
            ],
            span: None,
        };
        let result = eval.eval_concrete(&expr).unwrap();
        assert!(matches!(result, Expression::Object(ref s) if s == "true"));
    }

    #[test]
    fn test_eval_concrete_conditional() {
        let eval = Evaluator::new();

        // if gt(5, 3) then "yes" else "no" → "yes"
        let expr = Expression::Conditional {
            condition: Box::new(Expression::Operation {
                name: "gt".to_string(),
                args: vec![
                    Expression::Const("5".to_string()),
                    Expression::Const("3".to_string()),
                ],
                span: None,
            }),
            then_branch: Box::new(Expression::String("yes".to_string())),
            else_branch: Box::new(Expression::String("no".to_string())),
            span: None,
        };
        let result = eval.eval_concrete(&expr).unwrap();
        assert!(matches!(result, Expression::String(ref s) if s == "yes"));

        // if lt(5, 3) then "yes" else "no" → "no"
        let expr = Expression::Conditional {
            condition: Box::new(Expression::Operation {
                name: "lt".to_string(),
                args: vec![
                    Expression::Const("5".to_string()),
                    Expression::Const("3".to_string()),
                ],
                span: None,
            }),
            then_branch: Box::new(Expression::String("yes".to_string())),
            else_branch: Box::new(Expression::String("no".to_string())),
            span: None,
        };
        let result = eval.eval_concrete(&expr).unwrap();
        assert!(matches!(result, Expression::String(ref s) if s == "no"));
    }

    #[test]
    fn test_eval_concrete_user_function() {
        let mut eval = Evaluator::new();

        // define double(x) = x + x
        let code = "define double(x) = x + x";
        let program = parse_kleis_program(code).unwrap();
        eval.load_program(&program).unwrap();

        // double(5) → 10
        let expr = Expression::Operation {
            name: "double".to_string(),
            args: vec![Expression::Const("5".to_string())],
            span: None,
        };
        let result = eval.eval_concrete(&expr).unwrap();
        assert!(matches!(result, Expression::Const(ref s) if s == "10"));
    }

    #[test]
    fn test_eval_concrete_recursion() {
        let mut eval = Evaluator::new();

        // define fib(n) = if le(n, 1) then n else fib(n - 1) + fib(n - 2)
        let code = "define fib(n) = if le(n, 1) then n else fib(n - 1) + fib(n - 2)";
        let program = parse_kleis_program(code).unwrap();
        eval.load_program(&program).unwrap();

        // fib(0) → 0
        let expr = Expression::Operation {
            name: "fib".to_string(),
            args: vec![Expression::Const("0".to_string())],
            span: None,
        };
        let result = eval.eval_concrete(&expr).unwrap();
        assert!(matches!(result, Expression::Const(ref s) if s == "0"));

        // fib(1) → 1
        let expr = Expression::Operation {
            name: "fib".to_string(),
            args: vec![Expression::Const("1".to_string())],
            span: None,
        };
        let result = eval.eval_concrete(&expr).unwrap();
        assert!(matches!(result, Expression::Const(ref s) if s == "1"));

        // fib(5) → 5
        let expr = Expression::Operation {
            name: "fib".to_string(),
            args: vec![Expression::Const("5".to_string())],
            span: None,
        };
        let result = eval.eval_concrete(&expr).unwrap();
        assert!(matches!(result, Expression::Const(ref s) if s == "5"));

        // fib(10) → 55
        let expr = Expression::Operation {
            name: "fib".to_string(),
            args: vec![Expression::Const("10".to_string())],
            span: None,
        };
        let result = eval.eval_concrete(&expr).unwrap();
        assert!(matches!(result, Expression::Const(ref s) if s == "55"));
    }

    #[test]
    fn test_eval_concrete_lisp_parsing() {
        let mut eval = Evaluator::new();

        // Define LISP parsing helpers
        let code = r#"
            define is_list_expr(s) = hasPrefix(s, "(")
            define strip_parens(s) = substr(s, 1, strlen(s) - 2)
            define get_op(s) = charAt(strip_parens(s), 0)
        "#;
        let program = parse_kleis_program(code).unwrap();
        eval.load_program(&program).unwrap();

        // is_list_expr("(+ 2 3)") → true
        let expr = Expression::Operation {
            name: "is_list_expr".to_string(),
            args: vec![Expression::String("(+ 2 3)".to_string())],
            span: None,
        };
        let result = eval.eval_concrete(&expr).unwrap();
        assert!(matches!(result, Expression::Object(ref s) if s == "true"));

        // strip_parens("(+ 2 3)") → "+ 2 3"
        let expr = Expression::Operation {
            name: "strip_parens".to_string(),
            args: vec![Expression::String("(+ 2 3)".to_string())],
            span: None,
        };
        let result = eval.eval_concrete(&expr).unwrap();
        assert!(matches!(result, Expression::String(ref s) if s == "+ 2 3"));

        // get_op("(+ 2 3)") → "+"
        let expr = Expression::Operation {
            name: "get_op".to_string(),
            args: vec![Expression::String("(+ 2 3)".to_string())],
            span: None,
        };
        let result = eval.eval_concrete(&expr).unwrap();
        assert!(matches!(result, Expression::String(ref s) if s == "+"));
    }

    #[test]
    fn test_remove_function() {
        let mut eval = Evaluator::new();

        let code = "define foo(x) = x + 1\ndefine bar(x) = x * 2";
        let program = parse_kleis_program(code).unwrap();
        eval.load_program(&program).unwrap();

        assert!(eval.has_function("foo"));
        assert!(eval.has_function("bar"));

        // Remove foo
        assert!(eval.remove_function("foo"));
        assert!(!eval.has_function("foo"));
        assert!(eval.has_function("bar"));

        // Removing again returns false
        assert!(!eval.remove_function("foo"));
    }

    #[test]
    fn test_remove_data_type() {
        let mut eval = Evaluator::new();

        let code = r#"
            data Color = Red | Green | Blue
            data Option(T) = None | Some(value: T)
        "#;
        let program = parse_kleis_program(code).unwrap();
        eval.load_program(&program).unwrap();

        // Constructors should be registered
        assert!(eval.get_adt_constructors().contains("Red"));
        assert!(eval.get_adt_constructors().contains("Green"));
        assert!(eval.get_adt_constructors().contains("Blue"));

        // Remove Color data type
        assert!(eval.remove_data_type("Color"));

        // Constructors should be gone
        assert!(!eval.get_adt_constructors().contains("Red"));
        assert!(!eval.get_adt_constructors().contains("Green"));
        assert!(!eval.get_adt_constructors().contains("Blue"));

        // Option should still exist - verify by checking data type count
        let (_, data_count, _, _) = eval.definition_counts();
        assert_eq!(data_count, 1); // Only Option remains
    }

    #[test]
    fn test_reset() {
        let mut eval = Evaluator::new();

        let code = r#"
            define foo(x) = x + 1
            data Color = Red | Green | Blue
        "#;
        let program = parse_kleis_program(code).unwrap();
        eval.load_program(&program).unwrap();

        // Set some bindings
        eval.set_binding("x".to_string(), Expression::Const("42".to_string()));
        eval.set_last_result(Expression::Const("100".to_string()));

        assert!(eval.has_function("foo"));
        assert!(eval.get_adt_constructors().contains("Red"));
        assert!(eval.get_binding("x").is_some());
        assert!(eval.get_last_result().is_some());

        // Reset
        eval.reset();

        // Everything should be gone
        assert!(!eval.has_function("foo"));
        assert!(!eval.get_adt_constructors().contains("Red"));
        assert!(eval.get_binding("x").is_none());
        assert!(eval.get_last_result().is_none());

        let (funcs, data, structs, bindings) = eval.definition_counts();
        assert_eq!(funcs, 0);
        assert_eq!(data, 0);
        assert_eq!(structs, 0);
        assert_eq!(bindings, 0);
    }

    #[test]
    fn test_definition_counts() {
        let mut eval = Evaluator::new();

        let code = r#"
            define foo(x) = x + 1
            define bar(x) = x * 2
            data Color = Red | Green | Blue
        "#;
        let program = parse_kleis_program(code).unwrap();
        eval.load_program(&program).unwrap();

        eval.set_binding("x".to_string(), Expression::Const("1".to_string()));
        eval.set_binding("y".to_string(), Expression::Const("2".to_string()));

        let (funcs, data, structs, bindings) = eval.definition_counts();
        assert_eq!(funcs, 2);
        assert_eq!(data, 1);
        assert_eq!(structs, 0);
        assert_eq!(bindings, 2);
    }

    #[test]
    fn test_eval_without_debug_hook() {
        // Default behavior: no debug hook set, eval works normally
        let eval = Evaluator::new();

        // Simple constant - should return as-is
        let expr = Expression::Const("42".to_string());
        let result = eval.eval(&expr);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Expression::Const("42".to_string()));
    }

    #[test]
    fn test_eval_function_call_no_hook() {
        let mut eval = Evaluator::new();

        // Load a function
        let code = "define double(x) = x + x";
        let program = parse_kleis_program(code).unwrap();
        eval.load_program(&program).unwrap();

        // Call without debug hook
        let expr = Expression::Operation {
            name: "double".to_string(),
            args: vec![Expression::Const("5".to_string())],
            span: None,
        };
        let result = eval.eval(&expr);
        assert!(result.is_ok());

        // Result should be 5 + 5 (symbolic)
        match result.unwrap() {
            Expression::Operation { name, args, .. } => {
                assert_eq!(name, "plus"); // Parser converts + to "plus"
                assert_eq!(args.len(), 2);
            }
            other => panic!("Expected Operation, got {:?}", other),
        }
    }

    #[test]
    fn test_eval_with_debug_hook_tracks_bindings() {
        use crate::debug::{DebugAction, DebugHook, DebugState, SourceLocation, StackFrame};
        use std::sync::{Arc, Mutex};

        // A hook that tracks bindings (uses Arc<Mutex> for shared state)
        struct BindingTracker {
            bindings: Arc<Mutex<Vec<(String, String)>>>,
        }

        impl DebugHook for BindingTracker {
            fn on_eval_start(
                &mut self,
                _: &Expression,
                _: &SourceLocation,
                _: usize,
            ) -> DebugAction {
                DebugAction::Continue
            }
            fn on_eval_end(&mut self, _: &Expression, _: &Result<Expression, String>, _: usize) {}
            fn on_function_enter(
                &mut self,
                _: &str,
                _: &[Expression],
                _: &SourceLocation,
                _: usize,
            ) {
            }
            fn on_function_exit(&mut self, _: &str, _: &Result<Expression, String>, _: usize) {}
            fn on_bind(&mut self, name: &str, value: &Expression, _: usize) {
                self.bindings
                    .lock()
                    .unwrap()
                    .push((name.to_string(), format!("{:?}", value)));
            }
            fn state(&self) -> &DebugState {
                &DebugState::Running
            }
            fn should_stop(&self, _: &SourceLocation, _: usize) -> bool {
                false
            }
            fn wait_for_command(&mut self) -> DebugAction {
                DebugAction::Continue
            }
            fn get_stack(&self) -> &[StackFrame] {
                &[]
            }
            fn push_frame(&mut self, _: StackFrame) {}
            fn pop_frame(&mut self) -> Option<StackFrame> {
                None
            }
        }

        let mut eval = Evaluator::new();

        // Shared state to collect bindings
        let bindings = Arc::new(Mutex::new(Vec::new()));
        let hook = BindingTracker {
            bindings: bindings.clone(),
        };

        // Set the debug hook
        eval.set_debug_hook(Box::new(hook));

        // Load a function with parameters
        let code = "define add(a, b) = a + b";
        let program = parse_kleis_program(code).unwrap();
        eval.load_program(&program).unwrap();

        // Call the function - debug hook will be called automatically
        let expr = Expression::Operation {
            name: "add".to_string(),
            args: vec![
                Expression::Const("1".to_string()),
                Expression::Const("2".to_string()),
            ],
            span: None,
        };
        let result = eval.eval(&expr);
        assert!(result.is_ok());

        // Check that bindings were tracked
        let tracked = bindings.lock().unwrap();
        assert!(
            tracked.len() >= 2,
            "Expected at least 2 bindings, got {}",
            tracked.len()
        );
        assert!(tracked.iter().any(|(name, _)| name == "a"));
        assert!(tracked.iter().any(|(name, _)| name == "b"));
    }

    #[test]
    fn test_set_and_clear_debug_hook() {
        let eval = Evaluator::new();

        // Initially, no hook is set
        assert!(!eval.is_debugging());

        // Set a hook
        use crate::debug::NoOpDebugHook;
        eval.set_debug_hook(Box::new(NoOpDebugHook));
        assert!(eval.is_debugging());

        // Clear the hook
        eval.clear_debug_hook();
        assert!(!eval.is_debugging());
    }

    #[test]
    fn test_eval_example_block_simple() {
        use crate::kleis_ast::{ExampleBlock, ExampleStatement};

        let mut eval = Evaluator::new();

        // Create a simple example block
        let example = ExampleBlock {
            name: "simple test".to_string(),
            statements: vec![
                ExampleStatement::Let {
                    name: "x".to_string(),
                    type_annotation: None,
                    value: Expression::Const("5".to_string()),
                    location: None,
                },
                ExampleStatement::Let {
                    name: "y".to_string(),
                    type_annotation: None,
                    value: Expression::Const("5".to_string()),
                    location: None,
                },
                ExampleStatement::Assert {
                    condition: Expression::Operation {
                        name: "eq".to_string(),
                        args: vec![
                            Expression::Object("x".to_string()),
                            Expression::Object("y".to_string()),
                        ],
                        span: None,
                    },
                    location: None,
                },
            ],
        };

        let result = eval.eval_example_block(&example);
        assert!(
            result.passed,
            "Expected example to pass: {:?}",
            result.error
        );
        assert_eq!(result.assertions_passed, 1);
        assert_eq!(result.assertions_total, 1);
    }

    #[test]
    fn test_eval_example_block_failing_assert() {
        use crate::kleis_ast::{ExampleBlock, ExampleStatement};

        let mut eval = Evaluator::new();

        // Create an example block with a failing assertion
        let example = ExampleBlock {
            name: "failing test".to_string(),
            statements: vec![
                ExampleStatement::Let {
                    name: "x".to_string(),
                    type_annotation: None,
                    value: Expression::Const("5".to_string()),
                    location: None,
                },
                ExampleStatement::Let {
                    name: "y".to_string(),
                    type_annotation: None,
                    value: Expression::Const("10".to_string()),
                    location: None,
                },
                ExampleStatement::Assert {
                    condition: Expression::Operation {
                        name: "eq".to_string(),
                        args: vec![
                            Expression::Object("x".to_string()),
                            Expression::Object("y".to_string()),
                        ],
                        span: None,
                    },
                    location: None,
                },
            ],
        };

        let result = eval.eval_example_block(&example);
        assert!(!result.passed, "Expected example to fail");
        assert!(result.error.is_some());
        assert_eq!(result.assertions_passed, 0);
        assert_eq!(result.assertions_total, 1);
    }

    #[test]
    fn test_eval_example_block_bindings_dont_leak() {
        use crate::kleis_ast::{ExampleBlock, ExampleStatement};

        let mut eval = Evaluator::new();

        // Set a binding before the example
        eval.set_binding("outer".to_string(), Expression::Const("1".to_string()));

        let example = ExampleBlock {
            name: "scope test".to_string(),
            statements: vec![ExampleStatement::Let {
                name: "inner".to_string(),
                type_annotation: None,
                value: Expression::Const("2".to_string()),
                location: None,
            }],
        };

        let result = eval.eval_example_block(&example);
        assert!(result.passed);

        // The inner binding should NOT leak out
        assert!(eval.get_binding("inner").is_none());
        // The outer binding should still exist
        assert!(eval.get_binding("outer").is_some());
    }

    #[test]
    fn test_run_all_examples() {
        use crate::kleis_parser::parse_kleis_program;

        let code = r#"
            example "test one" {
                let x = 1
            }
            
            example "test two" {
                let y = 2
            }
        "#;

        let program = parse_kleis_program(code).unwrap();
        let mut eval = Evaluator::new();
        let results = eval.run_all_examples(&program);

        assert_eq!(results.len(), 2);
        assert!(results[0].passed);
        assert!(results[1].passed);
        assert_eq!(results[0].name, "test one");
        assert_eq!(results[1].name, "test two");
    }

    #[test]
    fn test_object_resolves_from_bindings() {
        // Test that Expression::Object looks up values from self.bindings
        // This is critical for example blocks where let-bound variables
        // must be accessible when evaluating expressions containing Object references
        let mut eval = Evaluator::new();

        // Set a binding
        eval.set_binding("x".to_string(), Expression::Const("42".to_string()));

        // Evaluate an Object that references the binding
        let result = eval.eval(&Expression::Object("x".to_string())).unwrap();

        // Should resolve to the bound value
        assert_eq!(result, Expression::Const("42".to_string()));
    }

    #[test]
    fn test_bindings_used_in_operations() {
        // Test that bindings are resolved when used inside operations
        let mut eval = Evaluator::new();

        eval.set_binding("a".to_string(), Expression::Const("10".to_string()));
        eval.set_binding("b".to_string(), Expression::Const("5".to_string()));

        // Evaluate: a + b where a=10, b=5
        let expr = Expression::Operation {
            name: "plus".to_string(),
            args: vec![
                Expression::Object("a".to_string()),
                Expression::Object("b".to_string()),
            ],
            span: None,
        };

        // eval() resolves bindings, eval_concrete() also computes arithmetic
        let result = eval.eval_concrete(&expr).unwrap();

        // Should compute 10 + 5 = 15
        assert_eq!(result, Expression::Const("15".to_string()));
    }

    #[test]
    fn test_bindings_captured_in_lambda() {
        // Test that lambdas can access bindings from enclosing scope
        // This is the key fix for inverted_pendulum.kleis
        use crate::kleis_parser::parse_kleis;

        let mut eval = Evaluator::new();

        // Set up a binding
        eval.set_binding("k".to_string(), Expression::Const("2".to_string()));

        // Create and evaluate: let f = lambda x . k * x in f(5)
        // k should be captured from bindings
        let code = "let f = lambda x . k * x in f(5)";
        let expr = parse_kleis(code).unwrap();
        let result = eval.eval_concrete(&expr).unwrap();

        // k=2, x=5, so k*x = 10
        assert_eq!(result, Expression::Const("10".to_string()));
    }
}
