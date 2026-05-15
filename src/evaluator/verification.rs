use std::collections::HashMap;

use crate::ast::Expression;
use crate::axiom_verifier::{AxiomVerifier, VerificationResult};
use crate::debug::SourceLocation;
use crate::kleis_ast::{ExampleBlock, ExampleStatement, FunctionDef, Program, TopLevel};
use crate::structure_registry::StructureRegistry;

use super::{AssertResult, Evaluator, ExampleResult};
use crate::solvers::backend::Witness;

impl Evaluator {
    /// Helper: Get the source location of a statement (if available)
    pub(crate) fn get_statement_location(
        stmt: &ExampleStatement,
    ) -> Option<crate::ast::FullSourceLocation> {
        match stmt {
            ExampleStatement::Let { location, .. } => location.clone(),
            ExampleStatement::Assert { location, .. } => location.clone(),
            ExampleStatement::Expr { location, .. } => location.clone(),
        }
    }

    /// Helper: Convert a statement to an expression for debug hook
    pub(crate) fn statement_to_expr(stmt: &ExampleStatement) -> Expression {
        match stmt {
            ExampleStatement::Let { value, .. } => value.clone(),
            ExampleStatement::Assert { condition, .. } => condition.clone(),
            ExampleStatement::Expr { expr, .. } => expr.clone(),
        }
    }

    /// Evaluate an example block, returning the result
    ///
    /// Example blocks execute statements sequentially:
    /// - `let` bindings add to local scope
    /// - `assert` statements check conditions
    /// - Expression statements are evaluated for side effects
    ///
    /// # Arguments
    /// * `example` - The example block to evaluate
    ///
    /// # Returns
    /// * `ExampleResult` - Summary of the example execution
    pub fn eval_example_block(&mut self, example: &ExampleBlock) -> ExampleResult {
        let mut assertions_passed = 0;
        let mut assertions_total = 0;
        let mut last_witness: Option<Witness> = None;

        // Create a snapshot of current bindings to restore later
        let saved_bindings = self.bindings.clone();

        for stmt in &example.statements {
            // Call debug hook with statement location (includes file path)
            if let Some(full_loc) = Self::get_statement_location(stmt) {
                // Convert FullSourceLocation to debug::SourceLocation
                let loc = SourceLocation::new(full_loc.line, full_loc.column);
                let loc = if let Some(ref file) = full_loc.file {
                    loc.with_file(std::path::PathBuf::from(file))
                } else {
                    loc
                };

                if let Some(ref mut hook) = *self.debug_hook.borrow_mut() {
                    let action = hook.on_eval_start(
                        &Self::statement_to_expr(stmt),
                        &loc,
                        0, // top-level depth
                    );
                    // Handle step/continue actions
                    match action {
                        crate::debug::DebugAction::Continue => {}
                        crate::debug::DebugAction::StepInto
                        | crate::debug::DebugAction::StepOver
                        | crate::debug::DebugAction::StepOut => {
                            // These will be handled by the hook's wait_for_command
                        }
                    }
                }
            }

            match stmt {
                ExampleStatement::Let {
                    name,
                    type_annotation: _,
                    value,
                    location: _,
                } => {
                    // Evaluate the value: first via eval (fires debug hooks for
                    // cross-file stepping), then eval_concrete to fully reduce.
                    // eval_internal returns substituted-but-unevaluated bodies for
                    // user-defined functions; without the second step, let bindings
                    // would hold intermediate expressions that break pattern matching.
                    match self.eval(value) {
                        Ok(partial) => {
                            let evaluated = self.eval_concrete(&partial).unwrap_or(partial);
                            {
                                let mut hook_ref = self.debug_hook.borrow_mut();
                                if let Some(ref mut hook) = *hook_ref {
                                    hook.on_bind(name, &evaluated, 0);
                                }
                            }
                            self.bindings.insert(name.clone(), evaluated);
                        }
                        Err(e) => {
                            // Restore bindings and return error
                            self.bindings = saved_bindings;
                            return ExampleResult {
                                name: example.name.clone(),
                                passed: false,
                                error: Some(format!("Error evaluating let {}: {}", name, e)),
                                assertions_passed,
                                assertions_total,
                                witness: None,
                            };
                        }
                    }
                }
                ExampleStatement::Assert {
                    condition,
                    location: _,
                } => {
                    assertions_total += 1;
                    let result = self.eval_assert(condition);

                    // Notify debug hook about assertion verification
                    {
                        let mut hook_ref = self.debug_hook.borrow_mut();
                        if let Some(ref mut hook) = *hook_ref {
                            match &result {
                                AssertResult::Passed => {
                                    hook.on_assert_verified(
                                        condition,
                                        true,
                                        "Passed (concrete)",
                                        0,
                                    );
                                }
                                AssertResult::Verified { witness } => {
                                    let msg = if let Some(w) = witness {
                                        format!("Verified by Z3 ✓ (witness: {})", w)
                                    } else {
                                        "Verified by Z3 ✓".to_string()
                                    };
                                    hook.on_assert_verified(condition, true, &msg, 0);
                                }
                                AssertResult::Failed { expected, actual } => {
                                    hook.on_assert_verified(
                                        condition,
                                        false,
                                        &format!(
                                            "Failed: expected {:?}, got {:?}",
                                            expected, actual
                                        ),
                                        0,
                                    );
                                }
                                AssertResult::Disproved { witness } => {
                                    hook.on_assert_verified(
                                        condition,
                                        false,
                                        &format!("Disproved by Z3: {}", witness),
                                        0,
                                    );
                                }
                                AssertResult::Unknown(reason) => {
                                    hook.on_assert_verified(
                                        condition,
                                        false,
                                        &format!("Unknown: {}", reason),
                                        0,
                                    );
                                }
                                AssertResult::InconsistentAxioms => {
                                    hook.on_assert_verified(
                                        condition,
                                        false,
                                        "AXIOM INCONSISTENCY: loaded axioms are contradictory",
                                        0,
                                    );
                                }
                            }
                        }
                    }

                    match result {
                        AssertResult::Passed => {
                            assertions_passed += 1;
                        }
                        AssertResult::Verified { witness } => {
                            assertions_passed += 1;
                            if witness.is_some() {
                                last_witness = witness;
                            }
                        }
                        AssertResult::Failed { expected, actual } => {
                            // Restore bindings and return error
                            self.bindings = saved_bindings;
                            return ExampleResult {
                                name: example.name.clone(),
                                passed: false,
                                error: Some(format!(
                                    "Assertion failed: expected {:?}, got {:?}",
                                    expected, actual
                                )),
                                assertions_passed,
                                assertions_total,
                                witness: None,
                            };
                        }
                        AssertResult::Disproved { witness } => {
                            // Z3 found a counterexample!
                            self.bindings = saved_bindings;
                            return ExampleResult {
                                name: example.name.clone(),
                                passed: false,
                                error: Some(format!(
                                    "Assertion disproved by Z3. Counterexample: {}",
                                    witness
                                )),
                                assertions_passed,
                                assertions_total,
                                witness: None,
                            };
                        }
                        AssertResult::InconsistentAxioms => {
                            self.bindings = saved_bindings;
                            return ExampleResult {
                                name: example.name.clone(),
                                passed: false,
                                error: Some(
                                    "AXIOM INCONSISTENCY DETECTED: The loaded axioms are \
                                     mutually unsatisfiable (Z3 proved them contradictory). \
                                     All assertions would be vacuously true. \
                                     This is a theory bug — check your axiom definitions \
                                     for contradictions."
                                        .to_string(),
                                ),
                                assertions_passed,
                                assertions_total,
                                witness: None,
                            };
                        }
                        AssertResult::Unknown(reason) => {
                            // Unknown means we couldn't verify - fail with explanation
                            // (could be Z3 timeout, feature disabled, or symbolic limitation)
                            self.bindings = saved_bindings;
                            return ExampleResult {
                                name: example.name.clone(),
                                passed: false,
                                error: Some(format!("Assertion unknown: {}", reason)),
                                assertions_passed,
                                assertions_total,
                                witness: None,
                            };
                        }
                    }
                }
                ExampleStatement::Expr { expr, location: _ } => {
                    // Evaluate expression concretely for side effects (e.g., out())
                    if let Err(e) = self.eval_concrete(expr) {
                        // Restore bindings and return error
                        self.bindings = saved_bindings;
                        return ExampleResult {
                            name: example.name.clone(),
                            passed: false,
                            error: Some(format!("Error evaluating expression: {}", e)),
                            assertions_passed,
                            assertions_total,
                            witness: None,
                        };
                    }
                }
            }
        }

        // Restore original bindings (example blocks don't leak bindings)
        self.bindings = saved_bindings;

        ExampleResult {
            name: example.name.clone(),
            passed: true,
            error: None,
            assertions_passed,
            assertions_total,
            witness: last_witness,
        }
    }

    /// Evaluate an assert condition
    ///
    /// Handles different forms of assertions:
    /// - `a = b` - Equality check
    /// - `predicate(x)` - Predicate check (must evaluate to true-like)
    /// - Concrete values - Directly check if true
    /// - Symbolic values - Return Unknown (for future Z3 integration)
    pub(crate) fn eval_assert(&self, condition: &Expression) -> AssertResult {
        // Check if this is an equality assertion: a = b
        if let Expression::Operation { name, args, .. } = condition
            && (name == "eq" || name == "equals" || name == "=")
            && args.len() == 2
        {
            return self.eval_equality_assert(&args[0], &args[1]);
        }

        // For quantified assertions, try Z3 first (they can't be evaluated concretely)
        if matches!(condition, Expression::Quantifier { .. }) {
            if let Some(result) = self.verify_with_z3(condition) {
                return result;
            }
            // Z3 couldn't help - return unknown
            return AssertResult::Unknown(
                "Quantified assertion could not be verified (Z3 unavailable or inconclusive)"
                    .to_string(),
            );
        }

        // Otherwise, evaluate the condition and check if it's "true"
        match self.eval(condition) {
            Ok(result) => {
                if self.is_truthy(&result) {
                    AssertResult::Passed
                } else {
                    // Try Z3 as fallback for any non-truthy result.
                    // This handles both symbolic expressions (free variables)
                    // and unreduced operations like `f(x) = y ∧ g(x) = z`
                    // where the evaluator can't simplify but Z3 can verify
                    // via loaded axioms.
                    if let Some(z3_result) = self.verify_with_z3(condition) {
                        return z3_result;
                    }
                    AssertResult::Failed {
                        expected: Box::new(Expression::Object("true".to_string())),
                        actual: Box::new(result),
                    }
                }
            }
            Err(e) => AssertResult::Unknown(format!("Could not evaluate: {}", e)),
        }
    }

    /// Build a StructureRegistry from the evaluator's loaded structures
    pub fn build_registry(&self, registry: &mut StructureRegistry) {
        for structure in &self.structures {
            let _ = registry.register(structure.clone());
        }
        // Add implements blocks (for where constraints)
        for impl_def in &self.implements_blocks {
            registry.register_implements(impl_def.clone());
        }
        // Add data types (for ADT constructor recognition)
        for data_def in &self.data_types {
            registry.register_data_type(data_def.clone());
        }
        // Add type aliases
        for type_alias in &self.type_aliases {
            registry.register_type_alias(
                type_alias.name.clone(),
                type_alias.params.clone(),
                type_alias.type_expr.clone(),
            );
        }
        // Add top-level operations
        for (name, type_sig) in &self.toplevel_operations {
            registry.register_toplevel_operation(name.clone(), type_sig.clone());
        }
        // Add function definitions (convert Closure to FunctionDef for Z3)
        for (name, closure) in &self.functions {
            let func_def = FunctionDef {
                name: name.clone(),
                params: closure.params.clone(),
                type_annotation: None, // Closures don't preserve type annotations
                body: closure.body.clone(),
                span: closure.span.clone(),
            };
            registry.register_function(func_def);
        }
    }

    /// Build a new StructureRegistry from the evaluator's loaded structures (internal use)
    pub(crate) fn build_registry_internal(&self) -> StructureRegistry {
        let mut registry = StructureRegistry::new();
        self.build_registry(&mut registry);
        registry
    }

    /// Try to verify an assertion using Z3 (for symbolic claims)
    pub fn verify_with_z3(&self, condition: &Expression) -> Option<AssertResult> {
        // Fast path: if we already know axioms are inconsistent, skip solver entirely
        if let Some(Some(false)) = *self.axiom_consistency_cache.borrow() {
            return Some(AssertResult::InconsistentAxioms);
        }

        let registry = self.build_registry_internal();

        match AxiomVerifier::new(&registry) {
            Ok(mut verifier) => {
                // Pass cached consistency result to avoid redundant checks.
                // If we already checked and got Sat or Unknown, tell the verifier
                // to skip the expensive re-check.
                if let Some(cached) = *self.axiom_consistency_cache.borrow() {
                    verifier.set_consistency_cache(cached);
                }

                verifier.load_adt_constructors(self.adt_constructors.iter());

                let result = verifier.verify_axiom(condition);

                // Capture the verifier's consistency result for future assertions
                if let Some(consistency) = verifier.get_consistency_result() {
                    *self.axiom_consistency_cache.borrow_mut() = Some(consistency);
                }

                match result {
                    Ok(result) => match result {
                        VerificationResult::Valid => Some(AssertResult::Verified { witness: None }),
                        VerificationResult::ValidWithWitness { witness } => {
                            Some(AssertResult::Verified {
                                witness: Some(witness),
                            })
                        }
                        VerificationResult::Invalid { witness } => {
                            Some(AssertResult::Disproved { witness })
                        }
                        VerificationResult::InconsistentAxioms => {
                            *self.axiom_consistency_cache.borrow_mut() = Some(Some(false));
                            Some(AssertResult::InconsistentAxioms)
                        }
                        VerificationResult::Unknown => None,
                        VerificationResult::Disabled => None,
                    },
                    Err(_) => None,
                }
            }
            Err(_) => None,
        }
    }

    /// Targeted Z3 verification for a structure operation with concrete arguments.
    ///
    /// Instead of loading quantified axioms (which cause timeouts with Z3's
    /// string theory), this method instantiates axioms with the concrete
    /// argument, producing ground (quantifier-free) assertions that Z3 can
    /// solve instantly.
    pub fn verify_structure_operation(
        &self,
        condition: &Expression,
        structure_name: &str,
    ) -> Option<AssertResult> {
        let concrete_arg = match condition {
            Expression::Operation { args, .. } if args.len() == 1 => &args[0],
            _ => return self.verify_with_z3(condition),
        };

        const LARGE_STRING_THRESHOLD: usize = 1_000_000;
        if let Expression::String(s) = concrete_arg
            && s.len() > LARGE_STRING_THRESHOLD
        {
            eprintln!(
                "[kleis-review] Warning: large string ({} bytes) passed to Z3 — \
                     temporary memory usage may be high",
                s.len()
            );
        }

        let structure = self.structures.iter().find(|s| s.name == structure_name)?;

        let mut ground_axioms = Vec::new();
        for member in &structure.members {
            if let crate::kleis_ast::StructureMember::Axiom {
                proposition:
                    Expression::Quantifier {
                        variables, body, ..
                    },
                ..
            } = member
                && variables.len() == 1
            {
                let mut subst = HashMap::new();
                subst.insert(variables[0].name.clone(), concrete_arg.clone());
                ground_axioms.push(self.substitute(body, &subst));
            }
        }

        if ground_axioms.is_empty() {
            return self.verify_with_z3(condition);
        }

        let mut ground_structure = structure.clone();
        ground_structure.members = ground_structure
            .members
            .iter()
            .filter(|m| !matches!(m, crate::kleis_ast::StructureMember::Axiom { .. }))
            .cloned()
            .collect();
        for (i, axiom) in ground_axioms.iter().enumerate() {
            ground_structure
                .members
                .push(crate::kleis_ast::StructureMember::Axiom {
                    name: format!("ground_{}", i),
                    proposition: axiom.clone(),
                });
        }

        let mut registry = StructureRegistry::new();
        let _ = registry.register(ground_structure);

        match AxiomVerifier::new(&registry) {
            Ok(mut verifier) => {
                verifier.set_consistency_cache(Some(true));
                verifier.load_adt_constructors(self.adt_constructors.iter());

                let result = verifier.verify_axiom(condition);
                match result {
                    Ok(result) => match result {
                        VerificationResult::Valid => Some(AssertResult::Verified { witness: None }),
                        VerificationResult::ValidWithWitness { witness } => {
                            Some(AssertResult::Verified {
                                witness: Some(witness),
                            })
                        }
                        VerificationResult::Invalid { witness } => {
                            Some(AssertResult::Disproved { witness })
                        }
                        VerificationResult::InconsistentAxioms => {
                            Some(AssertResult::InconsistentAxioms)
                        }
                        VerificationResult::Unknown => None,
                        VerificationResult::Disabled => None,
                    },
                    Err(_) => None,
                }
            }
            Err(_) => None,
        }
    }

    /// Evaluate a ground term (no free variables) using Z3 simplify.
    ///
    /// This provides concrete evaluation for expressions like:
    /// - `eval(1 + 2 * 3)` → `7`
    /// - `eval(if 5 ≤ 3 then 1 else 2)` → `2`
    ///
    /// # Errors
    /// Returns an error if:
    /// - Z3 is not available (axiom-verification feature disabled)
    /// - Z3 fails to simplify the expression
    pub(crate) fn eval_ground_term(&self, expr: &Expression) -> Result<Expression, String> {
        let registry = self.build_registry_internal();

        match AxiomVerifier::new(&registry) {
            Ok(mut verifier) => {
                // Load ADT constructors for proper type handling
                verifier.load_adt_constructors(self.adt_constructors.iter());

                // Use Z3 simplify to evaluate the ground term
                match verifier.simplify(expr) {
                    Ok(simplified) => Ok(simplified),
                    Err(e) => Err(format!("eval() failed to simplify expression: {}", e)),
                }
            }
            Err(e) => Err(format!(
                "eval() requires Z3 (axiom-verification feature). Error: {}",
                e
            )),
        }
    }

    /// Evaluate an equality assertion: assert(a = b)
    pub(crate) fn eval_equality_assert(
        &self,
        left: &Expression,
        right: &Expression,
    ) -> AssertResult {
        // First, resolve any variables in the expressions
        let left_resolved = self.resolve_expression(left);
        let right_resolved = self.resolve_expression(right);

        // Try to FULLY evaluate both sides using eval_concrete()
        // This ensures sin(0) becomes 0, not Operation{sin, [0]}
        let left_result = self.eval_concrete(&left_resolved);
        let right_result = self.eval_concrete(&right_resolved);

        match (left_result, right_result) {
            (Ok(left_val), Ok(right_val)) => {
                // Both sides evaluated - check structural equality
                if self.expressions_equal(&left_val, &right_val) {
                    return AssertResult::Passed;
                }

                // For numeric comparisons, also try floating-point equality
                // (handles cases like 1.0 vs 1 or floating point rounding)
                if let (Some(left_num), Some(right_num)) =
                    (self.as_number(&left_val), self.as_number(&right_val))
                {
                    // Use relative epsilon for floating point comparison
                    let diff = (left_num - right_num).abs();
                    let max_val = left_num.abs().max(right_num.abs()).max(1.0);
                    if diff < max_val * 1e-10 {
                        return AssertResult::Passed;
                    }
                }

                // Structural equality failed - check if either side is symbolic
                // If so, try Z3 verification
                if self.is_symbolic(&left_val) || self.is_symbolic(&right_val) {
                    let equality_expr = Expression::Operation {
                        name: "equals".to_string(),
                        args: vec![left_val.clone(), right_val.clone()],
                        span: None,
                    };

                    if let Some(result) = self.verify_with_z3(&equality_expr) {
                        return result;
                    }

                    // Z3 couldn't help - return unknown (optimistic)
                    return AssertResult::Unknown(format!(
                        "Symbolic assertion: cannot verify {} = {}",
                        self.expr_summary(&left_val),
                        self.expr_summary(&right_val)
                    ));
                }

                // Both sides evaluated but not equal. Before failing, try Z3
                // as a last resort — handles unreduced operations like
                // xi_QGi(complex(2,0)) where is_symbolic returns false
                // (all args are concrete) but the operation is uninterpreted.
                let equality_expr = Expression::Operation {
                    name: "equals".to_string(),
                    args: vec![left_val.clone(), right_val.clone()],
                    span: None,
                };
                if let Some(result) = self.verify_with_z3(&equality_expr) {
                    return result;
                }

                AssertResult::Failed {
                    expected: Box::new(right_val),
                    actual: Box::new(left_val),
                }
            }
            (Err(_), _) | (_, Err(_)) => {
                // At least one side couldn't be evaluated - try Z3 verification
                let equality_expr = Expression::Operation {
                    name: "equals".to_string(),
                    args: vec![left_resolved.clone(), right_resolved.clone()],
                    span: None,
                };

                if let Some(result) = self.verify_with_z3(&equality_expr) {
                    return result;
                }

                // Z3 couldn't help - return unknown
                AssertResult::Unknown(format!(
                    "Symbolic assertion: {} = {}",
                    self.expr_summary(&left_resolved),
                    self.expr_summary(&right_resolved)
                ))
            }
        }
    }

    /// Check if an expression is symbolic (contains unbound variables)
    pub(crate) fn is_symbolic(&self, expr: &Expression) -> bool {
        match expr {
            Expression::Object(name) => {
                // It's symbolic if not bound and not an ADT constructor
                !self.bindings.contains_key(name) && !self.adt_constructors.contains(name)
            }
            Expression::Const(_) | Expression::String(_) | Expression::Placeholder { .. } => false,
            Expression::Operation { args, .. } => args.iter().any(|arg| self.is_symbolic(arg)),
            Expression::List(elements) => elements.iter().any(|e| self.is_symbolic(e)),
            Expression::Match {
                scrutinee, cases, ..
            } => {
                self.is_symbolic(scrutinee) || cases.iter().any(|case| self.is_symbolic(&case.body))
            }
            Expression::Conditional {
                condition,
                then_branch,
                else_branch,
                ..
            } => {
                self.is_symbolic(condition)
                    || self.is_symbolic(then_branch)
                    || self.is_symbolic(else_branch)
            }
            Expression::Let { value, body, .. } => {
                self.is_symbolic(value) || self.is_symbolic(body)
            }
            Expression::Lambda { body, .. } => self.is_symbolic(body),
            Expression::Ascription { expr, .. } => self.is_symbolic(expr),
            Expression::Quantifier { body, .. } => self.is_symbolic(body),
        }
    }

    /// Get a short summary of an expression for error messages
    pub(crate) fn expr_summary(&self, expr: &Expression) -> String {
        format!("{:?}", expr).chars().take(40).collect()
    }

    /// Resolve variables in an expression using current bindings
    pub(crate) fn resolve_expression(&self, expr: &Expression) -> Expression {
        match expr {
            Expression::Object(name) => {
                // Check if this variable is bound
                if let Some(value) = self.bindings.get(name) {
                    value.clone()
                } else {
                    expr.clone()
                }
            }
            Expression::Operation { name, args, .. } => Expression::Operation {
                name: name.clone(),
                args: args.iter().map(|a| self.resolve_expression(a)).collect(),
                span: None,
            },
            _ => expr.clone(),
        }
    }

    /// Check if two expressions are structurally equal
    pub(crate) fn expressions_equal(&self, left: &Expression, right: &Expression) -> bool {
        // First, normalize boolean representations
        let left_bool = self.as_boolean(left);
        let right_bool = self.as_boolean(right);
        if let (Some(l), Some(r)) = (left_bool, right_bool) {
            return l == r;
        }

        match (left, right) {
            (Expression::Const(a), Expression::Const(b)) => {
                // Try numeric comparison for constants
                match (a.parse::<f64>(), b.parse::<f64>()) {
                    (Ok(a_num), Ok(b_num)) => (a_num - b_num).abs() < 1e-10,
                    _ => a == b,
                }
            }
            (Expression::String(a), Expression::String(b)) => a == b,
            (Expression::Object(a), Expression::Object(b)) => a == b,
            // Handle Const/Object cross-comparison for identical strings
            (Expression::Const(a), Expression::Object(b))
            | (Expression::Object(a), Expression::Const(b)) => a == b,
            (
                Expression::Operation {
                    name: n1, args: a1, ..
                },
                Expression::Operation {
                    name: n2, args: a2, ..
                },
            ) => {
                n1 == n2
                    && a1.len() == a2.len()
                    && a1
                        .iter()
                        .zip(a2.iter())
                        .all(|(x, y)| self.expressions_equal(x, y))
            }
            _ => left == right, // Fall back to Eq trait
        }
    }

    /// Try to interpret an expression as a boolean value
    pub(crate) fn as_boolean(&self, expr: &Expression) -> Option<bool> {
        match expr {
            Expression::Const(s) | Expression::Object(s) => match s.to_lowercase().as_str() {
                "true" => Some(true),
                "false" => Some(false),
                _ => None,
            },
            _ => None,
        }
    }

    /// Check if an expression represents a "truthy" value
    pub(crate) fn is_truthy(&self, expr: &Expression) -> bool {
        match expr {
            Expression::Object(name) => {
                // Common truth values
                name == "true" || name == "True" || name == "⊤"
            }
            Expression::Const(s) => {
                // Non-zero numbers are truthy
                s.parse::<f64>().map(|n| n != 0.0).unwrap_or(false)
            }
            _ => false,
        }
    }

    /// Run all example blocks in a program
    ///
    /// Returns a vector of results for each example block
    pub fn run_all_examples(&mut self, program: &Program) -> Vec<ExampleResult> {
        let mut results = Vec::new();

        for item in &program.items {
            if let TopLevel::ExampleBlock(example) = item {
                let result = self.eval_example_block(example);
                results.push(result);
            }
        }

        results
    }
}
