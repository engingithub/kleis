//! Z3 Backend Implementation
//!
//! Implements the SolverBackend trait for Z3 SMT solver.
//!
//! This is extracted and refactored from axiom_verifier.rs to fit the new
//! pluggable solver architecture.
//!
//! **Key Features:**
//! - Incremental solving (push/pop for efficiency)
//! - Smart axiom loading (on-demand, with dependency analysis)
//! - Mixed type handling (Int/Real conversions)
//! - Uninterpreted functions for unknown operations
//!
//! **Critical:** All public methods return Kleis Expression, not Z3 types!

use crate::ast::{Expression, MatchCase, Pattern, QuantifiedVar, QuantifierKind};
use crate::evaluator::Evaluator;
use crate::kleis_ast::TypeExpr;
use crate::solvers::backend::{
    SatisfiabilityResult, SolverBackend, SolverStats, VerificationResult, Witness,
};
use crate::solvers::capabilities::SolverCapabilities;
use crate::solvers::result_converter::ResultConverter;
use crate::solvers::z3::converter::Z3ResultConverter;
use crate::solvers::z3::translators::{arithmetic, boolean, comparison};
use crate::solvers::z3::type_mapping::{
    get_builtin_sort_kind, get_parameterized_sort_name, get_type_dispatch_info,
};
use crate::structure_registry::StructureRegistry;
use crate::type_inference::Type;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use z3::ast::{Ast, Bool, Dynamic, Int, Real};
use z3::{DatatypeAccessor, DatatypeBuilder, DatatypeSort, FuncDecl, SatResult, Solver, Sort};

/// Memory limit for the Z3 watchdog (bytes). Set once in Z3Backend::new(),
/// read by solver_check_with_watchdog on every poll tick. 0 = no memory watchdog.
static Z3_MEMORY_LIMIT_BYTES: AtomicU64 = AtomicU64::new(0);

/// Return the current Z3 memory limit in bytes (0 = unlimited).
/// Used by axiom_verifier for proactive memory checks before calling into Z3.
pub fn z3_memory_limit_bytes() -> u64 {
    Z3_MEMORY_LIMIT_BYTES.load(Ordering::Relaxed)
}

/// Run `solver.check()` with a hard wall-clock watchdog and memory monitor.
///
/// Z3's internal timeout can fail to trigger during complex quantifier
/// reasoning or simplification. This spawns a scoped watchdog thread that
/// calls `ContextHandle::interrupt()` after `wall_timeout` elapses OR when
/// Z3's estimated memory allocation exceeds `Z3_MEMORY_LIMIT_BYTES`,
/// guaranteeing the call returns (with `SatResult::Unknown`, reason "canceled").
fn solver_check_with_watchdog(solver: &Solver, wall_timeout: std::time::Duration) -> SatResult {
    let ctx = z3::Context::thread_local();
    let handle = ctx.handle(); // ContextHandle is Send + Sync
    let finished = Arc::new(AtomicBool::new(false));
    let finished_clone = Arc::clone(&finished);
    let mem_limit = Z3_MEMORY_LIMIT_BYTES.load(Ordering::Relaxed);

    std::thread::scope(|s| {
        s.spawn(move || {
            let poll_interval = std::time::Duration::from_millis(100);
            let start = std::time::Instant::now();
            while start.elapsed() < wall_timeout {
                if finished_clone.load(Ordering::Relaxed) {
                    return;
                }
                if mem_limit > 0 {
                    let used = z3::get_estimated_alloc_size();
                    if used > mem_limit {
                        eprintln!(
                            "   ⚠️  Z3 memory watchdog: {}MB used > {}MB limit, interrupting solver",
                            used / (1024 * 1024),
                            mem_limit / (1024 * 1024),
                        );
                        handle.interrupt();
                        return;
                    }
                }
                std::thread::sleep(poll_interval);
            }
            if !finished_clone.load(Ordering::Relaxed) {
                handle.interrupt();
            }
        });

        let result = solver.check();
        finished.store(true, Ordering::Relaxed);
        result
    })
}

/// Z3 SMT Solver Backend
///
/// Wraps Z3's SMT solver to implement the SolverBackend trait.
/// Maintains long-lived solver state and loads axioms on-demand.
pub struct Z3Backend<'r> {
    /// Z3 solver instance (long-lived for incremental solving)
    solver: Solver,

    /// Structure registry (source of axioms and operations)
    /// Used for axiom loading, operation lookup, data types, and type aliases
    registry: &'r StructureRegistry,

    /// Capability manifest (loaded from capabilities.toml)
    capabilities: SolverCapabilities,

    /// Cached sort signature for each uninterpreted operation: (domain sorts, range sort).
    /// The first declaration fixes the signature; subsequent calls re-create
    /// the FuncDecl from the cached sorts, which Z3 interns to the same object.
    /// This enforces sort consistency: a second call with different argument
    /// sorts will produce a mismatch error instead of silently creating a
    /// different overloaded declaration.
    declared_ops: HashMap<String, (Vec<Sort>, Sort)>,

    /// Track which structures' axioms are currently loaded
    loaded_structures: HashSet<String>,

    /// Identity elements (zero, one, e, etc.) mapped to Z3 constants.
    /// This is the global (unscoped) map: first registration wins.
    /// Used as fallback when no structure scope is active.
    identity_elements: HashMap<String, Dynamic>,

    /// Per-structure identity elements: structure_name → (element_name → Z3 constant).
    /// Each structure gets its own independent Z3 constant for each element,
    /// preventing name collisions when different structures declare `element n : ℝ`.
    structure_elements: HashMap<String, HashMap<String, Dynamic>>,

    /// Tracks which structure first registered each global identity element name.
    /// Used for collision warnings.
    identity_element_owners: HashMap<String, String>,

    /// Current structure scope for element loading and axiom translation.
    /// When set, `kleis_to_z3` resolves bare names against this structure's
    /// scoped elements first, falling back to the global map.
    current_structure_scope: Option<String>,

    /// Free variables auto-created from undefined Object names
    free_variables: HashMap<String, Dynamic>,

    /// Result converter (Z3 Dynamic → Kleis Expression)
    converter: Z3ResultConverter,

    /// Complex number datatype for hybrid translation
    /// Enables concrete complex arithmetic: complex(1,2) + complex(3,4) = complex(4,6)
    complex_datatype: Option<ComplexDatatype>,

    /// Registry-loaded data types as Z3 ADTs
    /// Maps data type name (e.g., "Channel") to its Z3 DatatypeSort
    /// Enables automatic constructor distinctness and exhaustiveness
    declared_data_types: HashMap<String, DatatypeSort>,

    /// When true, cons/nil use the "List" ADT from `declared_data_types`
    /// for native injectivity and distinctness.
    list_adt_enabled: bool,

    /// Warnings collected during translation (e.g., unknown types, duplicate operations)
    /// These are surfaced when verification fails to help diagnose issues
    warnings: Vec<String>,

    /// Inferred types from TypeChecker (optional)
    /// Maps expression signatures to their inferred types
    /// Used for type-dispatched operations (e.g., plus on Matrix vs plus on Real)
    inferred_types: Option<HashMap<String, crate::type_inference::Type>>,

    /// Quantifier variable tracking for witness extraction.
    ///
    /// During `translate_quantifier`, we save (Kleis name, Z3 Dynamic) pairs.
    /// After `get_model()`, we use `model.eval()` on each Z3 variable
    /// and `Z3ResultConverter` to produce structured `Witness` bindings.
    ///
    /// Cleared before each `verify_axiom` / `check_satisfiability` call
    /// and populated during the `kleis_to_z3` translation pass.
    quantifier_vars: Vec<(String, Dynamic)>,

    /// Set to true when Z3 reports `memout`. Once set, all subsequent
    /// verify/check calls return Unknown immediately without calling into Z3,
    /// preventing panics from null pointer returns on exhausted allocators.
    memout: bool,
}

/// Complex number Z3 datatype
/// Stores the DatatypeSort which contains constructor and accessors
struct ComplexDatatype {
    /// The Complex sort (contains constructor and accessors)
    sort: DatatypeSort,
}

impl ComplexDatatype {
    /// Get the constructor: mk_complex(re: Real, im: Real) -> Complex
    fn constructor(&self) -> &FuncDecl {
        &self.sort.variants[0].constructor
    }

    /// Get the real part accessor
    fn accessor_re(&self) -> &FuncDecl {
        &self.sort.variants[0].accessors[0]
    }

    /// Get the imaginary part accessor
    fn accessor_im(&self) -> &FuncDecl {
        &self.sort.variants[0].accessors[1]
    }

    /// Get the Z3 Sort for Complex numbers
    #[allow(dead_code)]
    fn sort(&self) -> &Sort {
        &self.sort.sort
    }
}

impl<'r> Z3Backend<'r> {
    /// Helper function to convert a Dynamic to a Set
    /// Z3's .as_set() may fail on dynamically-created set constants,
    /// so we use a fallback that checks the sort kind.
    fn dynamic_to_set(d: &Dynamic) -> Option<z3::ast::Set> {
        // First try the standard conversion
        if let Some(s) = d.as_set() {
            return Some(s);
        }

        // Fallback: as_set() can fail on dynamically-created set constants
        // even when the underlying sort is correct. Verify both that the sort
        // is Array AND that the range sort is Bool (Z3 represents sets as
        // Array<Element, Bool>). Without the range check, a plain
        // Array<Int, Int> would be unsoundly wrapped as a Set.
        use z3::SortKind;
        let sort = d.get_sort();
        if sort.kind() == SortKind::Array
            && let Some(range) = sort.array_range()
            && range.kind() == SortKind::Bool
        {
            // SAFETY: sort is Array with Bool range, which is Z3's Set representation
            let ctx = &z3::Context::thread_local();
            return unsafe { Some(z3::ast::Set::wrap(ctx, d.get_z3_ast())) };
        }
        None
    }

    /// Helper function to convert a Dynamic to a String
    /// Z3's .as_string() may fail on dynamically-created string constants,
    /// so we use a fallback that checks the sort kind.
    fn dynamic_to_string(d: &Dynamic) -> Option<z3::ast::String> {
        // First try the standard conversion
        if let Some(s) = d.as_string() {
            return Some(s);
        }

        // Fallback: as_string() can fail on dynamically-created string constants.
        // Use Z3_is_string_sort (not just SortKind::Seq) to distinguish String
        // from other Seq sorts like Seq<Int>.
        let sort = d.get_sort();
        if sort.is_string() {
            // SAFETY: sort is verified as Z3's string sort via Z3_is_string_sort
            let ctx = &z3::Context::thread_local();
            unsafe { Some(z3::ast::String::wrap(ctx, d.get_z3_ast())) }
        } else {
            None
        }
    }

    /// Helper function to convert a Dynamic to a BV (bitvector)
    /// Z3's .as_bv() may fail on dynamically-created bitvector constants,
    /// so we use a fallback that checks the sort kind.
    fn dynamic_to_bv(d: &Dynamic) -> Option<z3::ast::BV> {
        // First try the standard conversion
        if let Some(bv) = d.as_bv() {
            return Some(bv);
        }

        // Fallback: check if the sort is a bitvector sort
        use z3::SortKind;
        let sort = d.get_sort();
        if sort.kind() == SortKind::BV {
            let ctx = &z3::Context::thread_local();
            unsafe { Some(z3::ast::BV::wrap(ctx, d.get_z3_ast())) }
        } else {
            None
        }
    }

    /// Helper function to convert a Dynamic to a Regexp.
    /// Z3's Regexp sort is used for regular expression AST nodes.
    /// We check the sort kind to ensure the Dynamic actually wraps a regex.
    fn dynamic_to_regexp(d: &Dynamic) -> Option<z3::ast::Regexp> {
        use z3::SortKind;
        let sort = d.get_sort();
        // Z3 regex sort kind is Re
        if sort.kind() == SortKind::RE {
            let ctx = &z3::Context::thread_local();
            unsafe { Some(z3::ast::Regexp::wrap(ctx, d.get_z3_ast())) }
        } else {
            None
        }
    }

    /// Convert a numeric string (integer, decimal, or scientific notation) to
    /// exact numerator/denominator strings suitable for `z3::ast::Real::from_rational_str`.
    ///
    /// Examples:
    ///   "3.14"     -> ("314", "100")
    ///   "5"        -> ("5", "1")
    ///   "1.5e3"    -> ("1500", "1")
    ///   "6.674e-11"-> ("6674", "100000000000000")
    fn decimal_to_rational_strings(s: &str) -> (String, String) {
        let lower = s.to_ascii_lowercase();
        if let Some(e_pos) = lower.find('e') {
            let mantissa = &s[..e_pos];
            let exp: i32 = s[e_pos + 1..].parse().unwrap_or(0);

            let (digits, decimal_places) = if let Some(dot_pos) = mantissa.find('.') {
                let dec = mantissa.len() - dot_pos - 1;
                let d = mantissa.replace('.', "");
                (d, dec as i32)
            } else {
                (mantissa.to_string(), 0)
            };

            // Effective power of 10: exponent shifts the decimal point
            let net_exp = exp - decimal_places;
            if net_exp >= 0 {
                let num = format!("{}{}", digits, "0".repeat(net_exp as usize));
                (num, "1".to_string())
            } else {
                let den = format!("1{}", "0".repeat((-net_exp) as usize));
                (digits, den)
            }
        } else if let Some(dot_pos) = s.find('.') {
            let decimals = s.len() - dot_pos - 1;
            let num = s.replace('.', "");
            let den = format!("1{}", "0".repeat(decimals));
            (num, den)
        } else {
            (s.to_string(), "1".to_string())
        }
    }

    /// Create a new Z3 backend
    ///
    /// # Arguments
    /// * `registry` - Structure registry containing operations and axioms
    ///
    /// # Axiom Loading
    /// Axioms are loaded from stdlib/*.kleis files via assert_axioms_from_registry().
    /// Call this method after creating the backend to load all axioms before verification.
    pub fn new(registry: &'r StructureRegistry) -> Result<Self, String> {
        let z3_debug = std::env::var("KLEIS_Z3_DEBUG").unwrap_or_default() == "1";

        // Read timeout from env (default 0 = no timeout).
        // Z3 can crash (internal assertion violation in smt_context.cpp)
        // when a global timeout fires mid-processing of complex quantifiers.
        // Only set for debugging divergence; the watchdog is the safe timeout.
        let timeout_ms: u32 = std::env::var("KLEIS_Z3_TIMEOUT_MS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);

        // Read resource limit from env (default 0 = unlimited; set to e.g. 5000000
        // to cap Z3 work units deterministically when debugging divergence)
        let rlimit: u32 = std::env::var("KLEIS_Z3_RLIMIT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);

        // Memory limit in MB. Default 2048 (2GB).
        //
        // IMPORTANT: Like the timeout case, we do NOT set Z3's internal
        // `memory_max_size` parameter. Z3's internal OOM handler throws a
        // C++ `out_of_memory_error` that aborts the process — same class of
        // bug as Z3's internal timeout causing assertion violations.
        //
        // Instead, memory is enforced externally:
        //   1. Watchdog thread polls Z3_get_estimated_alloc_size() during
        //      solver.check() and interrupts via ContextHandle if exceeded
        //   2. Proactive checks in axiom_verifier bail before calling into Z3
        //
        // Set KLEIS_Z3_MEMORY_MB=0 to disable the limit entirely.
        let memory_mb: u32 = std::env::var("KLEIS_Z3_MEMORY_MB")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(2048);

        // Only set global Z3 params when explicitly configured.
        // Z3 can crash internally when a timeout fires mid-processing, so
        // we rely on the watchdog for wall-clock safety by default.
        if timeout_ms > 0 {
            z3::set_global_param("timeout", &timeout_ms.to_string());
            z3::set_global_param("soft_timeout", &timeout_ms.to_string());
        }
        if rlimit > 0 {
            z3::set_global_param("rlimit", &rlimit.to_string());
        }
        // Two-layer memory enforcement (mirrors the timeout architecture):
        //
        // Layer 1 (primary): External monitoring
        //   - Watchdog thread polls Z3_get_estimated_alloc_size() during
        //     solver.check() and interrupts via ContextHandle if exceeded
        //   - Proactive checks in axiom_verifier bail before calling into Z3
        //
        // Layer 2 (backstop): Z3's internal memory_max_size
        //   - When hit, Z3 returns null from API calls instead of allocating
        //   - The vendored z3 crate handles null returns with clean process
        //     exit (no panic → no unwinding → no C++ abort)
        //   - Set slightly above the external limit to give Layer 1 priority
        if memory_mb > 0 {
            let backstop_mb = memory_mb.saturating_add(memory_mb / 4); // +25% headroom
            z3::set_global_param("memory_max_size", &backstop_mb.to_string());
            Z3_MEMORY_LIMIT_BYTES.store((memory_mb as u64) * 1024 * 1024, Ordering::Relaxed);
        } else {
            Z3_MEMORY_LIMIT_BYTES.store(0, Ordering::Relaxed);
        }

        if z3_debug {
            eprintln!("[Z3 DEBUG] ===== Z3 debug mode enabled =====");
            eprintln!(
                "[Z3 DEBUG] timeout={}ms  rlimit={}  memory={}MB  soft_timeout={}ms",
                timeout_ms, rlimit, memory_mb, timeout_ms
            );

            z3::set_global_param("smt.qi.profile", "true");
            z3::set_global_param("smt.qi.profile_freq", "500");
            z3::set_global_param("trace", "true");
        }

        // Create a properly configured context and install it as thread-local
        let mut cfg = z3::Config::new();
        cfg.set_model_generation(true);
        if timeout_ms > 0 {
            cfg.set_param_value("timeout", &timeout_ms.to_string());
        }
        if rlimit > 0 {
            cfg.set_param_value("rlimit", &rlimit.to_string());
        }
        if z3_debug {
            cfg.set_param_value("stats", "true");
            cfg.set_param_value("trace", "true");
        }
        let ctx = z3::Context::new(&cfg);
        z3::Context::set_thread_local(&ctx);

        if z3_debug {
            eprintln!(
                "[Z3 DEBUG] Custom Context installed (model=true, rlimit={}, timeout={})",
                rlimit, timeout_ms
            );
        }

        // Create solver (uses thread-local Context)
        let solver = Solver::new();

        // Solver-specific params (belt-and-suspenders with context-level settings)
        let mut params = z3::Params::new();
        params.set_u32("timeout", timeout_ms);
        params.set_u32("solver2_timeout", timeout_ms);
        solver.set_params(&params);

        let capabilities = super::load_capabilities()?;

        // Create Complex number datatype: Complex = mk_complex(re: Real, im: Real)
        let complex_dt = DatatypeBuilder::new("Complex")
            .variant(
                "mk_complex",
                vec![
                    ("re", DatatypeAccessor::sort(Sort::real())),
                    ("im", DatatypeAccessor::sort(Sort::real())),
                ],
            )
            .finish();

        let complex_datatype = ComplexDatatype { sort: complex_dt };

        let mut backend = Self {
            solver,
            registry,
            capabilities,
            declared_ops: HashMap::new(),
            loaded_structures: HashSet::new(),
            identity_elements: HashMap::new(),
            structure_elements: HashMap::new(),
            identity_element_owners: HashMap::new(),
            current_structure_scope: None,
            free_variables: HashMap::new(),
            converter: Z3ResultConverter,
            complex_datatype: Some(complex_datatype),
            declared_data_types: HashMap::new(),
            list_adt_enabled: false,
            warnings: Vec::new(),
            inferred_types: None,
            quantifier_vars: Vec::new(),
            memout: false,
        };

        // Initialize complex number constant 'i' as complex(0, 1)
        // This is now a concrete value, not an uninterpreted constant!
        backend.initialize_complex_i();

        Ok(backend)
    }

    /// Set inferred types from TypeChecker
    /// Call this after type checking but before verification
    /// to enable type-dispatched operations (e.g., matrix addition vs scalar addition)
    pub fn set_inferred_types(&mut self, types: HashMap<String, crate::type_inference::Type>) {
        self.inferred_types = Some(types);
    }

    /// Try type-dispatched operation handling
    ///
    /// This enables using the correct Z3 operations based on type information:
    /// - User-defined types (Matrix, Tensor, etc.) → uses uninterpreted functions
    /// - Built-in types (Real, Int, Bool) → uses Z3's built-in operations
    ///
    /// The dispatch logic is delegated to `type_mapping::get_type_dispatch_info`
    /// which is the SINGLE PLACE where type → operation mappings are defined.
    ///
    /// Returns Some(result) if type-based dispatch applies, None to fall through
    /// to the default operation handling.
    fn try_type_dispatched_operation(
        &mut self,
        op_name: &str,
        args: &[Expression],
        vars: &HashMap<String, Dynamic>,
    ) -> Result<Option<Dynamic>, String> {
        // Check if we have type information
        let types = match &self.inferred_types {
            Some(t) => t,
            None => return Ok(None), // No type info, fall through
        };

        // Look for type info for the first argument
        let arg_type = self.find_relevant_type_for_args(args, types);

        if let Some(ty) = arg_type {
            // Use type_mapping to determine if dispatch is needed
            if let Some(dispatch_info) = get_type_dispatch_info(op_name, &ty) {
                // Translate arguments
                let z3_args: Result<Vec<_>, _> =
                    args.iter().map(|arg| self.kleis_to_z3(arg, vars)).collect();
                let z3_args = z3_args?;

                // Get result sort for this type
                let result_sort = self.get_sort_for_type(&ty);
                let result = self.create_uninterpreted_call(
                    &dispatch_info.z3_func_name,
                    &z3_args,
                    &result_sort,
                );

                return Ok(Some(result));
            }
        }

        Ok(None) // No type-based dispatch applies
    }

    /// Find relevant type for the operation arguments
    fn find_relevant_type_for_args(
        &self,
        args: &[Expression],
        types: &HashMap<String, Type>,
    ) -> Option<Type> {
        // Strategy 1: Check if any argument is a well-known variable with type info
        for arg in args {
            if let Expression::Object(name) = arg {
                // Check various path patterns that might match
                for (path, ty) in types {
                    if path.contains(name) || path == "root" {
                        return Some(ty.clone());
                    }
                }
            }
        }

        // Strategy 2: Check if "root" type is available
        types.get("root").cloned()
    }

    /// Try to expand sum_over with concrete bounds
    ///
    /// Expands `sum_over(λ i . body, start, end)` into:
    /// `body[i=start] + body[i=start+1] + ... + body[i=end-1]`
    ///
    /// This enables Einstein summation / tensor contraction in Z3.
    ///
    /// Returns:
    /// - `Ok(Some(result))` if expansion succeeded
    /// - `Ok(None)` if bounds are not concrete (fall through to uninterpreted)
    /// - `Err(msg)` if there's an error in translation
    fn try_expand_sum_over(
        &mut self,
        lambda_arg: &Expression,
        start_arg: &Expression,
        end_arg: &Expression,
        vars: &HashMap<String, Dynamic>,
    ) -> Result<Option<Dynamic>, String> {
        // Extract the lambda
        let (param_name, body) = match lambda_arg {
            Expression::Lambda { params, body, .. } if params.len() == 1 => {
                (params[0].name.clone(), body.as_ref())
            }
            _ => return Ok(None), // Not a single-parameter lambda, fall through
        };

        // Extract concrete bounds
        let start = match start_arg {
            Expression::Const(s) => s.parse::<i64>().ok(),
            _ => None,
        };
        let end = match end_arg {
            Expression::Const(s) => s.parse::<i64>().ok(),
            _ => None,
        };

        let (start, end) = match (start, end) {
            (Some(s), Some(e)) => (s, e),
            _ => return Ok(None), // Bounds not concrete, fall through to uninterpreted
        };

        // Validate bounds
        if end <= start {
            // Empty sum = 0
            return Ok(Some(Int::from_i64(0).into()));
        }
        if end - start > 64 {
            // Too many terms, fall back to uninterpreted to avoid explosion
            self.warnings.push(format!(
                "sum_over: range [{}, {}) has {} terms, falling back to uninterpreted",
                start,
                end,
                end - start
            ));
            return Ok(None);
        }

        // Expand the sum
        let mut terms: Vec<Dynamic> = Vec::new();
        for i in start..end {
            // Substitute the loop variable with the concrete index
            let index_expr = Expression::Const(i.to_string());
            let substituted_body = self.substitute_in_expr(body, &param_name, &index_expr);

            // Translate the substituted body to Z3
            let term = self.kleis_to_z3(&substituted_body, vars)?;
            terms.push(term);
        }

        // Sum all terms
        if terms.is_empty() {
            return Ok(Some(Int::from_i64(0).into()));
        }

        let mut result = terms[0].clone();
        for term in &terms[1..] {
            result = arithmetic::translate_plus(&result, term)?;
        }

        Ok(Some(result))
    }

    /// Substitute a variable in an expression with another expression
    fn substitute_in_expr(
        &self,
        expr: &Expression,
        var_name: &str,
        replacement: &Expression,
    ) -> Expression {
        match expr {
            Expression::Object(name) if name == var_name => replacement.clone(),
            Expression::Object(_) => expr.clone(),
            Expression::Const(_) => expr.clone(),
            Expression::String(_) => expr.clone(),
            Expression::Placeholder { .. } => expr.clone(),

            Expression::Operation { name, args, span } => Expression::Operation {
                name: name.clone(),
                args: args
                    .iter()
                    .map(|a| self.substitute_in_expr(a, var_name, replacement))
                    .collect(),
                span: span.clone(),
            },

            Expression::Lambda { params, body, span } => {
                // Don't substitute if the variable is shadowed by a lambda param
                if params.iter().any(|p| p.name == var_name) {
                    expr.clone()
                } else {
                    Expression::Lambda {
                        params: params.clone(),
                        body: Box::new(self.substitute_in_expr(body, var_name, replacement)),
                        span: span.clone(),
                    }
                }
            }

            Expression::Let {
                pattern,
                type_annotation,
                value,
                body,
                span,
            } => {
                // Check if the variable is shadowed by the pattern
                let shadowed = if let Pattern::Variable(pname) = pattern {
                    pname == var_name
                } else {
                    false
                };

                if shadowed {
                    expr.clone()
                } else {
                    Expression::Let {
                        pattern: pattern.clone(),
                        type_annotation: type_annotation.clone(),
                        value: Box::new(self.substitute_in_expr(value, var_name, replacement)),
                        body: Box::new(self.substitute_in_expr(body, var_name, replacement)),
                        span: span.clone(),
                    }
                }
            }

            Expression::Quantifier {
                quantifier,
                variables,
                where_clause,
                body,
            } => {
                // Check if the variable is bound by the quantifier
                let shadowed = variables.iter().any(|v| v.name == var_name);
                if shadowed {
                    expr.clone()
                } else {
                    Expression::Quantifier {
                        quantifier: quantifier.clone(),
                        variables: variables.clone(),
                        where_clause: where_clause
                            .as_ref()
                            .map(|wc| Box::new(self.substitute_in_expr(wc, var_name, replacement))),
                        body: Box::new(self.substitute_in_expr(body, var_name, replacement)),
                    }
                }
            }

            Expression::Conditional {
                condition,
                then_branch,
                else_branch,
                span,
            } => Expression::Conditional {
                condition: Box::new(self.substitute_in_expr(condition, var_name, replacement)),
                then_branch: Box::new(self.substitute_in_expr(then_branch, var_name, replacement)),
                else_branch: Box::new(self.substitute_in_expr(else_branch, var_name, replacement)),
                span: span.clone(),
            },

            Expression::Match {
                scrutinee,
                cases,
                span,
            } => Expression::Match {
                scrutinee: Box::new(self.substitute_in_expr(scrutinee, var_name, replacement)),
                cases: cases
                    .iter()
                    .map(|c| MatchCase {
                        pattern: c.pattern.clone(),
                        guard: c
                            .guard
                            .as_ref()
                            .map(|g| self.substitute_in_expr(g, var_name, replacement)),
                        body: self.substitute_in_expr(&c.body, var_name, replacement),
                    })
                    .collect(),
                span: span.clone(),
            },

            Expression::List(items) => Expression::List(
                items
                    .iter()
                    .map(|item| self.substitute_in_expr(item, var_name, replacement))
                    .collect(),
            ),

            // Ascription: substitute in inner expression
            Expression::Ascription {
                expr: inner,
                type_annotation,
            } => Expression::Ascription {
                expr: Box::new(self.substitute_in_expr(inner, var_name, replacement)),
                type_annotation: type_annotation.clone(),
            },
        }
    }

    /// Get Z3 sort for a Kleis type
    ///
    /// Uses type_mapping for the mapping logic. Handles:
    /// 1. Declared data types from registry → use their Z3 sort
    /// 2. Built-in types (Real, Int, etc.) → use Z3 native sorts
    /// 3. User-defined types → create uninterpreted sort
    fn get_sort_for_type(&self, ty: &Type) -> z3::Sort {
        match ty {
            // Data types - check registry first, then built-ins, then create uninterpreted
            Type::Data {
                type_name, args, ..
            } => {
                // 1. Check if it's a declared data type from registry
                if let Some(dt_sort) = self.declared_data_types.get(type_name) {
                    return dt_sort.sort.clone();
                }

                // 2. Check if it's a built-in type (Real, Int, etc.)
                if let Some(sort_kind) = get_builtin_sort_kind(type_name) {
                    return match sort_kind {
                        "Real" => Sort::real(),
                        "Int" => Sort::int(),
                        "Bool" => Sort::bool(),
                        "String" => Sort::string(),
                        _ => Sort::int(), // Fallback
                    };
                }

                // 3. User-defined type - create uninterpreted sort
                let sort_name = get_parameterized_sort_name(type_name, args);
                Sort::uninterpreted(sort_name.into())
            }

            // Type application - treat like parameterized data type
            Type::App(_, _) => {
                if let Some((base, args)) = Self::flatten_type_app(ty) {
                    let sort_name = get_parameterized_sort_name(&base, &args);
                    Sort::uninterpreted(sort_name.into())
                } else {
                    Sort::int()
                }
            }

            // Primitive types - use Z3 native sorts
            Type::Nat | Type::NatValue(_) | Type::NatExpr(_) => Sort::int(),
            Type::Bool => Sort::bool(),
            Type::String | Type::StringValue(_) => Sort::string(),
            Type::Unit => Sort::bool(),          // Unit ≈ Bool
            Type::Function(_, _) => Sort::int(), // Functions as uninterpreted (conservative)
            Type::Product(_) => Sort::int(),     // Products as uninterpreted
            Type::Var(_) | Type::ForAll(_, _) => Sort::int(), // Type vars default to Int
        }
    }

    fn flatten_type_app(ty: &Type) -> Option<(String, Vec<Type>)> {
        match ty {
            Type::App(func, arg) => {
                let (base, mut args) = Self::flatten_type_app(func)?;
                args.push((**arg).clone());
                Some((base, args))
            }
            Type::Data {
                constructor, args, ..
            } => Some((constructor.clone(), args.clone())),
            _ => None,
        }
    }

    /// Create an uninterpreted function call
    fn create_uninterpreted_call(
        &mut self,
        func_name: &str,
        z3_args: &[Dynamic],
        result_sort: &z3::Sort,
    ) -> Dynamic {
        // Declare the function if not already declared
        let arg_sorts: Vec<_> = z3_args.iter().map(|a| a.get_sort()).collect();
        let arg_sort_refs: Vec<&z3::Sort> = arg_sorts.iter().collect();

        let func_decl = z3::FuncDecl::new(func_name, &arg_sort_refs, result_sort);

        // Apply the function to arguments
        let arg_refs: Vec<&dyn z3::ast::Ast> =
            z3_args.iter().map(|a| a as &dyn z3::ast::Ast).collect();

        func_decl.apply(&arg_refs)
    }

    /// Add a warning message (surfaced when verification fails)
    fn add_warning(&mut self, msg: String) {
        // Deduplicate warnings
        if !self.warnings.contains(&msg) {
            self.warnings.push(msg);
        }
    }

    /// Get all collected warnings
    pub fn get_warnings(&self) -> &[String] {
        &self.warnings
    }

    /// Clear all warnings (e.g., before a new verification)
    pub fn clear_warnings(&mut self) {
        self.warnings.clear();
    }

    /// Format warnings for display
    pub fn format_warnings(&self) -> String {
        if self.warnings.is_empty() {
            String::new()
        } else {
            let mut result = String::from("\n⚠️  Warnings during verification:\n");
            for (i, warning) in self.warnings.iter().enumerate() {
                result.push_str(&format!("  {}. {}\n", i + 1, warning));
            }
            result
        }
    }

    /// Initialize Z3 with all registry data (data types, axioms, etc.)
    ///
    /// Call this after creation to fully initialize Z3 with:
    /// - Data types as Z3 ADTs (automatic constructor distinctness)
    /// - Axioms from structures
    ///
    /// # Example
    /// ```ignore
    /// let mut backend = Z3Backend::new(&registry)?;
    /// backend.initialize_from_registry()?;  // Load everything
    /// ```
    pub fn initialize_from_registry(&mut self) -> Result<(), String> {
        let z3_debug = std::env::var("KLEIS_Z3_DEBUG").unwrap_or_default() == "1";

        // 1. Declare data types first (needed for function sort resolution)
        if z3_debug {
            eprintln!("   [Z3 DEBUG] Step 1: Declaring data types...");
        }
        let _dt_count = self.declare_data_types_from_registry()?;
        if z3_debug {
            eprintln!("   [Z3 DEBUG] Step 1 done: {} data types", _dt_count);
        }

        // 2. Load identity elements from structures (needed for axiom translation)
        if z3_debug {
            eprintln!("   [Z3 DEBUG] Step 2: Loading identity elements...");
        }
        let _id_count = self.load_identity_elements_from_registry()?;
        if z3_debug {
            eprintln!("   [Z3 DEBUG] Step 2 done: {} identity elements", _id_count);
        }

        // 3. Then load axioms
        if z3_debug {
            eprintln!("   [Z3 DEBUG] Step 3: Asserting axioms...");
        }
        let _axiom_count = self.assert_axioms_from_registry()?;
        if z3_debug {
            eprintln!(
                "   [Z3 DEBUG] Step 3 done: {} axioms asserted",
                _axiom_count
            );
        }

        Ok(())
    }

    /// Load all identity elements (nullary operations) from the registry
    ///
    /// Identity elements like `zero : M` are registered with their correct Z3 sort.
    fn load_identity_elements_from_registry(&mut self) -> Result<usize, String> {
        use crate::kleis_ast::TypeExpr;

        let mut count = 0;

        // Collect structures (need to avoid borrow issues)
        let structure_names: Vec<String> = self
            .registry
            .structure_names()
            .iter()
            .map(|s| (*s).clone())
            .collect();

        for structure_name in structure_names {
            if let Some(structure) = self.registry.get(&structure_name) {
                // Collect identity elements from this structure
                let elements: Vec<(String, TypeExpr)> =
                    Self::collect_identity_elements(&structure.members);

                for (name, type_expr) in elements {
                    let sort = self.type_expr_to_sort(&type_expr);
                    let z3_const: Dynamic = Dynamic::fresh_const(&name, &sort);

                    // Always store in per-structure scoped map
                    self.structure_elements
                        .entry(structure_name.clone())
                        .or_default()
                        .insert(name.clone(), z3_const.clone());

                    // Store in global map only if this name hasn't been claimed yet
                    if !self.identity_elements.contains_key(&name) {
                        self.identity_elements.insert(name.clone(), z3_const);
                        self.identity_element_owners
                            .insert(name, structure_name.clone());
                    } else {
                        let owner = self
                            .identity_element_owners
                            .get(&name)
                            .cloned()
                            .unwrap_or_else(|| "<unknown>".to_string());
                        if owner != structure_name {
                            eprintln!(
                                "   ⚠️  Element '{}' in structure '{}' collides with \
                                 same-named element in '{}'. \
                                 Each structure gets an independent Z3 constant.",
                                name, structure_name, owner
                            );
                        }
                    }
                    count += 1;
                }
            }
        }

        Ok(count)
    }

    /// Collect identity elements from structure members (helper function)
    fn collect_identity_elements(
        members: &[crate::kleis_ast::StructureMember],
    ) -> Vec<(String, crate::kleis_ast::TypeExpr)> {
        use crate::kleis_ast::{StructureMember, TypeExpr};

        let mut elements = Vec::new();

        for member in members {
            match member {
                StructureMember::Operation {
                    name,
                    type_signature,
                } => {
                    // Check if nullary (identity element)
                    let is_nullary = !matches!(type_signature, TypeExpr::Function(..));
                    if is_nullary {
                        elements.push((name.clone(), type_signature.clone()));
                    }
                }
                StructureMember::NestedStructure { members, .. } => {
                    // Recursively collect from nested structure
                    elements.extend(Self::collect_identity_elements(members));
                }
                _ => {}
            }
        }

        elements
    }

    /// Assert all axioms from the registry into Z3 solver
    ///
    /// This is the key method for making user-defined axioms available to Z3.
    /// Axioms are translated to Z3 assertions so they can be used for verification.
    ///
    /// # Example
    /// ```ignore
    /// let mut backend = Z3Backend::new(&registry)?;
    /// backend.assert_axioms_from_registry()?;  // Load all axioms
    /// backend.verify_axiom(&theorem)?;          // Now uses loaded axioms
    /// ```
    ///
    /// # Returns
    /// - Ok(count) - number of axioms successfully loaded
    /// - Err if any axiom fails to translate
    pub fn assert_axioms_from_registry(&mut self) -> Result<usize, String> {
        if self.memout {
            return Err("Z3 memory exhausted (memout)".to_string());
        }
        let mut count = 0;
        let empty_vars: HashMap<String, Dynamic> = HashMap::new();

        // Get all structures that have axioms
        let structures_with_axioms: Vec<String> = self
            .registry
            .structures_with_axioms()
            .iter()
            .map(|s| (*s).clone())
            .collect();

        let z3_debug = std::env::var("KLEIS_Z3_DEBUG").unwrap_or_default() == "1";

        for structure_name in structures_with_axioms {
            // Skip if already loaded
            if self.loaded_structures.contains(&structure_name) {
                continue;
            }

            // Skip parameterized (abstract) structures during bulk init:
            // their type parameters are erased to Int, creating unconstrained
            // universal quantifiers that cause Z3 to explode. These can still
            // be loaded on-demand via load_axioms_for_expression() which only
            // picks axioms whose operations match the target expression.
            if let Some(structure) = self.registry.get(&structure_name)
                && !structure.type_params.is_empty()
            {
                if z3_debug {
                    eprintln!(
                        "   [Z3 DEBUG] Skipping parameterized structure '{}' ({} type params) — load on demand",
                        structure_name,
                        structure.type_params.len()
                    );
                }
                continue;
            }

            let axioms = self.registry.get_axioms(&structure_name);
            if z3_debug {
                eprintln!(
                    "   [Z3 DEBUG] Loading structure '{}' ({} axioms)",
                    structure_name,
                    axioms.len()
                );
            }

            // Set structure scope so axiom translation resolves bare element
            // names against this structure's scoped constants.
            self.current_structure_scope = Some(structure_name.clone());

            for (axiom_name, axiom_expr) in axioms {
                if z3_debug {
                    eprintln!("   [Z3 DEBUG]   asserting '{}'...", axiom_name);
                }
                let t = std::time::Instant::now();
                let ops_snapshot = self.declared_ops.clone();
                match self.translate_and_assert_axiom(&axiom_name, axiom_expr, &empty_vars) {
                    Ok(()) => {
                        count += 1;
                        if z3_debug {
                            eprintln!(
                                "   [Z3 DEBUG]   '{}' OK ({}ms)",
                                axiom_name,
                                t.elapsed().as_millis()
                            );
                        }
                    }
                    Err(e) => {
                        self.declared_ops = ops_snapshot;
                        eprintln!("   [Z3 DEBUG]   '{}' FAILED: {}", axiom_name, e);
                    }
                }
            }

            self.current_structure_scope = None;

            if z3_debug {
                eprintln!("   [Z3 DEBUG] ✅ Structure '{}' done", structure_name);
            }
            self.loaded_structures.insert(structure_name);
        }

        Ok(count)
    }

    /// Operations that Z3 translates natively (not as uninterpreted functions).
    /// These are always "known" for dependency analysis because they don't
    /// introduce new uninterpreted symbols.
    ///
    /// TODO: derive this from capabilities.toml instead of hardcoding
    fn z3_builtin_ops() -> HashSet<String> {
        [
            // Equality / comparison
            "equals",
            "eq",
            "neq",
            "not_equals",
            "less_than",
            "lt",
            "greater_than",
            "gt",
            "leq",
            "geq",
            // Boolean
            "and",
            "logical_and",
            "or",
            "logical_or",
            "not",
            "logical_not",
            "implies",
            "iff",
            "biconditional",
            // Arithmetic
            "plus",
            "add",
            "minus",
            "subtract",
            "times",
            "multiply",
            "negate",
            "neg",
            "divide",
            "power",
            "pow",
            "abs",
            "absolute",
            "sqrt",
            "nth_root",
            // Rational
            "rat_add",
            "rat_sub",
            "rat_mul",
            "rat_div",
            "rat_neg",
            "rat_inv",
            "reciprocal",
            "rat_lt",
            "rat_gt",
            "rat_le",
            "rat_ge",
            // List ADT (native when list_adt is enabled)
            "cons",
            "nil",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect()
    }

    /// Extract concrete argument bindings from constructor calls in an expression.
    ///
    /// Walks the expression tree and for each `Operation(name, args)`, records
    /// which argument positions hold `Const` values.  Returns a map:
    ///   `(operation_name, arg_position) → Const value`
    ///
    /// Example: `Matrix(2, 2, [a,b,c,d])` → `{("Matrix",0) → "2", ("Matrix",1) → "2"}`
    fn collect_concrete_args(expr: &Expression) -> HashMap<(String, usize), String> {
        let mut map = HashMap::new();
        Self::collect_concrete_args_recursive(expr, &mut map);
        map
    }

    fn collect_concrete_args_recursive(
        expr: &Expression,
        map: &mut HashMap<(String, usize), String>,
    ) {
        match expr {
            Expression::Operation { name, args, .. } => {
                for (pos, arg) in args.iter().enumerate() {
                    if let Expression::Const(val) = arg {
                        map.insert((name.clone(), pos), val.clone());
                    }
                    Self::collect_concrete_args_recursive(arg, map);
                }
            }
            Expression::List(items) => {
                for item in items {
                    Self::collect_concrete_args_recursive(item, map);
                }
            }
            Expression::Quantifier {
                body, where_clause, ..
            } => {
                Self::collect_concrete_args_recursive(body, map);
                if let Some(wc) = where_clause {
                    Self::collect_concrete_args_recursive(wc, map);
                }
            }
            _ => {}
        }
    }

    /// Ground an axiom by substituting structure parameters with concrete
    /// dimension values inferred from the expression.
    ///
    /// For each `Object("m")` in the axiom that appears at the same
    /// `(operation, position)` where the expression has a `Const`, replace
    /// `Object("m")` → `Const("2")` throughout the axiom.
    fn ground_axiom(
        axiom_expr: &Expression,
        concrete_args: &HashMap<(String, usize), String>,
    ) -> Expression {
        // 1. Find which Object names should be substituted
        let mut subst: HashMap<String, String> = HashMap::new();
        Self::find_groundable_params(axiom_expr, concrete_args, &mut subst);
        if subst.is_empty() {
            return axiom_expr.clone();
        }
        // 2. Apply substitution recursively
        Self::apply_param_subst(axiom_expr, &subst)
    }

    /// Walk the axiom to find Object refs at positions where the expression
    /// had Const values for the same operation.
    fn find_groundable_params(
        expr: &Expression,
        concrete_args: &HashMap<(String, usize), String>,
        subst: &mut HashMap<String, String>,
    ) {
        match expr {
            Expression::Operation { name, args, .. } => {
                for (pos, arg) in args.iter().enumerate() {
                    if let Expression::Object(param_name) = arg
                        && let Some(val) = concrete_args.get(&(name.clone(), pos))
                    {
                        subst.insert(param_name.clone(), val.clone());
                    }
                    Self::find_groundable_params(arg, concrete_args, subst);
                }
            }
            Expression::Quantifier {
                body,
                where_clause,
                variables,
                ..
            } => {
                let bound: HashSet<&str> = variables.iter().map(|v| v.name.as_str()).collect();
                Self::find_groundable_params_excluding(body, concrete_args, subst, &bound);
                if let Some(wc) = where_clause {
                    Self::find_groundable_params_excluding(wc, concrete_args, subst, &bound);
                }
            }
            Expression::List(items) => {
                for item in items {
                    Self::find_groundable_params(item, concrete_args, subst);
                }
            }
            _ => {}
        }
    }

    fn find_groundable_params_excluding(
        expr: &Expression,
        concrete_args: &HashMap<(String, usize), String>,
        subst: &mut HashMap<String, String>,
        bound: &HashSet<&str>,
    ) {
        match expr {
            Expression::Operation { name, args, .. } => {
                for (pos, arg) in args.iter().enumerate() {
                    if let Expression::Object(param_name) = arg
                        && !bound.contains(param_name.as_str())
                        && let Some(val) = concrete_args.get(&(name.clone(), pos))
                    {
                        subst.insert(param_name.clone(), val.clone());
                    }
                    Self::find_groundable_params_excluding(arg, concrete_args, subst, bound);
                }
            }
            Expression::Quantifier {
                body,
                where_clause,
                variables,
                ..
            } => {
                let mut inner_bound = bound.clone();
                for v in variables {
                    inner_bound.insert(v.name.as_str());
                }
                Self::find_groundable_params_excluding(body, concrete_args, subst, &inner_bound);
                if let Some(wc) = where_clause {
                    Self::find_groundable_params_excluding(wc, concrete_args, subst, &inner_bound);
                }
            }
            Expression::List(items) => {
                for item in items {
                    Self::find_groundable_params_excluding(item, concrete_args, subst, bound);
                }
            }
            _ => {}
        }
    }

    /// Replace `Object(name)` → `Const(val)` throughout an expression.
    fn apply_param_subst(expr: &Expression, subst: &HashMap<String, String>) -> Expression {
        match expr {
            Expression::Object(name) => {
                if let Some(val) = subst.get(name) {
                    Expression::Const(val.clone())
                } else {
                    expr.clone()
                }
            }
            Expression::Operation { name, args, span } => Expression::Operation {
                name: name.clone(),
                args: args
                    .iter()
                    .map(|a| Self::apply_param_subst(a, subst))
                    .collect(),
                span: span.clone(),
            },
            Expression::Quantifier {
                quantifier,
                variables,
                where_clause,
                body,
            } => Expression::Quantifier {
                quantifier: quantifier.clone(),
                variables: variables.clone(),
                where_clause: where_clause
                    .as_ref()
                    .map(|wc| Box::new(Self::apply_param_subst(wc, subst))),
                body: Box::new(Self::apply_param_subst(body, subst)),
            },
            Expression::List(items) => Expression::List(
                items
                    .iter()
                    .map(|i| Self::apply_param_subst(i, subst))
                    .collect(),
            ),
            other => other.clone(),
        }
    }

    /// Load axioms relevant to `expr` via transitive dependency closure,
    /// grounding structure parameters with concrete dimensions from `expr`.
    ///
    /// 1. Collect concrete constructor args from the expression
    /// 2. For each registry axiom whose operations are in the known set,
    ///    ground its free parameters using dimension matching, then assert
    /// 3. Repeat until no new axioms qualify (transitive closure)
    pub fn load_axioms_for_expression(&mut self, expr: &Expression) -> Result<usize, String> {
        if self.memout {
            return Err("Z3 memory exhausted (memout)".to_string());
        }

        let mut known_ops = expr.collect_operation_names();
        known_ops.extend(Self::z3_builtin_ops());
        let concrete_args = Self::collect_concrete_args(expr);
        let z3_debug = std::env::var("KLEIS_Z3_DEBUG").unwrap_or_default() == "1";

        // If expression uses cons/nil, enable the List ADT for native
        // injectivity instead of quantified axioms (avoids E-matching divergence)
        if known_ops.contains("cons") || known_ops.contains("nil") {
            self.enable_list_adt();
        }

        if z3_debug {
            let builtins = Self::z3_builtin_ops();
            let domain_ops: Vec<_> = known_ops.difference(&builtins).collect();
            eprintln!(
                "   [Z3 DEBUG] load_axioms_for_expression: domain ops = {:?}",
                domain_ops
            );
            eprintln!(
                "   [Z3 DEBUG] load_axioms_for_expression: concrete args = {:?}",
                concrete_args
            );
        }

        let empty_vars: HashMap<String, Dynamic> = HashMap::new();

        let structures_with_axioms: Vec<String> = self
            .registry
            .structures_with_axioms()
            .iter()
            .map(|s| (*s).clone())
            .collect();

        // Collect (structure, axiom_name, axiom_expr, axiom_ops) once
        let mut candidates: Vec<(String, String, Expression, HashSet<String>)> = Vec::new();
        for structure_name in &structures_with_axioms {
            if self.loaded_structures.contains(structure_name) {
                continue;
            }
            for (axiom_name, axiom_expr) in self.registry.get_axioms(structure_name) {
                let axiom_ops = axiom_expr.collect_operation_names();
                candidates.push((
                    structure_name.clone(),
                    axiom_name,
                    axiom_expr.clone(),
                    axiom_ops,
                ));
            }
        }

        let mut total_loaded = 0;
        let mut loaded_indices: HashSet<usize> = HashSet::new();

        loop {
            let mut progress = false;

            for (i, (struct_name, axiom_name, axiom_expr, axiom_ops)) in
                candidates.iter().enumerate()
            {
                if loaded_indices.contains(&i) {
                    continue;
                }

                if axiom_ops.is_subset(&known_ops) {
                    // Skip injectivity axioms — handled by
                    // decompose_constructor_equalities and List ADT
                    if axiom_name.contains("injective") {
                        if z3_debug {
                            eprintln!(
                                "   [Z3 DEBUG]   skipping '{}::{}' (injectivity handled by decomposition)",
                                struct_name, axiom_name
                            );
                        }
                        loaded_indices.insert(i);
                        continue;
                    }
                    if self.list_adt_enabled && struct_name == "ListConstructor" {
                        if z3_debug {
                            eprintln!(
                                "   [Z3 DEBUG]   skipping '{}::{}' (List ADT provides injectivity)",
                                struct_name, axiom_name
                            );
                        }
                        loaded_indices.insert(i);
                        continue;
                    }

                    // Ground structure parameters with concrete dimensions
                    let grounded = Self::ground_axiom(axiom_expr, &concrete_args);

                    if z3_debug {
                        eprintln!(
                            "   [Z3 DEBUG]   loading '{}::{}' (grounded: {})",
                            struct_name,
                            axiom_name,
                            if &grounded != axiom_expr { "yes" } else { "no" }
                        );
                    }
                    // Snapshot declared_ops so a failed axiom translation
                    // doesn't leave partial declarations with wrong sorts.
                    let ops_snapshot = self.declared_ops.clone();
                    match self.translate_and_assert_axiom(axiom_name, &grounded, &empty_vars) {
                        Ok(()) => {
                            total_loaded += 1;
                            for op in axiom_ops {
                                known_ops.insert(op.clone());
                            }
                        }
                        Err(e) => {
                            self.declared_ops = ops_snapshot;
                            if z3_debug {
                                eprintln!(
                                    "   [Z3 DEBUG]   '{}::{}' FAILED: {}",
                                    struct_name, axiom_name, e
                                );
                            }
                        }
                    }
                    loaded_indices.insert(i);
                    progress = true;
                }
            }

            if !progress {
                break;
            }
        }

        if z3_debug {
            eprintln!(
                "   [Z3 DEBUG] load_axioms_for_expression: loaded {} axioms",
                total_loaded
            );
        }
        Ok(total_loaded)
    }

    // =========================================================================
    // Registry Data Type Integration (ADR-022 Enhanced)
    // =========================================================================

    /// Declare all data types from the registry as Z3 ADTs
    ///
    /// This converts Kleis `data` declarations into Z3 algebraic data types,
    /// enabling automatic constructor distinctness and exhaustiveness checking.
    ///
    /// # Benefits
    /// - **Constructor Distinctness**: Z3 automatically knows `Mass ≠ EM ≠ Spin`
    /// - **Exhaustiveness**: Z3 verifies pattern matching is exhaustive
    /// - **Accessor Functions**: Fields can be accessed in Z3 reasoning
    /// - **No Hardcoding**: User-defined data types get first-class Z3 support
    ///
    /// # Example
    /// ```ignore
    /// // In Kleis: data Channel = Mass | EM | Spin | Color
    /// backend.declare_data_types_from_registry()?;
    /// // Z3 now has: Channel sort with Mass, EM, Spin, Color constructors
    /// // Automatic: Mass ≠ EM, Mass ≠ Spin, etc.
    /// ```
    ///
    /// # Returns
    /// - Ok(count) - number of data types successfully declared
    /// - Err if any data type fails to translate
    pub fn declare_data_types_from_registry(&mut self) -> Result<usize, String> {
        use crate::kleis_ast::TypeExpr;

        // Collect data types from registry
        let data_types: Vec<_> = self.registry.data_types().cloned().collect();

        // Build dependency graph: for each type, which other data types does it reference?
        let mut dependencies: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();
        let mut all_dt_names: std::collections::HashSet<String> = std::collections::HashSet::new();

        for data_def in &data_types {
            all_dt_names.insert(data_def.name.clone());
        }

        for data_def in &data_types {
            let mut deps = Vec::new();
            for variant in &data_def.variants {
                for field in &variant.fields {
                    // Check if field type references another data type
                    if let TypeExpr::Named(name) = &field.type_expr
                        && all_dt_names.contains(name)
                        && name != &data_def.name
                    {
                        deps.push(name.clone());
                    }
                }
            }
            dependencies.insert(data_def.name.clone(), deps);
        }

        // Topological sort - declare types with no dependencies first
        let mut declared: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut ordered: Vec<crate::kleis_ast::DataDef> = Vec::new();
        let mut remaining: Vec<_> = data_types;

        // Simple iterative topological sort
        let max_iterations = remaining.len() + 1;
        for _ in 0..max_iterations {
            let mut made_progress = false;
            let mut still_remaining = Vec::new();

            for data_def in remaining {
                let deps = dependencies
                    .get(&data_def.name)
                    .cloned()
                    .unwrap_or_default();
                let all_deps_satisfied = deps.iter().all(|d| declared.contains(d));

                if all_deps_satisfied {
                    declared.insert(data_def.name.clone());
                    ordered.push(data_def);
                    made_progress = true;
                } else {
                    still_remaining.push(data_def);
                }
            }

            remaining = still_remaining;

            if remaining.is_empty() || !made_progress {
                break;
            }
        }

        // Add any remaining (cyclic dependencies) at the end
        for data_def in remaining {
            ordered.push(data_def);
        }

        // Now declare in order
        let mut count = 0;
        for data_def in ordered {
            // Skip if already declared
            if self.declared_data_types.contains_key(&data_def.name) {
                continue;
            }

            // Build Z3 datatype
            match self.declare_data_type(&data_def) {
                Ok(dt_sort) => {
                    self.declared_data_types
                        .insert(data_def.name.clone(), dt_sort);
                    count += 1;
                }
                Err(e) => {
                    // Log but continue - some data types may use unsupported features
                    eprintln!(
                        "Warning: Could not declare data type '{}': {}",
                        data_def.name, e
                    );
                }
            }
        }

        Ok(count)
    }

    /// Declare a single data type as a Z3 ADT
    ///
    /// Converts a Kleis DataDef into a Z3 DatatypeSort with constructors.
    fn declare_data_type(
        &self,
        data_def: &crate::kleis_ast::DataDef,
    ) -> Result<DatatypeSort, String> {
        let mut builder = DatatypeBuilder::new(data_def.name.as_str());

        for variant in &data_def.variants {
            if variant.fields.is_empty() {
                // Nullary constructor (like True, False, None, Mass, EM)
                builder = builder.variant(variant.name.as_str(), vec![]);
            } else {
                // Constructor with fields
                // We need to store the names so they outlive the accessor_refs slice
                let field_names: Vec<String> = variant
                    .fields
                    .iter()
                    .enumerate()
                    .map(|(i, field)| field.name.clone().unwrap_or_else(|| format!("field_{}", i)))
                    .collect();

                let accessor_refs: Vec<(&str, DatatypeAccessor)> = variant
                    .fields
                    .iter()
                    .zip(field_names.iter())
                    .map(|(field, name)| {
                        let accessor = self.type_expr_to_accessor(&field.type_expr);
                        (name.as_str(), accessor)
                    })
                    .collect();

                builder = builder.variant(variant.name.as_str(), accessor_refs);
            }
        }

        Ok(builder.finish())
    }

    /// Convert a TypeExpr to a Z3 DatatypeAccessor
    ///
    /// Used when building ADT constructors with fields.
    ///
    /// For declared data types, uses `DatatypeAccessor::Datatype` which enables:
    /// - Recursive types (e.g., `data List(T) = Nil | Cons(T, List(T))`)
    /// - Cross-references between data types
    /// - Proper sort matching in Z3
    fn type_expr_to_accessor(&self, type_expr: &crate::kleis_ast::TypeExpr) -> DatatypeAccessor {
        use crate::kleis_ast::TypeExpr;

        match type_expr {
            TypeExpr::Named(name) => self.type_name_to_accessor(name),
            TypeExpr::Parametric(base_name, _) => {
                // Parametric types like Option(T) - check if base is a known type
                self.type_name_to_accessor(base_name)
            }
            TypeExpr::Function(_, _) => {
                // Function types - not directly representable as ADT field
                // Use Int as uninterpreted representation
                DatatypeAccessor::sort(Sort::int())
            }
            TypeExpr::Product(_) => {
                // Product types - would need tuple support in Z3
                DatatypeAccessor::sort(Sort::int())
            }
            TypeExpr::ForAll { body, .. } => {
                // Polymorphic types - use body
                self.type_expr_to_accessor(body)
            }
            TypeExpr::Var(name) => {
                // Type variable - check if it resolves to something known
                self.type_name_to_accessor(name)
            }
            TypeExpr::DimExpr(_) => {
                // Dimension expression - use Int (dimensions are natural numbers)
                DatatypeAccessor::sort(Sort::int())
            }
        }
    }

    /// Convert a type name to a Z3 DatatypeAccessor
    ///
    /// Checks in order:
    /// 1. Built-in primitive types (Bool, Int, Real, Complex, Rational)
    /// 2. Declared data types from registry
    /// 3. Type aliases from registry
    /// 4. Default to Int for unknown types
    fn type_name_to_accessor(&self, name: &str) -> DatatypeAccessor {
        match name {
            // Boolean types
            "Bool" | "Boolean" => DatatypeAccessor::sort(Sort::bool()),

            // Integer types (including naturals)
            "ℤ" | "Int" | "Z" | "Integer" | "ℕ" | "Nat" | "Natural" => {
                DatatypeAccessor::sort(Sort::int())
            }

            // Real types (scalars)
            "ℝ" | "Real" | "R" | "Scalar" => DatatypeAccessor::sort(Sort::real()),

            // Rational types (Z3 Real is actually ℚ)
            "ℚ" | "Rational" | "Q" => DatatypeAccessor::sort(Sort::real()),

            // Complex numbers - use the already-created Complex sort
            "ℂ" | "Complex" | "C" => {
                if let Some(ref cdt) = self.complex_datatype {
                    // cdt.sort is DatatypeSort, cdt.sort.sort is the underlying Sort
                    DatatypeAccessor::sort(cdt.sort.sort.clone())
                } else {
                    // Fallback: if Complex wasn't created, use two reals
                    DatatypeAccessor::sort(Sort::real())
                }
            }

            // Bitvector types - common widths
            // Note: For parametric BitVec(n), we'd need to extract n from the type
            "BitVec8" | "Byte" | "U8" | "I8" => DatatypeAccessor::sort(Sort::bitvector(8)),
            "BitVec16" | "U16" | "I16" => DatatypeAccessor::sort(Sort::bitvector(16)),
            "BitVec32" | "U32" | "I32" | "Word" => DatatypeAccessor::sort(Sort::bitvector(32)),
            "BitVec64" | "U64" | "I64" => DatatypeAccessor::sort(Sort::bitvector(64)),

            // Set types - Z3 sets are arrays from element type to Bool
            // For generic Set, we use Set(Int) as default
            "Set" | "IntSet" => DatatypeAccessor::sort(Sort::set(&Sort::int())),
            "RealSet" => DatatypeAccessor::sort(Sort::set(&Sort::real())),
            "BoolSet" => DatatypeAccessor::sort(Sort::set(&Sort::bool())),

            // String type
            "String" | "Str" => {
                // Z3 has a String sort, but for ADT fields we use Int as placeholder
                DatatypeAccessor::sort(Sort::int())
            }

            // Check if it's a declared data type
            type_name => {
                if let Some(dt_sort) = self.declared_data_types.get(type_name) {
                    // Already declared - use its Sort directly
                    // This is the correct approach for non-mutually-recursive types
                    DatatypeAccessor::sort(dt_sort.sort.clone())
                } else if self.registry.has_data_type(type_name) {
                    // Data type is in registry but not yet declared in Z3
                    // Use DatatypeAccessor::datatype for forward-reference (mutual recursion)
                    // Note: This only works if the referenced type will be in the same batch
                    DatatypeAccessor::datatype(type_name)
                } else if self.registry.has_type_alias(type_name) {
                    // Type alias - resolve and recurse
                    if let Some((_params, underlying)) = self.registry.get_type_alias(type_name) {
                        self.type_expr_to_accessor(underlying)
                    } else {
                        DatatypeAccessor::sort(Sort::int())
                    }
                } else {
                    // Unknown type - use Int as uninterpreted
                    DatatypeAccessor::sort(Sort::int())
                }
            }
        }
    }

    /// Check if a name is a known data type constructor
    ///
    /// Returns the data type name and variant if found.
    pub fn get_data_constructor_info(&self, name: &str) -> Option<(&str, usize)> {
        for (dt_name, dt_sort) in &self.declared_data_types {
            for (i, variant) in dt_sort.variants.iter().enumerate() {
                // Check if the constructor name matches
                // The constructor's name in Z3 matches the variant name we provided
                if variant.constructor.name() == name {
                    return Some((dt_name.as_str(), i));
                }
            }
        }
        None
    }

    /// Get a Z3 constructor function for a data type variant
    ///
    /// Used when translating constructor expressions to Z3.
    pub fn get_data_constructor(&self, type_name: &str, variant_name: &str) -> Option<&FuncDecl> {
        if let Some(dt_sort) = self.declared_data_types.get(type_name) {
            for variant in &dt_sort.variants {
                if variant.constructor.name() == variant_name {
                    return Some(&variant.constructor);
                }
            }
        }
        None
    }

    /// Get the Z3 Sort for a declared data type
    pub fn get_data_type_sort(&self, name: &str) -> Option<&Sort> {
        self.declared_data_types.get(name).map(|dt| &dt.sort)
    }

    /// Get a nullary constructor value as a Z3 Dynamic
    ///
    /// For data types like `data Channel = Mass | EM | Spin | Color`,
    /// this returns the Z3 value for `Mass`, `EM`, etc.
    fn get_nullary_constructor(&self, name: &str) -> Option<Dynamic> {
        // Search through all declared data types for a matching constructor
        for dt_sort in self.declared_data_types.values() {
            for variant in &dt_sort.variants {
                if variant.constructor.name() == name {
                    // Check if it's a nullary constructor (arity 0)
                    if variant.constructor.arity() == 0 {
                        // Apply the constructor with no arguments to get the value
                        let ast_args: Vec<&dyn Ast> = vec![];
                        return Some(variant.constructor.apply(&ast_args));
                    }
                }
            }
        }
        None
    }

    /// Check if a name is a constructor in a declared data type
    ///
    /// Used to avoid loading ADT constructors as separate identity elements.
    fn is_declared_constructor_internal(&self, name: &str) -> bool {
        for dt_sort in self.declared_data_types.values() {
            for variant in &dt_sort.variants {
                if variant.constructor.name() == name {
                    return true;
                }
            }
        }
        false
    }

    // =========================================================================
    // Type Alias Resolution (ADR-022 Enhanced)
    // =========================================================================

    /// Resolve a type alias to its underlying TypeExpr
    ///
    /// Recursively resolves aliases until reaching a non-alias type.
    pub fn resolve_type_alias(
        &self,
        type_expr: &crate::kleis_ast::TypeExpr,
    ) -> crate::kleis_ast::TypeExpr {
        use crate::kleis_ast::TypeExpr;

        match type_expr {
            TypeExpr::Named(name) => {
                // Check if this name is a type alias
                if let Some((params, underlying)) = self.registry.get_type_alias(name) {
                    if params.is_empty() {
                        // Simple alias - recursively resolve
                        self.resolve_type_alias(underlying)
                    } else {
                        // Parameterized alias without args - can't resolve
                        type_expr.clone()
                    }
                } else {
                    // Not an alias
                    type_expr.clone()
                }
            }
            TypeExpr::Parametric(base_name, args) => {
                // Check if base is a parameterized type alias
                if let Some((params, underlying)) = self.registry.get_type_alias(base_name) {
                    if params.len() == args.len() {
                        // Substitute parameters
                        let substituted = self.substitute_type_params(underlying, params, args);
                        // Recursively resolve
                        self.resolve_type_alias(&substituted)
                    } else {
                        // Arity mismatch - keep as is
                        type_expr.clone()
                    }
                } else {
                    // Not an alias, but resolve args
                    TypeExpr::Parametric(
                        base_name.clone(),
                        args.iter().map(|a| self.resolve_type_alias(a)).collect(),
                    )
                }
            }
            TypeExpr::Function(domain, codomain) => TypeExpr::Function(
                Box::new(self.resolve_type_alias(domain)),
                Box::new(self.resolve_type_alias(codomain)),
            ),
            TypeExpr::Product(types) => {
                TypeExpr::Product(types.iter().map(|t| self.resolve_type_alias(t)).collect())
            }
            TypeExpr::Var(name) => {
                // Check if this is a type alias
                if let Some((params, underlying)) = self.registry.get_type_alias(name) {
                    if params.is_empty() {
                        self.resolve_type_alias(underlying)
                    } else {
                        type_expr.clone()
                    }
                } else {
                    type_expr.clone()
                }
            }
            TypeExpr::ForAll { vars, body } => TypeExpr::ForAll {
                vars: vars.clone(),
                body: Box::new(self.resolve_type_alias(body)),
            },
            TypeExpr::DimExpr(_) => type_expr.clone(),
        }
    }

    /// Substitute type parameters in a type expression
    fn substitute_type_params(
        &self,
        type_expr: &crate::kleis_ast::TypeExpr,
        params: &[crate::kleis_ast::TypeAliasParam],
        args: &[crate::kleis_ast::TypeExpr],
    ) -> crate::kleis_ast::TypeExpr {
        use crate::kleis_ast::TypeExpr;

        // Build substitution map
        let subst: HashMap<&str, &TypeExpr> = params
            .iter()
            .zip(args.iter())
            .map(|(p, a)| (p.name.as_str(), a))
            .collect();

        self.apply_type_substitution(type_expr, &subst)
    }

    /// Apply a type substitution to a type expression
    fn apply_type_substitution(
        &self,
        type_expr: &crate::kleis_ast::TypeExpr,
        subst: &HashMap<&str, &crate::kleis_ast::TypeExpr>,
    ) -> crate::kleis_ast::TypeExpr {
        use crate::kleis_ast::TypeExpr;

        match type_expr {
            TypeExpr::Named(name) => {
                if let Some(replacement) = subst.get(name.as_str()) {
                    (*replacement).clone()
                } else {
                    type_expr.clone()
                }
            }
            TypeExpr::Parametric(base, args) => {
                let new_args: Vec<_> = args
                    .iter()
                    .map(|a| self.apply_type_substitution(a, subst))
                    .collect();
                TypeExpr::Parametric(base.clone(), new_args)
            }
            TypeExpr::Function(domain, codomain) => TypeExpr::Function(
                Box::new(self.apply_type_substitution(domain, subst)),
                Box::new(self.apply_type_substitution(codomain, subst)),
            ),
            TypeExpr::Product(types) => TypeExpr::Product(
                types
                    .iter()
                    .map(|t| self.apply_type_substitution(t, subst))
                    .collect(),
            ),
            TypeExpr::Var(name) => {
                if let Some(replacement) = subst.get(name.as_str()) {
                    (*replacement).clone()
                } else {
                    type_expr.clone()
                }
            }
            TypeExpr::ForAll { vars, body } => {
                // Don't substitute bound variables
                let bound: HashSet<&str> = vars.iter().map(|(name, _)| name.as_str()).collect();
                let filtered: HashMap<&str, &TypeExpr> = subst
                    .iter()
                    .filter(|(k, _)| !bound.contains(*k))
                    .map(|(k, v)| (*k, *v))
                    .collect();
                TypeExpr::ForAll {
                    vars: vars.clone(),
                    body: Box::new(self.apply_type_substitution(body, &filtered)),
                }
            }
            TypeExpr::DimExpr(_) => type_expr.clone(),
        }
    }

    // =========================================================================
    // Beta Reduction Integration
    // =========================================================================

    /// Pre-reduce an expression using beta reduction before Z3 translation
    ///
    /// This applies beta reduction to any lambda applications in the expression,
    /// converting `(λ x . x + 1)(5)` to `5 + 1` before Z3 sees it.
    ///
    /// # Why This Matters
    /// Z3 can't directly apply lambda expressions. By reducing them first,
    /// we convert lambda applications into simpler expressions Z3 can verify.
    ///
    /// # Example
    /// ```ignore
    /// let expr = parse_expression("(λ x . x + 1)(5) = 6")?;
    /// let reduced = backend.beta_reduce_expression(&expr)?;
    /// // reduced = "5 + 1 = 6"
    /// backend.check_satisfiability(&reduced)?;
    /// ```
    pub fn beta_reduce_expression(&self, expr: &Expression) -> Result<Expression, String> {
        let evaluator = Evaluator::new();
        evaluator.reduce_to_normal_form(expr)
    }

    /// Check satisfiability with automatic beta reduction
    ///
    /// This is like `check_satisfiability` but first reduces any lambda expressions.
    pub fn check_satisfiability_with_reduction(
        &mut self,
        expr: &Expression,
    ) -> Result<SatisfiabilityResult, String> {
        let reduced = self.beta_reduce_expression(expr)?;
        self.check_satisfiability(&reduced)
    }

    /// Translate a single axiom and assert it into Z3
    fn translate_and_assert_axiom(
        &mut self,
        name: &str,
        expr: &Expression,
        vars: &HashMap<String, Dynamic>,
    ) -> Result<(), String> {
        let start = std::time::Instant::now();

        // Handle quantified axioms (∀ x : T . body)
        if let Expression::Quantifier {
            quantifier,
            variables,
            where_clause,
            body,
            ..
        } = expr
        {
            let t0 = std::time::Instant::now();
            let z3_axiom =
                self.translate_quantifier_as_forall(quantifier, variables, where_clause, body)?;
            let translate_ms = t0.elapsed().as_millis();

            let t1 = std::time::Instant::now();
            self.solver.assert(&z3_axiom);
            let assert_ms = t1.elapsed().as_millis();

            let total_ms = start.elapsed().as_millis();
            if total_ms > 100 {
                eprintln!(
                    "   [Z3 DEBUG] axiom '{}': translate={}ms assert={}ms TOTAL={}ms",
                    name, translate_ms, assert_ms, total_ms
                );
            }
            return Ok(());
        }

        // Non-quantified axiom: translate directly
        let t0 = std::time::Instant::now();
        let z3_expr = self.kleis_to_z3(expr, vars)?;
        let translate_ms = t0.elapsed().as_millis();

        // Must be boolean
        let z3_bool = z3_expr
            .as_bool()
            .ok_or_else(|| format!("Axiom '{}' must be a boolean expression", name))?;

        let t1 = std::time::Instant::now();
        self.solver.assert(&z3_bool);
        let assert_ms = t1.elapsed().as_millis();

        let total_ms = start.elapsed().as_millis();
        if total_ms > 100 {
            eprintln!(
                "   [Z3 DEBUG] axiom '{}': translate={}ms assert={}ms TOTAL={}ms",
                name, translate_ms, assert_ms, total_ms
            );
        }
        Ok(())
    }

    /// Translate a quantified expression to a proper Z3 forall
    ///
    /// This creates an actual Z3 forall constraint, not just the body.
    fn translate_quantifier_as_forall(
        &mut self,
        quantifier: &QuantifierKind,
        variables: &[QuantifiedVar],
        where_clause: &Option<Box<Expression>>,
        body: &Expression,
    ) -> Result<Bool, String> {
        // Create Z3 bound variables
        let mut bound_vars: Vec<Dynamic> = Vec::new();
        let mut var_map: HashMap<String, Dynamic> = HashMap::new();

        for var in variables {
            let z3_var: Dynamic = if let Some(type_annotation) = &var.type_annotation {
                match type_annotation.as_str() {
                    // Boolean types
                    "Bool" | "Boolean" => Bool::fresh_const(&var.name).into(),

                    // Real types
                    "ℝ" | "Real" => Real::fresh_const(&var.name).into(),

                    // Rational types (Z3's Real is actually ℚ)
                    "ℚ" | "Rational" | "Q" => Real::fresh_const(&var.name).into(),

                    // Integer/Natural types
                    "ℤ" | "Int" | "Z" | "Integer" | "ℕ" | "Nat" | "Natural" => {
                        Int::fresh_const(&var.name).into()
                    }

                    // Complex types
                    "ℂ" | "Complex" | "C" => self
                        .fresh_complex_const(&var.name)
                        .unwrap_or_else(|| Int::fresh_const(&var.name).into()),

                    // Bitvector types - common widths
                    "BitVec8" | "Byte" | "U8" | "I8" => {
                        Dynamic::fresh_const(&var.name, &Sort::bitvector(8))
                    }
                    "BitVec16" | "U16" | "I16" => {
                        Dynamic::fresh_const(&var.name, &Sort::bitvector(16))
                    }
                    "BitVec32" | "U32" | "I32" | "Word" => {
                        Dynamic::fresh_const(&var.name, &Sort::bitvector(32))
                    }
                    "BitVec64" | "U64" | "I64" => {
                        Dynamic::fresh_const(&var.name, &Sort::bitvector(64))
                    }

                    // Set types
                    "Set" | "IntSet" => Dynamic::fresh_const(&var.name, &Sort::set(&Sort::int())),
                    "RealSet" => Dynamic::fresh_const(&var.name, &Sort::set(&Sort::real())),
                    "BoolSet" => Dynamic::fresh_const(&var.name, &Sort::set(&Sort::bool())),

                    // String type
                    "String" | "Str" => z3::ast::String::fresh_const(&var.name).into(),

                    type_name => {
                        // Check if it's a declared data type (exact match)
                        if let Some(dt_sort) = self.declared_data_types.get(type_name) {
                            Dynamic::fresh_const(&var.name, &dt_sort.sort)
                        }
                        // Parameterized types: "List(T)" → base "List"
                        else if let Some(base) = type_name.split('(').next() {
                            if let Some(dt_sort) = self.declared_data_types.get(base) {
                                Dynamic::fresh_const(&var.name, &dt_sort.sort)
                            } else {
                                self.add_warning(format!(
                                    "Unknown type '{}' for variable '{}'. Treating as Int.",
                                    type_name, var.name
                                ));
                                Int::fresh_const(&var.name).into()
                            }
                        } else {
                            Int::fresh_const(&var.name).into()
                        }
                    }
                }
            } else {
                Int::fresh_const(&var.name).into()
            };
            bound_vars.push(z3_var.clone());
            // Track for witness extraction: Kleis name → Z3 variable
            self.quantifier_vars
                .push((var.name.clone(), z3_var.clone()));
            var_map.insert(var.name.clone(), z3_var);
        }

        // Translate body
        let body_z3 = self.kleis_to_z3(body, &var_map)?;
        let body_bool = body_z3
            .as_bool()
            .ok_or_else(|| "Quantifier body must be boolean".to_string())?;

        // Handle where clause: where_clause ⟹ body
        let formula = if let Some(condition) = where_clause {
            let condition_z3 = self.kleis_to_z3(condition, &var_map)?;
            let condition_bool = condition_z3
                .as_bool()
                .ok_or_else(|| "Where clause must be boolean".to_string())?;
            condition_bool.implies(&body_bool)
        } else {
            body_bool
        };

        // Create Z3 forall/exists
        let bound_refs: Vec<&dyn Ast> = bound_vars.iter().map(|v| v as &dyn Ast).collect();

        let result = match quantifier {
            QuantifierKind::ForAll => z3::ast::forall_const(&bound_refs, &[], &formula),
            QuantifierKind::Exists => z3::ast::exists_const(&bound_refs, &[], &formula),
        };

        // Convert back to Bool (forall_const returns Bool)
        Ok(result)
    }

    /// Declare a monomorphic List ADT: `cons(head: Int, tail: List) | nil`.
    ///
    /// When enabled, `cons`/`nil` use Z3 algebraic datatype constructors,
    /// giving Z3 native injectivity (`cons(a,x) = cons(b,y) → a=b ∧ x=y`)
    /// and distinctness (`cons(a,x) ≠ nil`) with zero quantifier axioms.
    pub fn enable_list_adt(&mut self) {
        if self.list_adt_enabled {
            return;
        }
        let list_sort = DatatypeBuilder::new("KleisList")
            .variant(
                "cons",
                vec![
                    ("head", DatatypeAccessor::Sort(Sort::int())),
                    ("tail", DatatypeAccessor::Datatype("KleisList".into())),
                ],
            )
            .variant("nil", vec![])
            .finish();

        let z3_debug = std::env::var("KLEIS_Z3_DEBUG").unwrap_or_default() == "1";
        if z3_debug {
            eprintln!(
                "   [Z3 DEBUG] Declared List ADT (KleisList) — cons/nil are now injective constructors"
            );
        }
        self.declared_data_types
            .insert("List".to_string(), list_sort);
        self.list_adt_enabled = true;
    }

    /// Translate a Kleis List to a cons-chain
    ///
    /// [a, b, c] -> cons(a, cons(b, cons(c, nil)))
    ///
    /// When the List ADT is enabled (`enable_list_adt`), uses Z3 datatype
    /// constructors for native injectivity.  Otherwise falls back to
    /// uninterpreted functions.
    fn translate_list_to_cons(
        &mut self,
        items: &[Expression],
        vars: &HashMap<String, Dynamic>,
    ) -> Result<Dynamic, String> {
        if self.list_adt_enabled {
            let list_adt = self
                .declared_data_types
                .get("List")
                .ok_or_else(|| "List ADT enabled but not in declared_data_types".to_string())?;
            if list_adt.variants.len() < 2 {
                return Err("List ADT has fewer than 2 variants (expected cons + nil)".to_string());
            }
            let mut result = list_adt.variants[1].constructor.apply(&[]);
            for item in items.iter().rev() {
                let item_z3 = self.kleis_to_z3(item, vars)?;
                let list_adt = self
                    .declared_data_types
                    .get("List")
                    .ok_or_else(|| "List ADT disappeared during translation".to_string())?;
                result = list_adt.variants[0]
                    .constructor
                    .apply(&[&item_z3 as &dyn Ast, &result as &dyn Ast]);
            }
            Ok(result)
        } else {
            // Uninterpreted function fallback
            let nil_func = self.declare_uninterpreted("nil", 0);
            let mut result = nil_func.apply(&[]);
            for item in items.iter().rev() {
                let item_z3 = self.kleis_to_z3(item, vars)?;
                let cons_func = self.declare_uninterpreted("cons", 2);
                result = cons_func.apply(&[&item_z3 as &dyn Ast, &result as &dyn Ast]);
            }
            Ok(result)
        }
    }

    /// Translate Kleis expression to Z3 Dynamic
    ///
    /// This is the core translation function. It recursively converts
    /// Kleis expressions to Z3's internal representation.
    ///
    /// **Internal only** - results stay within Z3Backend.
    fn kleis_to_z3(
        &mut self,
        expr: &Expression,
        vars: &HashMap<String, Dynamic>,
    ) -> Result<Dynamic, String> {
        match expr {
            Expression::Object(name) => {
                // 1. Check quantified variables (highest priority)
                if let Some(var) = vars.get(name) {
                    return Ok(var.clone());
                }

                // 2. Check structure-scoped identity elements (if a scope is active)
                if let Some(scope) = &self.current_structure_scope
                    && let Some(struct_map) = self.structure_elements.get(scope)
                    && let Some(scoped) = struct_map.get(name)
                {
                    return Ok(scoped.clone());
                }

                // 3. Check global identity elements (fallback)
                if let Some(identity) = self.identity_elements.get(name) {
                    return Ok(identity.clone());
                }

                // DEBUG: Log when we fall through for known identity element names
                if name == "zero" || name == "one" {
                    eprintln!(
                        "DEBUG: '{}' not found in identity_elements. Available: {:?}",
                        name,
                        self.identity_elements.keys().collect::<Vec<_>>()
                    );
                }

                // 3. Check if it's a nullary constructor from a declared data type
                // e.g., Mass, EM, Spin, Color from "data Channel = Mass | EM | Spin | Color"
                if let Some(constructor_z3) = self.get_nullary_constructor(name) {
                    return Ok(constructor_z3);
                }

                // 3.5. Special case: empty_set is a nullary operation that returns the empty set
                if name == "empty_set" || name == "∅" {
                    let int_sort = z3::Sort::int();
                    return Ok(z3::ast::Set::empty(&int_sort).into());
                }

                // 3.6. Special case: empty_string is a nullary operation that returns ""
                if name == "empty_string" || name == "ε" {
                    return Ok(z3::ast::String::from("").into());
                }

                // 3.7. Special case: bv_zero is the all-zeros bitvector (8-bit)
                if name == "bv_zero" {
                    return Ok(z3::ast::BV::from_i64(0, 8).into());
                }

                // 3.8. Special case: bv_ones is the all-ones bitvector (0xFF for 8-bit)
                if name == "bv_ones" {
                    return Ok(z3::ast::BV::from_i64(255, 8).into());
                }

                // 4. Special case: 'i' as the complex imaginary unit
                // Only use complex i if NOT already in free_variables (which means
                // it was used as a loop variable first)
                if name == "i"
                    && !self.free_variables.contains_key("i")
                    && let Some(i_value) = self.get_complex_i()
                {
                    return Ok(i_value);
                }

                // 5. Check already-created free variables
                if let Some(free_var) = self.free_variables.get(name) {
                    return Ok(free_var.clone());
                }

                // 6. Create fresh constant with the correct sort from the registry
                // Look up the declared type so `operation foo : ℝ` produces a Real
                // constant, not an Int.
                let sort = if let Some(sig) = self.registry.get_operation_signature(name).cloned() {
                    let (_args, ret) = self.extract_signature_types(&sig);
                    self.type_expr_to_sort(&ret)
                } else {
                    Sort::int()
                };
                let dynamic = Dynamic::fresh_const(name, &sort);
                self.free_variables.insert(name.clone(), dynamic.clone());
                Ok(dynamic)
            }

            Expression::Const(s) => {
                // Try to parse as integer first, then as real
                if let Ok(n) = s.parse::<i64>() {
                    Ok(Int::from_i64(n).into())
                } else if s.parse::<f64>().is_ok() {
                    // Use from_rational_str to avoid i32 truncation in Z3_mk_real.
                    // Strategy: build exact numerator/denominator strings.
                    let (num_str, den_str) = Self::decimal_to_rational_strings(s);
                    Real::from_rational_str(&num_str, &den_str)
                        .map(|r| r.into())
                        .ok_or_else(|| format!("Cannot convert decimal to Z3 Real: {}", s))
                } else {
                    Err(format!("Cannot convert constant to Z3: {}", s))
                }
            }

            Expression::String(s) => {
                // String literals are converted to Z3 String sort
                // Note: Z3's String sort requires z3::ast::String which can represent string constants
                Ok(z3::ast::String::from(s.clone()).into())
            }

            Expression::Operation { name, args, .. } => {
                // Matrix and tensor operations are handled via axioms from stdlib/*.kleis
                // Use assert_axioms_from_registry() to load them before verification

                // Check if this is a defined function (not just an operation)
                // If so, expand it at the Kleis level before translating to Z3
                if let Some(func_def) = self.registry.get_function(name)
                    && func_def.params.len() == args.len()
                {
                    // Create substitution map: param -> arg
                    let subst: HashMap<String, Expression> = func_def
                        .params
                        .iter()
                        .cloned()
                        .zip(args.iter().cloned())
                        .collect();
                    // Substitute and translate
                    let substituted_body = substitute_expr(&func_def.body, &subst);
                    return self.kleis_to_z3(&substituted_body, vars);
                }

                // Type-dispatched operations: check if we have type info for this operation
                // This enables using matrix_add instead of Z3's + for matrix types
                if let Some(dispatched_result) =
                    self.try_type_dispatched_operation(name, args, vars)?
                {
                    return Ok(dispatched_result);
                }

                // Special case: sum_over with concrete bounds
                // Expands sum_over(λ i . body, start, end) into body[i=start] + body[i=start+1] + ... + body[i=end-1]
                // This enables tensor contraction/Einstein summation in Z3
                if name == "sum_over"
                    && args.len() == 3
                    && let Some(expanded) =
                        self.try_expand_sum_over(&args[0], &args[1], &args[2], vars)?
                {
                    return Ok(expanded);
                }

                // Standard path: translate arguments first
                let z3_args: Result<Vec<_>, _> =
                    args.iter().map(|arg| self.kleis_to_z3(arg, vars)).collect();
                let z3_args = z3_args?;

                // Use modular translators
                self.translate_operation(name, &z3_args)
            }

            Expression::Quantifier {
                quantifier,
                variables,
                where_clause,
                body,
                ..
            } => {
                let bool_result = self.translate_quantifier(
                    quantifier,
                    variables,
                    where_clause.as_ref().map(|b| &**b),
                    body,
                    vars,
                )?;
                Ok(bool_result.into())
            }

            Expression::Conditional {
                condition,
                then_branch,
                else_branch,
                ..
            } => {
                // Translate all three parts
                let cond_z3 = self.kleis_to_z3(condition, vars)?;
                let then_z3 = self.kleis_to_z3(then_branch, vars)?;
                let else_z3 = self.kleis_to_z3(else_branch, vars)?;

                // Convert condition to Bool
                let cond_bool = cond_z3.as_bool().ok_or_else(|| {
                    "Conditional condition must be a boolean expression".to_string()
                })?;

                // Use Z3's ite (if-then-else)
                Ok(boolean::translate_ite(&cond_bool, &then_z3, &else_z3))
            }

            Expression::Let {
                pattern,
                value,
                body,
                ..
            } => {
                // 1. Translate the value expression
                let z3_value = self.kleis_to_z3(value, vars)?;

                // 2. Extend vars with bindings from pattern match
                // Grammar v0.8: Support pattern destructuring
                let mut extended_vars = vars.clone();
                self.bind_pattern_to_z3(pattern, &z3_value, value, &mut extended_vars)?;

                // 3. Translate body with the extended context
                self.kleis_to_z3(body, &extended_vars)
            }

            Expression::Match {
                scrutinee, cases, ..
            } => {
                // Translate match expression to nested ite
                self.translate_match(scrutinee, cases, vars)
            }

            Expression::List(items) => {
                // Convert list to cons-chain: [a, b, c] -> cons(a, cons(b, cons(c, nil)))
                // This allows axioms from stdlib/lists.kleis to work
                self.translate_list_to_cons(items, vars)
            }

            Expression::Ascription { expr, .. } => {
                // Type annotations don't affect Z3 semantics - just translate the inner expression
                self.kleis_to_z3(expr, vars)
            }

            Expression::Lambda { params, body, .. } => {
                // Lambda expressions in Z3 context:
                // Translate the lambda body with parameters bound as fresh Int variables.
                // This allows Z3 to reason about the body symbolically.
                //
                // NOTE: For lambda applications like (λ x . x + 1)(5), use
                // check_satisfiability_with_reduction() which performs beta reduction
                // before Z3 translation, converting it to 5 + 1.
                let mut new_vars = vars.clone();
                for param in params {
                    // Create fresh variable for each lambda parameter
                    // Use type annotation if available, default to Int
                    let z3_var: Dynamic = if let Some(ref ty) = param.type_annotation {
                        match ty.as_str() {
                            // Boolean types
                            "Bool" | "Boolean" => Bool::fresh_const(&param.name).into(),

                            // Real types
                            "ℝ" | "Real" => Real::fresh_const(&param.name).into(),

                            // Rational types (Z3's Real is actually ℚ)
                            "ℚ" | "Rational" | "Q" => Real::fresh_const(&param.name).into(),

                            // Integer/Natural types
                            "ℤ" | "Int" | "Z" | "Integer" | "ℕ" | "Nat" | "Natural" => {
                                Int::fresh_const(&param.name).into()
                            }

                            // Complex types
                            "ℂ" | "Complex" | "C" => self
                                .fresh_complex_const(&param.name)
                                .unwrap_or_else(|| Int::fresh_const(&param.name).into()),

                            // Bitvector types
                            "BitVec8" | "Byte" | "U8" | "I8" => {
                                Dynamic::fresh_const(&param.name, &Sort::bitvector(8))
                            }
                            "BitVec16" | "U16" | "I16" => {
                                Dynamic::fresh_const(&param.name, &Sort::bitvector(16))
                            }
                            "BitVec32" | "U32" | "I32" | "Word" => {
                                Dynamic::fresh_const(&param.name, &Sort::bitvector(32))
                            }
                            "BitVec64" | "U64" | "I64" => {
                                Dynamic::fresh_const(&param.name, &Sort::bitvector(64))
                            }

                            // Set types
                            "Set" | "IntSet" => {
                                Dynamic::fresh_const(&param.name, &Sort::set(&Sort::int()))
                            }
                            "RealSet" => {
                                Dynamic::fresh_const(&param.name, &Sort::set(&Sort::real()))
                            }
                            "BoolSet" => {
                                Dynamic::fresh_const(&param.name, &Sort::set(&Sort::bool()))
                            }

                            // String type
                            "String" | "Str" => z3::ast::String::fresh_const(&param.name).into(),

                            _ => Int::fresh_const(&param.name).into(),
                        }
                    } else {
                        Int::fresh_const(&param.name).into()
                    };
                    new_vars.insert(param.name.clone(), z3_var);
                }
                self.kleis_to_z3(body, &new_vars)
            }

            Expression::Placeholder { .. } => {
                // Placeholders shouldn't reach Z3 - they're for the editor
                Err(
                    "Placeholder expressions cannot be verified - fill in all slots first"
                        .to_string(),
                )
            }
        }
    }

    /// Translate operation using modular translators
    fn translate_operation(&mut self, name: &str, args: &[Dynamic]) -> Result<Dynamic, String> {
        match name {
            // Equality
            "equals" | "eq" => {
                if args.len() != 2 {
                    return Err("equals requires 2 arguments".to_string());
                }
                Ok(comparison::translate_equals(&args[0], &args[1])?.into())
            }

            "neq" | "not_equals" => {
                if args.len() != 2 {
                    return Err("neq requires 2 arguments".to_string());
                }
                Ok(comparison::translate_not_equals(&args[0], &args[1])?.into())
            }

            // Comparisons
            "less_than" | "lt" => {
                if args.len() != 2 {
                    return Err("less_than requires 2 arguments".to_string());
                }
                Ok(comparison::translate_less_than(&args[0], &args[1])?.into())
            }

            "greater_than" | "gt" => {
                if args.len() != 2 {
                    return Err("greater_than requires 2 arguments".to_string());
                }
                Ok(comparison::translate_greater_than(&args[0], &args[1])?.into())
            }

            "leq" => {
                if args.len() != 2 {
                    return Err("leq requires 2 arguments".to_string());
                }
                Ok(comparison::translate_leq(&args[0], &args[1])?.into())
            }

            "geq" => {
                if args.len() != 2 {
                    return Err("geq requires 2 arguments".to_string());
                }
                Ok(comparison::translate_geq(&args[0], &args[1])?.into())
            }

            // Boolean operations
            "and" | "logical_and" => {
                if args.len() != 2 {
                    return Err("and requires 2 arguments".to_string());
                }
                Ok(boolean::translate_and(&args[0], &args[1])?.into())
            }

            "or" | "logical_or" => {
                if args.len() != 2 {
                    return Err("or requires 2 arguments".to_string());
                }
                Ok(boolean::translate_or(&args[0], &args[1])?.into())
            }

            "not" | "logical_not" => {
                if args.len() != 1 {
                    return Err("not requires 1 argument".to_string());
                }
                Ok(boolean::translate_not(&args[0])?.into())
            }

            "implies" => {
                if args.len() != 2 {
                    return Err("implies requires 2 arguments".to_string());
                }
                Ok(boolean::translate_implies(&args[0], &args[1])?.into())
            }

            // Biconditional (iff): A ↔ B is equivalent to (A → B) ∧ (B → A)
            "iff" | "biconditional" | "equiv_bool" => {
                if args.len() != 2 {
                    return Err("iff requires 2 arguments".to_string());
                }
                // A ↔ B = (A → B) ∧ (B → A), which for booleans is A == B
                if let (Some(a), Some(b)) = (args[0].as_bool(), args[1].as_bool()) {
                    // Use Z3's built-in boolean equality for iff
                    #[allow(deprecated)]
                    Ok(a._eq(&b).into())
                } else {
                    Err("iff requires boolean arguments".to_string())
                }
            }

            // Arithmetic operations - including rat_* operations for rationals
            "plus" | "add" | "rat_add" => {
                if args.len() != 2 {
                    return Err("plus requires 2 arguments".to_string());
                }
                arithmetic::translate_plus(&args[0], &args[1])
            }

            "minus" | "subtract" | "rat_sub" => {
                if args.len() != 2 {
                    return Err("minus requires 2 arguments".to_string());
                }
                arithmetic::translate_minus(&args[0], &args[1])
            }

            "times" | "multiply" | "rat_mul" => {
                if args.len() != 2 {
                    return Err("times requires 2 arguments".to_string());
                }
                arithmetic::translate_times(&args[0], &args[1])
            }

            "negate" | "rat_neg" => {
                if args.len() != 1 {
                    return Err("negate requires 1 argument".to_string());
                }
                arithmetic::translate_negate(&args[0])
            }

            "rat_inv" | "reciprocal" => {
                if args.len() != 1 {
                    return Err("rat_inv requires 1 argument".to_string());
                }
                // Division by 1/x: represented as 1/x in Z3
                // NOTE: "inv" is NOT matched here — it's the abstract Group inverse
                // (inv : G → G) and must fall through to uninterpreted function.
                #[allow(deprecated)]
                let one = Real::from_real(1, 1);
                if let Some(r) = args[0].as_real() {
                    Ok(one.div(&r).into())
                } else if let Some(i) = args[0].as_int() {
                    let r = Int::to_real(&i);
                    Ok(one.div(&r).into())
                } else {
                    Err("rat_inv requires a numeric argument".to_string())
                }
            }

            "rat_div" | "divide" => {
                if args.len() != 2 {
                    return Err("rat_div requires 2 arguments".to_string());
                }
                // Translate division as a/b
                if let (Some(a), Some(b)) = (args[0].as_real(), args[1].as_real()) {
                    Ok(a.div(&b).into())
                } else if let (Some(a), Some(b)) = (args[0].as_int(), args[1].as_int()) {
                    let a_real = Int::to_real(&a);
                    let b_real = Int::to_real(&b);
                    Ok(a_real.div(&b_real).into())
                } else {
                    Err("rat_div requires numeric arguments".to_string())
                }
            }

            "rat_lt" => {
                if args.len() != 2 {
                    return Err("rat_lt requires 2 arguments".to_string());
                }
                if let (Some(a), Some(b)) = (args[0].as_real(), args[1].as_real()) {
                    Ok(a.lt(&b).into())
                } else if let (Some(a), Some(b)) = (args[0].as_int(), args[1].as_int()) {
                    Ok(a.lt(&b).into())
                } else {
                    Err("rat_lt requires numeric arguments".to_string())
                }
            }

            "rat_gt" => {
                if args.len() != 2 {
                    return Err("rat_gt requires 2 arguments".to_string());
                }
                if let (Some(a), Some(b)) = (args[0].as_real(), args[1].as_real()) {
                    Ok(a.gt(&b).into())
                } else if let (Some(a), Some(b)) = (args[0].as_int(), args[1].as_int()) {
                    Ok(a.gt(&b).into())
                } else {
                    Err("rat_gt requires numeric arguments".to_string())
                }
            }

            "rat_le" => {
                if args.len() != 2 {
                    return Err("rat_le requires 2 arguments".to_string());
                }
                if let (Some(a), Some(b)) = (args[0].as_real(), args[1].as_real()) {
                    Ok(a.le(&b).into())
                } else if let (Some(a), Some(b)) = (args[0].as_int(), args[1].as_int()) {
                    Ok(a.le(&b).into())
                } else {
                    Err("rat_le requires numeric arguments".to_string())
                }
            }

            "rat_ge" => {
                if args.len() != 2 {
                    return Err("rat_ge requires 2 arguments".to_string());
                }
                if let (Some(a), Some(b)) = (args[0].as_real(), args[1].as_real()) {
                    Ok(a.ge(&b).into())
                } else if let (Some(a), Some(b)) = (args[0].as_int(), args[1].as_int()) {
                    Ok(a.ge(&b).into())
                } else {
                    Err("rat_ge requires numeric arguments".to_string())
                }
            }

            "power" | "pow" | "^" => {
                if args.len() != 2 {
                    return Err("power requires 2 arguments".to_string());
                }
                arithmetic::translate_power(&args[0], &args[1])
            }

            "sqrt" => {
                if args.len() != 1 {
                    return Err("sqrt requires 1 argument".to_string());
                }
                arithmetic::translate_sqrt(&args[0])
            }

            // Derivative operators (Mathematica-style)
            // D(f, x) - partial derivative ∂f/∂x
            // Dt(f, x) - total derivative df/dx
            "D" | "partial" => {
                // D(f, x) or D(f, x, y) for mixed partials
                if args.is_empty() {
                    return Err("D requires at least 1 argument".to_string());
                }
                // Use uninterpreted function - axioms define behavior
                let func_decl = self.declare_uninterpreted("D", args.len());
                let ast_args: Vec<&dyn z3::ast::Ast> =
                    args.iter().map(|a| a as &dyn z3::ast::Ast).collect();
                Ok(func_decl.apply(&ast_args))
            }

            "Dt" | "total" => {
                // Dt(f, x) - total derivative
                if args.len() < 2 {
                    return Err("Dt requires at least 2 arguments".to_string());
                }
                // Use uninterpreted function - axioms define behavior
                let func_decl = self.declare_uninterpreted("Dt", args.len());
                let ast_args: Vec<&dyn z3::ast::Ast> =
                    args.iter().map(|a| a as &dyn z3::ast::Ast).collect();
                Ok(func_decl.apply(&ast_args))
            }

            // Integral operators (Mathematica-style)
            // Integrate(f, x) - indefinite integral ∫f dx
            // Integrate(f, {x, a, b}) - definite integral ∫[a,b] f dx
            "Integrate" | "integral" => {
                if args.is_empty() {
                    return Err("Integrate requires at least 1 argument".to_string());
                }
                let func_decl = self.declare_uninterpreted("Integrate", args.len());
                let ast_args: Vec<&dyn z3::ast::Ast> =
                    args.iter().map(|a| a as &dyn z3::ast::Ast).collect();
                Ok(func_decl.apply(&ast_args))
            }

            // Double integral ∬
            "DoubleIntegral" | "integral2" => {
                if args.is_empty() {
                    return Err("DoubleIntegral requires at least 1 argument".to_string());
                }
                let func_decl = self.declare_uninterpreted("DoubleIntegral", args.len());
                let ast_args: Vec<&dyn z3::ast::Ast> =
                    args.iter().map(|a| a as &dyn z3::ast::Ast).collect();
                Ok(func_decl.apply(&ast_args))
            }

            // Triple integral ∭
            "TripleIntegral" | "integral3" => {
                if args.is_empty() {
                    return Err("TripleIntegral requires at least 1 argument".to_string());
                }
                let func_decl = self.declare_uninterpreted("TripleIntegral", args.len());
                let ast_args: Vec<&dyn z3::ast::Ast> =
                    args.iter().map(|a| a as &dyn z3::ast::Ast).collect();
                Ok(func_decl.apply(&ast_args))
            }

            // Line integral ∮
            "LineIntegral" | "contour" => {
                if args.is_empty() {
                    return Err("LineIntegral requires at least 1 argument".to_string());
                }
                let func_decl = self.declare_uninterpreted("LineIntegral", args.len());
                let ast_args: Vec<&dyn z3::ast::Ast> =
                    args.iter().map(|a| a as &dyn z3::ast::Ast).collect();
                Ok(func_decl.apply(&ast_args))
            }

            // Surface integral ∯
            "SurfaceIntegral" | "surface" => {
                if args.is_empty() {
                    return Err("SurfaceIntegral requires at least 1 argument".to_string());
                }
                let func_decl = self.declare_uninterpreted("SurfaceIntegral", args.len());
                let ast_args: Vec<&dyn z3::ast::Ast> =
                    args.iter().map(|a| a as &dyn z3::ast::Ast).collect();
                Ok(func_decl.apply(&ast_args))
            }

            "abs" | "absolute" => {
                if args.len() != 1 {
                    return Err("abs requires 1 argument".to_string());
                }
                arithmetic::translate_abs(&args[0])
            }

            "neg" => {
                if args.len() != 1 {
                    return Err("neg requires 1 argument".to_string());
                }
                arithmetic::translate_negate(&args[0])
            }

            // Nth root: nth_root(n, x) - uninterpreted for integers
            // (sqrt is already handled above via arithmetic::translate_sqrt)
            "nth_root" => {
                if args.len() != 2 {
                    return Err("nth_root requires 2 arguments (index, radicand)".to_string());
                }
                let func_decl = self.declare_uninterpreted("nth_root", 2);
                let ast_args: Vec<&dyn Ast> = args.iter().map(|d| d as &dyn Ast).collect();
                Ok(func_decl.apply(&ast_args))
            }

            // ============================================
            // STRING OPERATIONS (Grammar v0.8)
            // These use Z3's native string theory (QF_SLIA)
            // ============================================

            // String concatenation: concat("hello", " world") = "hello world"
            "concat" | "str_concat" | "++" => {
                if args.len() < 2 {
                    return Err("concat requires at least 2 arguments".to_string());
                }
                // Convert all args to Z3 strings and concatenate
                let strings: Result<Vec<z3::ast::String>, String> = args
                    .iter()
                    .map(|a| {
                        Self::dynamic_to_string(a)
                            .ok_or_else(|| "concat arguments must be strings".to_string())
                    })
                    .collect();
                let strings = strings?;
                // Use Z3's concat (variadic)
                let refs: Vec<&z3::ast::String> = strings.iter().collect();
                Ok(z3::ast::String::concat(&refs).into())
            }

            // String length: strlen("hello") = 5
            "strlen" | "str_len" | "length" => {
                if args.len() != 1 {
                    return Err("strlen requires 1 argument".to_string());
                }
                let s = Self::dynamic_to_string(&args[0])
                    .ok_or_else(|| "strlen argument must be a string".to_string())?;
                Ok(s.length().into())
            }

            // String contains: contains("hello", "ell") = True
            "contains" | "str_contains" => {
                if args.len() != 2 {
                    return Err("contains requires 2 arguments".to_string());
                }
                let haystack = Self::dynamic_to_string(&args[0])
                    .ok_or_else(|| "contains first argument must be a string".to_string())?;
                let needle = Self::dynamic_to_string(&args[1])
                    .ok_or_else(|| "contains second argument must be a string".to_string())?;
                Ok(haystack.contains(&needle).into())
            }

            // String prefix: hasPrefix("hello", "he") = True
            "hasPrefix" | "str_prefix" | "prefix" => {
                if args.len() != 2 {
                    return Err("hasPrefix requires 2 arguments".to_string());
                }
                let s = Self::dynamic_to_string(&args[0])
                    .ok_or_else(|| "hasPrefix first argument must be a string".to_string())?;
                let pre = Self::dynamic_to_string(&args[1])
                    .ok_or_else(|| "hasPrefix second argument must be a string".to_string())?;
                Ok(pre.prefix(&s).into())
            }

            // String suffix: hasSuffix("hello", "lo") = True
            "hasSuffix" | "str_suffix" | "suffix" => {
                if args.len() != 2 {
                    return Err("hasSuffix requires 2 arguments".to_string());
                }
                let s = Self::dynamic_to_string(&args[0])
                    .ok_or_else(|| "hasSuffix first argument must be a string".to_string())?;
                let suf = Self::dynamic_to_string(&args[1])
                    .ok_or_else(|| "hasSuffix second argument must be a string".to_string())?;
                Ok(suf.suffix(&s).into())
            }

            // ============================================
            // SUBSTRING OPERATIONS
            // ============================================

            // Substring extraction: substr("hello", 1, 3) = "ell"
            "substr" | "substring" => {
                if args.len() != 3 {
                    return Err("substr requires 3 arguments (string, start, length)".to_string());
                }
                let s = Self::dynamic_to_string(&args[0])
                    .ok_or_else(|| "substr first argument must be a string".to_string())?;
                let start = args[1].as_int().ok_or_else(|| {
                    "substr second argument (start) must be an integer".to_string()
                })?;
                let len = args[2].as_int().ok_or_else(|| {
                    "substr third argument (length) must be an integer".to_string()
                })?;
                Ok(s.substr(start, len).into())
            }

            // Find index of substring: indexOf("hello", "ll", 0) = 2
            "indexOf" | "str_indexof" | "indexof" => {
                if args.len() != 3 {
                    return Err(
                        "indexOf requires 3 arguments (haystack, needle, start)".to_string()
                    );
                }
                let haystack = Self::dynamic_to_string(&args[0])
                    .ok_or_else(|| "indexOf first argument must be a string".to_string())?;
                let needle = Self::dynamic_to_string(&args[1])
                    .ok_or_else(|| "indexOf second argument must be a string".to_string())?;
                let start = args[2]
                    .as_int()
                    .ok_or_else(|| "indexOf third argument must be an integer".to_string())?;
                Ok(haystack.index_of(&needle, start).into())
            }

            // Replace first occurrence: replace("hello", "l", "L") = "heLlo"
            "replace" | "str_replace" => {
                if args.len() != 3 {
                    return Err("replace requires 3 arguments (string, old, new)".to_string());
                }
                let s = Self::dynamic_to_string(&args[0])
                    .ok_or_else(|| "replace first argument must be a string".to_string())?;
                let old = Self::dynamic_to_string(&args[1])
                    .ok_or_else(|| "replace second argument must be a string".to_string())?;
                let new_str = Self::dynamic_to_string(&args[2])
                    .ok_or_else(|| "replace third argument must be a string".to_string())?;
                Ok(s.replace(&old, &new_str).into())
            }

            // Get character at index: charAt("hello", 0) = "h"
            // Uses at() which returns the character at the given index as a string
            "charAt" | "str_at" => {
                if args.len() != 2 {
                    return Err("charAt requires 2 arguments (string, index)".to_string());
                }
                let s = Self::dynamic_to_string(&args[0])
                    .ok_or_else(|| "charAt first argument must be a string".to_string())?;
                let idx = args[1]
                    .as_int()
                    .ok_or_else(|| "charAt second argument must be an integer".to_string())?;
                Ok(s.at(idx).into())
            }

            // ============================================
            // STRING-INTEGER CONVERSION
            // ============================================

            // String to integer: strToInt("42") = 42
            "strToInt" | "str_to_int" | "toInt" => {
                if args.len() != 1 {
                    return Err("strToInt requires 1 argument".to_string());
                }
                let s = Self::dynamic_to_string(&args[0])
                    .ok_or_else(|| "strToInt argument must be a string".to_string())?;
                Ok(s.to_int().into())
            }

            // Integer to string: intToStr(42) = "42"
            "intToStr" | "int_to_str" | "fromInt" | "intToString" => {
                if args.len() != 1 {
                    return Err("intToStr requires 1 argument".to_string());
                }
                let n = args[0]
                    .as_int()
                    .ok_or_else(|| "intToStr argument must be an integer".to_string())?;
                Ok(z3::ast::String::from_int(&n).into())
            }

            // ============================================
            // REGULAR EXPRESSION OPERATIONS
            // ============================================
            //
            // Two levels of API:
            //   1. Composable regex constructors (re_literal, re_range, re_star, etc.)
            //      These return Regexp-typed Dynamic values that can be combined.
            //   2. Convenience predicates (isDigits, isAlpha, isAscii, etc.)
            //      These build regexes internally and return Bool.
            //
            // Usage:
            //   matches(s, re_plus(re_range("a", "z")))   — composable
            //   isAscii(s)                                  — convenience
            // ============================================

            // --- String-to-regex matching ---
            // matches(s, re) — check if string s matches regex re
            // Accepts either a composed regex (Dynamic with Re sort) or a
            // literal string pattern (for backward compatibility).
            "matchesRegex" | "matches" | "str_in_re" => {
                if args.len() != 2 {
                    return Err("matches requires 2 arguments (string, regex)".to_string());
                }
                let s = Self::dynamic_to_string(&args[0])
                    .ok_or_else(|| "matches first argument must be a string".to_string())?;
                // Try as composed regex first
                if let Some(re) = Self::dynamic_to_regexp(&args[1]) {
                    Ok(s.regex_matches(&re).into())
                } else if let Some(pattern) = Self::dynamic_to_string(&args[1]) {
                    // Backward compatible: treat string as literal regex
                    if let Some(pattern_str) = pattern.as_string() {
                        let re = z3::ast::Regexp::literal(&pattern_str);
                        Ok(s.regex_matches(&re).into())
                    } else {
                        // Symbolic string — use uninterpreted function
                        let func_decl = self.declare_uninterpreted("matchesRegex", 2);
                        let ast_args: Vec<&dyn Ast> = args.iter().map(|d| d as &dyn Ast).collect();
                        Ok(func_decl.apply(&ast_args))
                    }
                } else {
                    Err("matches second argument must be a regex or string".to_string())
                }
            }

            // --- Regex constructors (composable) ---

            // re_literal(s) — regex matching exactly the string s
            "re_literal" | "re_str" | "str_to_re" => {
                if args.len() != 1 {
                    return Err("re_literal requires 1 argument (string)".to_string());
                }
                let s = Self::dynamic_to_string(&args[0])
                    .ok_or_else(|| "re_literal argument must be a string".to_string())?;
                if let Some(s_str) = s.as_string() {
                    Ok(Dynamic::from_ast(&z3::ast::Regexp::literal(&s_str)))
                } else {
                    Err("re_literal requires a concrete string".to_string())
                }
            }

            // re_range(lo, hi) — character class [lo-hi]
            // lo and hi are single-character strings, e.g. re_range("a", "z")
            "re_range" => {
                if args.len() != 2 {
                    return Err("re_range requires 2 arguments (lo_char, hi_char)".to_string());
                }
                let lo = Self::dynamic_to_string(&args[0])
                    .and_then(|s| s.as_string())
                    .ok_or_else(|| {
                        "re_range first argument must be a single-char string".to_string()
                    })?;
                let hi = Self::dynamic_to_string(&args[1])
                    .and_then(|s| s.as_string())
                    .ok_or_else(|| {
                        "re_range second argument must be a single-char string".to_string()
                    })?;
                let lo_char = lo
                    .chars()
                    .next()
                    .ok_or_else(|| "re_range lo must be a non-empty string".to_string())?;
                let hi_char = hi
                    .chars()
                    .next()
                    .ok_or_else(|| "re_range hi must be a non-empty string".to_string())?;
                Ok(Dynamic::from_ast(&z3::ast::Regexp::range(
                    &lo_char, &hi_char,
                )))
            }

            // re_star(re) — Kleene star: zero or more repetitions
            "re_star" => {
                if args.len() != 1 {
                    return Err("re_star requires 1 argument (regex)".to_string());
                }
                let re = Self::dynamic_to_regexp(&args[0])
                    .ok_or_else(|| "re_star argument must be a regex".to_string())?;
                Ok(Dynamic::from_ast(&re.star()))
            }

            // re_plus(re) — one or more repetitions
            "re_plus" => {
                if args.len() != 1 {
                    return Err("re_plus requires 1 argument (regex)".to_string());
                }
                let re = Self::dynamic_to_regexp(&args[0])
                    .ok_or_else(|| "re_plus argument must be a regex".to_string())?;
                Ok(Dynamic::from_ast(&re.plus()))
            }

            // re_option(re) — optional: zero or one
            "re_option" | "re_opt" => {
                if args.len() != 1 {
                    return Err("re_option requires 1 argument (regex)".to_string());
                }
                let re = Self::dynamic_to_regexp(&args[0])
                    .ok_or_else(|| "re_option argument must be a regex".to_string())?;
                Ok(Dynamic::from_ast(&re.option()))
            }

            // re_concat(re1, re2, ...) — sequence: re1 followed by re2
            "re_concat" => {
                if args.len() < 2 {
                    return Err("re_concat requires at least 2 arguments".to_string());
                }
                let regexes: Result<Vec<z3::ast::Regexp>, String> = args
                    .iter()
                    .map(|a| {
                        Self::dynamic_to_regexp(a)
                            .ok_or_else(|| "re_concat arguments must be regexes".to_string())
                    })
                    .collect();
                let regexes = regexes?;
                let refs: Vec<&z3::ast::Regexp> = regexes.iter().collect();
                Ok(Dynamic::from_ast(&z3::ast::Regexp::concat(&refs)))
            }

            // re_union(re1, re2, ...) — alternation: re1 or re2
            "re_union" => {
                if args.len() < 2 {
                    return Err("re_union requires at least 2 arguments".to_string());
                }
                let regexes: Result<Vec<z3::ast::Regexp>, String> = args
                    .iter()
                    .map(|a| {
                        Self::dynamic_to_regexp(a)
                            .ok_or_else(|| "re_union arguments must be regexes".to_string())
                    })
                    .collect();
                let regexes = regexes?;
                let refs: Vec<&z3::ast::Regexp> = regexes.iter().collect();
                Ok(Dynamic::from_ast(&z3::ast::Regexp::union(&refs)))
            }

            // re_intersect(re1, re2, ...) — intersection: matches both
            "re_intersect" | "re_inter" => {
                if args.len() < 2 {
                    return Err("re_intersect requires at least 2 arguments".to_string());
                }
                let regexes: Result<Vec<z3::ast::Regexp>, String> = args
                    .iter()
                    .map(|a| {
                        Self::dynamic_to_regexp(a)
                            .ok_or_else(|| "re_intersect arguments must be regexes".to_string())
                    })
                    .collect();
                let regexes = regexes?;
                let refs: Vec<&z3::ast::Regexp> = regexes.iter().collect();
                Ok(Dynamic::from_ast(&z3::ast::Regexp::intersect(&refs)))
            }

            // re_complement(re) — matches anything re doesn't
            "re_complement" | "re_comp" => {
                if args.len() != 1 {
                    return Err("re_complement requires 1 argument (regex)".to_string());
                }
                let re = Self::dynamic_to_regexp(&args[0])
                    .ok_or_else(|| "re_complement argument must be a regex".to_string())?;
                Ok(Dynamic::from_ast(&re.complement()))
            }

            // re_full() — matches any string
            "re_full" | "re_all" => Ok(Dynamic::from_ast(&z3::ast::Regexp::full())),

            // re_empty() — matches no string
            "re_empty" | "re_none" => Ok(Dynamic::from_ast(&z3::ast::Regexp::empty())),

            // re_allchar() — matches any single character
            // Use re_range('\x00', '\xff') for portability across Z3 versions
            // (Z3_mk_re_allchar requires Z3 ≥ 4.8.13, not available on all CI runners)
            "re_allchar" | "re_any" => {
                // Use union of two ranges to cover full Latin-1 without multi-byte encoding issues:
                // U+0001..U+007F (ASCII) ∪ U+0080..U+00FF (Latin-1 supplement)
                let ascii = z3::ast::Regexp::range(&'\x01', &'\x7f');
                let latin1 = z3::ast::Regexp::range(&'\u{80}', &'\u{ff}');
                Ok(Dynamic::from_ast(&z3::ast::Regexp::union(&[
                    &ascii, &latin1,
                ])))
            }

            // re_loop(re, lo, hi) — matches re between lo and hi times
            "re_loop" => {
                if args.len() != 3 {
                    return Err("re_loop requires 3 arguments (regex, lo, hi)".to_string());
                }
                let re = Self::dynamic_to_regexp(&args[0])
                    .ok_or_else(|| "re_loop first argument must be a regex".to_string())?;
                let lo = args[1]
                    .as_int()
                    .ok_or_else(|| "re_loop second argument (lo) must be an integer".to_string())?;
                let hi = args[2]
                    .as_int()
                    .ok_or_else(|| "re_loop third argument (hi) must be an integer".to_string())?;
                // Extract concrete values (Z3 loop requires u32)
                let lo_val = lo
                    .as_u64()
                    .ok_or_else(|| "re_loop lo must be a concrete integer".to_string())?
                    as u32;
                let hi_val = hi
                    .as_u64()
                    .ok_or_else(|| "re_loop hi must be a concrete integer".to_string())?
                    as u32;
                Ok(Dynamic::from_ast(&re.r#loop(lo_val, hi_val)))
            }

            // --- Convenience predicates (built-in regex patterns) ---

            // isDigits(s) — string contains only digits [0-9]*
            "isDigits" | "is_digits" => {
                if args.len() != 1 {
                    return Err("isDigits requires 1 argument".to_string());
                }
                let s = Self::dynamic_to_string(&args[0])
                    .ok_or_else(|| "isDigits argument must be a string".to_string())?;
                let digit = z3::ast::Regexp::range(&'0', &'9');
                let digits_re = z3::ast::Regexp::star(&digit);
                Ok(s.regex_matches(&digits_re).into())
            }

            // isAlpha(s) — string contains only letters [a-zA-Z]*
            "isAlpha" | "is_alpha" => {
                if args.len() != 1 {
                    return Err("isAlpha requires 1 argument".to_string());
                }
                let s = Self::dynamic_to_string(&args[0])
                    .ok_or_else(|| "isAlpha argument must be a string".to_string())?;
                let lower = z3::ast::Regexp::range(&'a', &'z');
                let upper = z3::ast::Regexp::range(&'A', &'Z');
                let letter = z3::ast::Regexp::union(&[&lower, &upper]);
                let letters_re = z3::ast::Regexp::star(&letter);
                Ok(s.regex_matches(&letters_re).into())
            }

            // isAlphaNum(s) — string contains only [a-zA-Z0-9]*
            "isAlphaNum" | "is_alphanum" => {
                if args.len() != 1 {
                    return Err("isAlphaNum requires 1 argument".to_string());
                }
                let s = Self::dynamic_to_string(&args[0])
                    .ok_or_else(|| "isAlphaNum argument must be a string".to_string())?;
                let lower = z3::ast::Regexp::range(&'a', &'z');
                let upper = z3::ast::Regexp::range(&'A', &'Z');
                let digit = z3::ast::Regexp::range(&'0', &'9');
                let alphanum = z3::ast::Regexp::union(&[&lower, &upper, &digit]);
                let alphanum_re = z3::ast::Regexp::star(&alphanum);
                Ok(s.regex_matches(&alphanum_re).into())
            }

            // isAscii(s) — string contains only ASCII printable characters
            // ASCII printable range: space (0x20 = ' ') through tilde (0x7E = '~')
            "isAscii" | "is_ascii" | "isAsciiPrintable" | "is_ascii_printable" => {
                if args.len() != 1 {
                    return Err("isAscii requires 1 argument".to_string());
                }
                let s = Self::dynamic_to_string(&args[0])
                    .ok_or_else(|| "isAscii argument must be a string".to_string())?;
                // ASCII printable: [ -~] which is 0x20 to 0x7E
                let ascii_char = z3::ast::Regexp::range(&' ', &'~');
                let ascii_re = z3::ast::Regexp::star(&ascii_char);
                Ok(s.regex_matches(&ascii_re).into())
            }

            // ============================================
            // SET OPERATIONS
            // Uses Z3's native set theory
            // ============================================

            // Empty set: empty_set or builtin_set_empty
            "empty_set" | "builtin_set_empty" | "set_empty" => {
                // Empty sets require element type; default to Int (uninterpreted elements)
                let int_sort = z3::Sort::int();
                Ok(z3::ast::Set::empty(&int_sort).into())
            }

            // Set membership: in_set(x, S) or builtin_set_member
            "in_set" | "builtin_set_member" | "set_member" | "member" => {
                if args.len() != 2 {
                    return Err("in_set requires 2 arguments (element, set)".to_string());
                }
                let set = Self::dynamic_to_set(&args[1])
                    .ok_or_else(|| "in_set second argument must be a set".to_string())?;
                Ok(set.member(&args[0]).into())
            }

            // Set union: union(A, B) or builtin_set_union
            "union" | "builtin_set_union" | "set_union" => {
                if args.len() < 2 {
                    return Err("union requires at least 2 arguments".to_string());
                }
                let sets: Result<Vec<z3::ast::Set>, String> = args
                    .iter()
                    .map(|a| {
                        Self::dynamic_to_set(a)
                            .ok_or_else(|| "union arguments must be sets".to_string())
                    })
                    .collect();
                let sets = sets?;
                let refs: Vec<&z3::ast::Set> = sets.iter().collect();
                Ok(z3::ast::Set::set_union(&refs).into())
            }

            // Set intersection: intersect(A, B) or builtin_set_intersect
            "intersect" | "builtin_set_intersect" | "set_intersect" | "intersection" => {
                if args.len() < 2 {
                    return Err("intersect requires at least 2 arguments".to_string());
                }
                let sets: Result<Vec<z3::ast::Set>, String> = args
                    .iter()
                    .map(|a| {
                        Self::dynamic_to_set(a)
                            .ok_or_else(|| "intersect arguments must be sets".to_string())
                    })
                    .collect();
                let sets = sets?;
                let refs: Vec<&z3::ast::Set> = sets.iter().collect();
                Ok(z3::ast::Set::intersect(&refs).into())
            }

            // Set difference: difference(A, B) or builtin_set_difference
            "difference" | "builtin_set_difference" | "set_difference" | "set_diff" => {
                if args.len() != 2 {
                    return Err("difference requires 2 arguments".to_string());
                }
                let set_a = Self::dynamic_to_set(&args[0])
                    .ok_or_else(|| "difference first argument must be a set".to_string())?;
                let set_b = Self::dynamic_to_set(&args[1])
                    .ok_or_else(|| "difference second argument must be a set".to_string())?;
                Ok(set_a.difference(&set_b).into())
            }

            // Set complement: complement(A) or builtin_set_complement
            "complement" | "builtin_set_complement" | "set_complement" => {
                if args.len() != 1 {
                    return Err("complement requires 1 argument".to_string());
                }
                let set = Self::dynamic_to_set(&args[0])
                    .ok_or_else(|| "complement argument must be a set".to_string())?;
                Ok(set.complement().into())
            }

            // Subset check: subset(A, B) or builtin_set_subset
            "subset" | "builtin_set_subset" | "set_subset" => {
                if args.len() != 2 {
                    return Err("subset requires 2 arguments".to_string());
                }
                let set_a = Self::dynamic_to_set(&args[0])
                    .ok_or_else(|| "subset first argument must be a set".to_string())?;
                let set_b = Self::dynamic_to_set(&args[1])
                    .ok_or_else(|| "subset second argument must be a set".to_string())?;
                Ok(set_a.set_subset(&set_b).into())
            }

            // Singleton set: singleton(x) or builtin_set_singleton
            "singleton" | "builtin_set_singleton" | "set_singleton" => {
                if args.len() != 1 {
                    return Err("singleton requires 1 argument".to_string());
                }
                // Create empty set and add the element
                let int_sort = z3::Sort::int();
                let empty = z3::ast::Set::empty(&int_sort);
                Ok(empty.add(&args[0]).into())
            }

            // Add element to set: insert(x, S) or builtin_set_add
            "insert" | "builtin_set_add" | "set_add" => {
                if args.len() != 2 {
                    return Err("insert requires 2 arguments (element, set)".to_string());
                }
                let set = Self::dynamic_to_set(&args[1])
                    .ok_or_else(|| "insert second argument must be a set".to_string())?;
                Ok(set.add(&args[0]).into())
            }

            // Remove element from set: remove(x, S) or builtin_set_del
            "remove" | "builtin_set_del" | "set_del" => {
                if args.len() != 2 {
                    return Err("remove requires 2 arguments (element, set)".to_string());
                }
                let set = Self::dynamic_to_set(&args[1])
                    .ok_or_else(|| "remove second argument must be a set".to_string())?;
                Ok(set.del(&args[0]).into())
            }

            // ============================================
            // COMPLEX NUMBER OPERATIONS (Hybrid Translation)
            // Uses Z3 Datatype for concrete arithmetic!
            // Complex = mk_complex(re: Real, im: Real)
            // ============================================

            // Imaginary unit: i = complex(0, 1)
            // This is a nullary operation (0 arguments)
            "i" => {
                if !args.is_empty() {
                    return Err("i takes no arguments".to_string());
                }
                if let Some(i_value) = self.get_complex_i() {
                    return Ok(i_value);
                }
                // Fallback to uninterpreted constant
                let func_decl = self.declare_uninterpreted("i", 0);
                Ok(func_decl.apply(&[]))
            }

            // Complex constructor: complex(re, im) creates re + im*i
            "complex" => {
                if args.len() != 2 {
                    return Err("complex requires 2 arguments (re, im)".to_string());
                }
                // Use datatype constructor for algebraic operations
                if let Some(ref cdt) = self.complex_datatype {
                    // Convert args to Real if they're Int
                    let re = args[0]
                        .as_real()
                        .or_else(|| args[0].as_int().map(|i| i.to_real()))
                        .ok_or("complex re argument must be numeric")?;
                    let im = args[1]
                        .as_real()
                        .or_else(|| args[1].as_int().map(|i| i.to_real()))
                        .ok_or("complex im argument must be numeric")?;
                    Ok(cdt.constructor().apply(&[&re as &dyn Ast, &im as &dyn Ast]))
                } else {
                    // Fallback to uninterpreted
                    let func_decl = self.declare_uninterpreted("complex", 2);
                    let ast_args: Vec<&dyn Ast> = args.iter().map(|d| d as &dyn Ast).collect();
                    Ok(func_decl.apply(&ast_args))
                }
            }

            // Extract real part: re(z)
            "re" | "real_part" => {
                if args.len() != 1 {
                    return Err("re requires 1 argument".to_string());
                }
                // Use datatype accessor
                if let Some(ref cdt) = self.complex_datatype
                    && self.is_complex_sort(&args[0])
                {
                    return Ok(cdt.accessor_re().apply(&[&args[0] as &dyn Ast]));
                }
                // Fallback for symbolic complex
                let func_decl = self.declare_uninterpreted("re", 1);
                let ast_args: Vec<&dyn Ast> = args.iter().map(|d| d as &dyn Ast).collect();
                Ok(func_decl.apply(&ast_args))
            }

            // Extract imaginary part: im(z)
            "im" | "imag_part" => {
                if args.len() != 1 {
                    return Err("im requires 1 argument".to_string());
                }
                // Use datatype accessor
                if let Some(ref cdt) = self.complex_datatype
                    && self.is_complex_sort(&args[0])
                {
                    return Ok(cdt.accessor_im().apply(&[&args[0] as &dyn Ast]));
                }
                // Fallback for symbolic complex
                let func_decl = self.declare_uninterpreted("im", 1);
                let ast_args: Vec<&dyn Ast> = args.iter().map(|d| d as &dyn Ast).collect();
                Ok(func_decl.apply(&ast_args))
            }

            // Complex conjugate: conj(z) = complex(re(z), -im(z))
            "conj" | "conjugate" => {
                if args.len() != 1 {
                    return Err("conj requires 1 argument".to_string());
                }
                // Use algebraic translation
                if let Some(ref cdt) = self.complex_datatype
                    && self.is_complex_sort(&args[0])
                {
                    let re = cdt.accessor_re().apply(&[&args[0] as &dyn Ast]);
                    let im = cdt.accessor_im().apply(&[&args[0] as &dyn Ast]);
                    let neg_im = im.as_real().map(|r| r.unary_minus()).ok_or("im not Real")?;
                    let re_real = re.as_real().ok_or("re not Real")?;
                    return Ok(cdt
                        .constructor()
                        .apply(&[&re_real as &dyn Ast, &neg_im as &dyn Ast]));
                }
                // Fallback
                let func_decl = self.declare_uninterpreted("conj", 1);
                let ast_args: Vec<&dyn Ast> = args.iter().map(|d| d as &dyn Ast).collect();
                Ok(func_decl.apply(&ast_args))
            }

            // Complex addition: (a+bi) + (c+di) = (a+c) + (b+d)i
            "complex_add" => {
                if args.len() != 2 {
                    return Err("complex_add requires 2 arguments".to_string());
                }
                // Algebraic translation
                if let Some(ref cdt) = self.complex_datatype
                    && self.is_complex_sort(&args[0])
                    && self.is_complex_sort(&args[1])
                {
                    let re1 = cdt.accessor_re().apply(&[&args[0] as &dyn Ast]);
                    let im1 = cdt.accessor_im().apply(&[&args[0] as &dyn Ast]);
                    let re2 = cdt.accessor_re().apply(&[&args[1] as &dyn Ast]);
                    let im2 = cdt.accessor_im().apply(&[&args[1] as &dyn Ast]);

                    let re_sum = Real::add(&[
                        &re1.as_real().ok_or("re1 not Real")?,
                        &re2.as_real().ok_or("re2 not Real")?,
                    ]);
                    let im_sum = Real::add(&[
                        &im1.as_real().ok_or("im1 not Real")?,
                        &im2.as_real().ok_or("im2 not Real")?,
                    ]);
                    return Ok(cdt
                        .constructor()
                        .apply(&[&re_sum as &dyn Ast, &im_sum as &dyn Ast]));
                }
                // Fallback
                let func_decl = self.declare_uninterpreted("complex_add", 2);
                let ast_args: Vec<&dyn Ast> = args.iter().map(|d| d as &dyn Ast).collect();
                Ok(func_decl.apply(&ast_args))
            }

            // Complex multiplication: (a+bi)(c+di) = (ac-bd) + (ad+bc)i
            "complex_mul" => {
                if args.len() != 2 {
                    return Err("complex_mul requires 2 arguments".to_string());
                }
                // Algebraic translation
                if let Some(ref cdt) = self.complex_datatype
                    && self.is_complex_sort(&args[0])
                    && self.is_complex_sort(&args[1])
                {
                    let a = cdt
                        .accessor_re()
                        .apply(&[&args[0] as &dyn Ast])
                        .as_real()
                        .ok_or("a not Real")?;
                    let b = cdt
                        .accessor_im()
                        .apply(&[&args[0] as &dyn Ast])
                        .as_real()
                        .ok_or("b not Real")?;
                    let c = cdt
                        .accessor_re()
                        .apply(&[&args[1] as &dyn Ast])
                        .as_real()
                        .ok_or("c not Real")?;
                    let d = cdt
                        .accessor_im()
                        .apply(&[&args[1] as &dyn Ast])
                        .as_real()
                        .ok_or("d not Real")?;

                    // Real part: ac - bd
                    let ac = Real::mul(&[&a, &c]);
                    let bd = Real::mul(&[&b, &d]);
                    let re_result = Real::sub(&[&ac, &bd]);

                    // Imaginary part: ad + bc
                    let ad = Real::mul(&[&a, &d]);
                    let bc = Real::mul(&[&b, &c]);
                    let im_result = Real::add(&[&ad, &bc]);

                    return Ok(cdt
                        .constructor()
                        .apply(&[&re_result as &dyn Ast, &im_result as &dyn Ast]));
                }
                // Fallback
                let func_decl = self.declare_uninterpreted("complex_mul", 2);
                let ast_args: Vec<&dyn Ast> = args.iter().map(|d| d as &dyn Ast).collect();
                Ok(func_decl.apply(&ast_args))
            }

            // Complex inverse: 1/z = conj(z) / |z|²
            "complex_inverse" => {
                if args.len() != 1 {
                    return Err("complex_inverse requires 1 argument".to_string());
                }
                // Algebraic: 1/z = (a - bi) / (a² + b²)
                if let Some(ref cdt) = self.complex_datatype
                    && self.is_complex_sort(&args[0])
                {
                    let a = cdt
                        .accessor_re()
                        .apply(&[&args[0] as &dyn Ast])
                        .as_real()
                        .ok_or("a not Real")?;
                    let b = cdt
                        .accessor_im()
                        .apply(&[&args[0] as &dyn Ast])
                        .as_real()
                        .ok_or("b not Real")?;

                    // |z|² = a² + b²
                    let a_sq = Real::mul(&[&a, &a]);
                    let b_sq = Real::mul(&[&b, &b]);
                    let abs_sq = Real::add(&[&a_sq, &b_sq]);

                    // 1/z = (a / |z|², -b / |z|²)
                    let re_result = a.div(&abs_sq);
                    let neg_b = b.unary_minus();
                    let im_result = neg_b.div(&abs_sq);

                    return Ok(cdt
                        .constructor()
                        .apply(&[&re_result as &dyn Ast, &im_result as &dyn Ast]));
                }
                let func_decl = self.declare_uninterpreted("complex_inverse", 1);
                let ast_args: Vec<&dyn Ast> = args.iter().map(|d| d as &dyn Ast).collect();
                Ok(func_decl.apply(&ast_args))
            }

            // Complex subtraction: (a+bi) - (c+di) = (a-c) + (b-d)i
            "complex_sub" => {
                if args.len() != 2 {
                    return Err("complex_sub requires 2 arguments".to_string());
                }
                if let Some(ref cdt) = self.complex_datatype
                    && self.is_complex_sort(&args[0])
                    && self.is_complex_sort(&args[1])
                {
                    let re1 = cdt
                        .accessor_re()
                        .apply(&[&args[0] as &dyn Ast])
                        .as_real()
                        .ok_or("re1")?;
                    let im1 = cdt
                        .accessor_im()
                        .apply(&[&args[0] as &dyn Ast])
                        .as_real()
                        .ok_or("im1")?;
                    let re2 = cdt
                        .accessor_re()
                        .apply(&[&args[1] as &dyn Ast])
                        .as_real()
                        .ok_or("re2")?;
                    let im2 = cdt
                        .accessor_im()
                        .apply(&[&args[1] as &dyn Ast])
                        .as_real()
                        .ok_or("im2")?;

                    let re_diff = Real::sub(&[&re1, &re2]);
                    let im_diff = Real::sub(&[&im1, &im2]);
                    return Ok(cdt
                        .constructor()
                        .apply(&[&re_diff as &dyn Ast, &im_diff as &dyn Ast]));
                }
                let func_decl = self.declare_uninterpreted("complex_sub", 2);
                let ast_args: Vec<&dyn Ast> = args.iter().map(|d| d as &dyn Ast).collect();
                Ok(func_decl.apply(&ast_args))
            }

            // Complex division: z1/z2 = z1 * (1/z2)
            "complex_div" => {
                if args.len() != 2 {
                    return Err("complex_div requires 2 arguments".to_string());
                }
                if let Some(ref cdt) = self.complex_datatype
                    && self.is_complex_sort(&args[0])
                    && self.is_complex_sort(&args[1])
                {
                    let a = cdt
                        .accessor_re()
                        .apply(&[&args[0] as &dyn Ast])
                        .as_real()
                        .ok_or("a")?;
                    let b = cdt
                        .accessor_im()
                        .apply(&[&args[0] as &dyn Ast])
                        .as_real()
                        .ok_or("b")?;
                    let c = cdt
                        .accessor_re()
                        .apply(&[&args[1] as &dyn Ast])
                        .as_real()
                        .ok_or("c")?;
                    let d = cdt
                        .accessor_im()
                        .apply(&[&args[1] as &dyn Ast])
                        .as_real()
                        .ok_or("d")?;

                    // z1/z2 = (ac + bd)/(c² + d²) + i(bc - ad)/(c² + d²)
                    let c_sq = Real::mul(&[&c, &c]);
                    let d_sq = Real::mul(&[&d, &d]);
                    let denom = Real::add(&[&c_sq, &d_sq]);

                    let ac = Real::mul(&[&a, &c]);
                    let bd = Real::mul(&[&b, &d]);
                    let bc = Real::mul(&[&b, &c]);
                    let ad = Real::mul(&[&a, &d]);

                    let re_num = Real::add(&[&ac, &bd]);
                    let im_num = Real::sub(&[&bc, &ad]);

                    let re_result = re_num.div(&denom);
                    let im_result = im_num.div(&denom);

                    return Ok(cdt
                        .constructor()
                        .apply(&[&re_result as &dyn Ast, &im_result as &dyn Ast]));
                }
                let func_decl = self.declare_uninterpreted("complex_div", 2);
                let ast_args: Vec<&dyn Ast> = args.iter().map(|d| d as &dyn Ast).collect();
                Ok(func_decl.apply(&ast_args))
            }

            // Complex negation: -z = (-re, -im)
            "neg_complex" => {
                if args.len() != 1 {
                    return Err("neg_complex requires 1 argument".to_string());
                }
                if let Some(ref cdt) = self.complex_datatype
                    && self.is_complex_sort(&args[0])
                {
                    let re = cdt
                        .accessor_re()
                        .apply(&[&args[0] as &dyn Ast])
                        .as_real()
                        .ok_or("re")?;
                    let im = cdt
                        .accessor_im()
                        .apply(&[&args[0] as &dyn Ast])
                        .as_real()
                        .ok_or("im")?;
                    let neg_re = re.unary_minus();
                    let neg_im = im.unary_minus();
                    return Ok(cdt
                        .constructor()
                        .apply(&[&neg_re as &dyn Ast, &neg_im as &dyn Ast]));
                }
                let func_decl = self.declare_uninterpreted("neg_complex", 1);
                let ast_args: Vec<&dyn Ast> = args.iter().map(|d| d as &dyn Ast).collect();
                Ok(func_decl.apply(&ast_args))
            }

            // Complex magnitude squared: |z|² = re² + im²
            "abs_squared" => {
                if args.len() != 1 {
                    return Err("abs_squared requires 1 argument".to_string());
                }
                if let Some(ref cdt) = self.complex_datatype
                    && self.is_complex_sort(&args[0])
                {
                    let re = cdt
                        .accessor_re()
                        .apply(&[&args[0] as &dyn Ast])
                        .as_real()
                        .ok_or("re")?;
                    let im = cdt
                        .accessor_im()
                        .apply(&[&args[0] as &dyn Ast])
                        .as_real()
                        .ok_or("im")?;
                    let re_sq = Real::mul(&[&re, &re]);
                    let im_sq = Real::mul(&[&im, &im]);
                    return Ok(Real::add(&[&re_sq, &im_sq]).into());
                }
                let func_decl = self.declare_uninterpreted("abs_squared", 1);
                let ast_args: Vec<&dyn Ast> = args.iter().map(|d| d as &dyn Ast).collect();
                Ok(func_decl.apply(&ast_args))
            }

            // ============================================
            // RATIONAL NUMBER OPERATIONS
            // Z3 Real sort is actually ℚ (rationals), so we use it directly
            // ============================================

            // Rational constructor: rational(p, q) = p / q
            "rational" => {
                if args.len() != 2 {
                    return Err("rational requires 2 arguments".to_string());
                }
                // Convert to Real and divide
                let numer = self.to_real(&args[0])?;
                let denom = self.to_real(&args[1])?;
                Ok(Real::div(&numer, &denom).into())
            }

            // Rational addition
            "rational_add" => {
                if args.len() != 2 {
                    return Err("rational_add requires 2 arguments".to_string());
                }
                let r1 = self.to_real(&args[0])?;
                let r2 = self.to_real(&args[1])?;
                Ok(Real::add(&[&r1, &r2]).into())
            }

            // Rational subtraction
            "rational_sub" => {
                if args.len() != 2 {
                    return Err("rational_sub requires 2 arguments".to_string());
                }
                let r1 = self.to_real(&args[0])?;
                let r2 = self.to_real(&args[1])?;
                Ok(Real::sub(&[&r1, &r2]).into())
            }

            // Rational multiplication
            "rational_mul" => {
                if args.len() != 2 {
                    return Err("rational_mul requires 2 arguments".to_string());
                }
                let r1 = self.to_real(&args[0])?;
                let r2 = self.to_real(&args[1])?;
                Ok(Real::mul(&[&r1, &r2]).into())
            }

            // Rational division
            "rational_div" => {
                if args.len() != 2 {
                    return Err("rational_div requires 2 arguments".to_string());
                }
                let r1 = self.to_real(&args[0])?;
                let r2 = self.to_real(&args[1])?;
                Ok(Real::div(&r1, &r2).into())
            }

            // Rational negation
            "neg_rational" => {
                if args.len() != 1 {
                    return Err("neg_rational requires 1 argument".to_string());
                }
                let r = self.to_real(&args[0])?;
                Ok(r.unary_minus().into())
            }

            // Rational inverse (reciprocal)
            "rational_inv" => {
                if args.len() != 1 {
                    return Err("rational_inv requires 1 argument".to_string());
                }
                let r = self.to_real(&args[0])?;
                let one = Real::from_rational(1, 1);
                Ok(Real::div(&one, &r).into())
            }

            // Rational comparisons - return Bool
            "rational_lt" => {
                if args.len() != 2 {
                    return Err("rational_lt requires 2 arguments".to_string());
                }
                let r1 = self.to_real(&args[0])?;
                let r2 = self.to_real(&args[1])?;
                Ok(r1.lt(&r2).into())
            }

            "rational_le" => {
                if args.len() != 2 {
                    return Err("rational_le requires 2 arguments".to_string());
                }
                let r1 = self.to_real(&args[0])?;
                let r2 = self.to_real(&args[1])?;
                Ok(r1.le(&r2).into())
            }

            "rational_gt" => {
                if args.len() != 2 {
                    return Err("rational_gt requires 2 arguments".to_string());
                }
                let r1 = self.to_real(&args[0])?;
                let r2 = self.to_real(&args[1])?;
                Ok(r1.gt(&r2).into())
            }

            "rational_ge" => {
                if args.len() != 2 {
                    return Err("rational_ge requires 2 arguments".to_string());
                }
                let r1 = self.to_real(&args[0])?;
                let r2 = self.to_real(&args[1])?;
                Ok(r1.ge(&r2).into())
            }

            // Integer to rational conversion
            "int_to_rational" | "nat_to_rational" => {
                if args.len() != 1 {
                    return Err(format!("{} requires 1 argument", name));
                }
                // Convert Int to Real (ℤ → ℚ)
                Ok(self.to_real(&args[0])?.into())
            }

            // Rational to real (identity in Z3, since Real = ℚ)
            "to_real" => {
                if args.len() != 1 {
                    return Err("to_real requires 1 argument".to_string());
                }
                Ok(self.to_real(&args[0])?.into())
            }

            // Numerator accessor (uninterpreted - Z3 doesn't expose this)
            "numer" => {
                let func_decl = self.declare_uninterpreted("numer", 1);
                let ast_args: Vec<&dyn Ast> = args.iter().map(|d| d as &dyn Ast).collect();
                Ok(func_decl.apply(&ast_args))
            }

            // Denominator accessor (uninterpreted - Z3 doesn't expose this)
            "denom" => {
                let func_decl = self.declare_uninterpreted("denom", 1);
                let ast_args: Vec<&dyn Ast> = args.iter().map(|d| d as &dyn Ast).collect();
                Ok(func_decl.apply(&ast_args))
            }

            // ============================================
            // INTEGER DIVISION AND MODULO OPERATIONS
            // ============================================

            // Integer division: a div b (floor division)
            "int_div" | "div" => {
                if args.len() != 2 {
                    return Err("int_div requires 2 arguments".to_string());
                }
                if let (Some(a), Some(b)) = (args[0].as_int(), args[1].as_int()) {
                    Ok(a.div(&b).into())
                } else {
                    Err("int_div requires integer arguments".to_string())
                }
            }

            // Integer modulo: a mod b (always non-negative result)
            "int_mod" | "mod" => {
                if args.len() != 2 {
                    return Err("int_mod requires 2 arguments".to_string());
                }
                if let (Some(a), Some(b)) = (args[0].as_int(), args[1].as_int()) {
                    Ok(a.modulo(&b).into())
                } else {
                    Err("int_mod requires integer arguments".to_string())
                }
            }

            // Integer remainder: a rem b (sign follows dividend)
            "int_rem" | "rem" => {
                if args.len() != 2 {
                    return Err("int_rem requires 2 arguments".to_string());
                }
                if let (Some(a), Some(b)) = (args[0].as_int(), args[1].as_int()) {
                    Ok(a.rem(&b).into())
                } else {
                    Err("int_rem requires integer arguments".to_string())
                }
            }

            // ============================================
            // FLOOR AND CEILING (ℚ → ℤ)
            // ============================================

            // Floor: largest integer ≤ r
            "floor" => {
                if args.len() != 1 {
                    return Err("floor requires 1 argument".to_string());
                }
                let r = self.to_real(&args[0])?;
                // Z3's Real::to_int() computes floor
                Ok(r.to_int().into())
            }

            // Ceiling: smallest integer ≥ r
            // ceil(r) = -floor(-r)
            "ceil" | "ceiling" => {
                if args.len() != 1 {
                    return Err("ceil requires 1 argument".to_string());
                }
                let r = self.to_real(&args[0])?;
                let neg_r = r.unary_minus();
                let floor_neg_r = neg_r.to_int();
                Ok(Int::unary_minus(&floor_neg_r).into())
            }

            // ============================================
            // GCD (Greatest Common Divisor)
            // Defined axiomatically: gcd(a,b) is the largest d such that d|a and d|b
            // ============================================
            "gcd" => {
                if args.len() != 2 {
                    return Err("gcd requires 2 arguments".to_string());
                }
                // Use uninterpreted function with axioms
                // The actual GCD computation is done via axioms in stdlib/rational.kleis
                let func_decl = self.declare_uninterpreted("gcd", 2);
                let ast_args: Vec<&dyn Ast> = args.iter().map(|d| d as &dyn Ast).collect();
                Ok(func_decl.apply(&ast_args))
            }

            // ============================================
            // ABSOLUTE VALUE
            // ============================================

            // Absolute value for rationals (abs is handled above, this catches abs_rational)
            "abs_rational" => {
                if args.len() != 1 {
                    return Err("abs requires 1 argument".to_string());
                }
                let r = self.to_real(&args[0])?;
                let zero = Real::from_rational(0, 1);
                let neg_r = r.unary_minus();
                // abs(r) = if r >= 0 then r else -r
                Ok(r.ge(&zero).ite(&r, &neg_r).into())
            }

            // ============================================
            // BIT-VECTOR OPERATIONS (native Z3 BitVec theory)
            // ============================================

            // Bitwise AND
            "bvand" => {
                if args.len() != 2 {
                    return Err("bvand requires 2 arguments".to_string());
                }
                if let (Some(a), Some(b)) =
                    (Self::dynamic_to_bv(&args[0]), Self::dynamic_to_bv(&args[1]))
                {
                    Ok(a.bvand(&b).into())
                } else {
                    let func_decl = self.declare_uninterpreted("bvand", 2);
                    let ast_args: Vec<&dyn Ast> = args.iter().map(|d| d as &dyn Ast).collect();
                    Ok(func_decl.apply(&ast_args))
                }
            }

            // Bitwise OR
            "bvor" => {
                if args.len() != 2 {
                    return Err("bvor requires 2 arguments".to_string());
                }
                if let (Some(a), Some(b)) =
                    (Self::dynamic_to_bv(&args[0]), Self::dynamic_to_bv(&args[1]))
                {
                    Ok(a.bvor(&b).into())
                } else {
                    let func_decl = self.declare_uninterpreted("bvor", 2);
                    let ast_args: Vec<&dyn Ast> = args.iter().map(|d| d as &dyn Ast).collect();
                    Ok(func_decl.apply(&ast_args))
                }
            }

            // Bitwise XOR
            "bvxor" => {
                if args.len() != 2 {
                    return Err("bvxor requires 2 arguments".to_string());
                }
                if let (Some(a), Some(b)) =
                    (Self::dynamic_to_bv(&args[0]), Self::dynamic_to_bv(&args[1]))
                {
                    Ok(a.bvxor(&b).into())
                } else {
                    let func_decl = self.declare_uninterpreted("bvxor", 2);
                    let ast_args: Vec<&dyn Ast> = args.iter().map(|d| d as &dyn Ast).collect();
                    Ok(func_decl.apply(&ast_args))
                }
            }

            // Bitwise NOT
            "bvnot" => {
                if args.len() != 1 {
                    return Err("bvnot requires 1 argument".to_string());
                }
                if let Some(a) = Self::dynamic_to_bv(&args[0]) {
                    Ok(a.bvnot().into())
                } else {
                    let func_decl = self.declare_uninterpreted("bvnot", 1);
                    let ast_args: Vec<&dyn Ast> = args.iter().map(|d| d as &dyn Ast).collect();
                    Ok(func_decl.apply(&ast_args))
                }
            }

            // Bit-vector addition (modular)
            "bvadd" => {
                if args.len() != 2 {
                    return Err("bvadd requires 2 arguments".to_string());
                }
                if let (Some(a), Some(b)) =
                    (Self::dynamic_to_bv(&args[0]), Self::dynamic_to_bv(&args[1]))
                {
                    Ok(a.bvadd(&b).into())
                } else {
                    let func_decl = self.declare_uninterpreted("bvadd", 2);
                    let ast_args: Vec<&dyn Ast> = args.iter().map(|d| d as &dyn Ast).collect();
                    Ok(func_decl.apply(&ast_args))
                }
            }

            // Bit-vector subtraction
            "bvsub" => {
                if args.len() != 2 {
                    return Err("bvsub requires 2 arguments".to_string());
                }
                if let (Some(a), Some(b)) =
                    (Self::dynamic_to_bv(&args[0]), Self::dynamic_to_bv(&args[1]))
                {
                    Ok(a.bvsub(&b).into())
                } else {
                    let func_decl = self.declare_uninterpreted("bvsub", 2);
                    let ast_args: Vec<&dyn Ast> = args.iter().map(|d| d as &dyn Ast).collect();
                    Ok(func_decl.apply(&ast_args))
                }
            }

            // Bit-vector multiplication
            "bvmul" => {
                if args.len() != 2 {
                    return Err("bvmul requires 2 arguments".to_string());
                }
                if let (Some(a), Some(b)) =
                    (Self::dynamic_to_bv(&args[0]), Self::dynamic_to_bv(&args[1]))
                {
                    Ok(a.bvmul(&b).into())
                } else {
                    let func_decl = self.declare_uninterpreted("bvmul", 2);
                    let ast_args: Vec<&dyn Ast> = args.iter().map(|d| d as &dyn Ast).collect();
                    Ok(func_decl.apply(&ast_args))
                }
            }

            // Bit-vector negation (two's complement)
            "bvneg" => {
                if args.len() != 1 {
                    return Err("bvneg requires 1 argument".to_string());
                }
                if let Some(a) = Self::dynamic_to_bv(&args[0]) {
                    Ok(a.bvneg().into())
                } else {
                    let func_decl = self.declare_uninterpreted("bvneg", 1);
                    let ast_args: Vec<&dyn Ast> = args.iter().map(|d| d as &dyn Ast).collect();
                    Ok(func_decl.apply(&ast_args))
                }
            }

            // Unsigned division
            "bvudiv" => {
                if args.len() != 2 {
                    return Err("bvudiv requires 2 arguments".to_string());
                }
                if let (Some(a), Some(b)) =
                    (Self::dynamic_to_bv(&args[0]), Self::dynamic_to_bv(&args[1]))
                {
                    Ok(a.bvudiv(&b).into())
                } else {
                    let func_decl = self.declare_uninterpreted("bvudiv", 2);
                    let ast_args: Vec<&dyn Ast> = args.iter().map(|d| d as &dyn Ast).collect();
                    Ok(func_decl.apply(&ast_args))
                }
            }

            // Signed division
            "bvsdiv" => {
                if args.len() != 2 {
                    return Err("bvsdiv requires 2 arguments".to_string());
                }
                if let (Some(a), Some(b)) =
                    (Self::dynamic_to_bv(&args[0]), Self::dynamic_to_bv(&args[1]))
                {
                    Ok(a.bvsdiv(&b).into())
                } else {
                    let func_decl = self.declare_uninterpreted("bvsdiv", 2);
                    let ast_args: Vec<&dyn Ast> = args.iter().map(|d| d as &dyn Ast).collect();
                    Ok(func_decl.apply(&ast_args))
                }
            }

            // Unsigned remainder
            "bvurem" => {
                if args.len() != 2 {
                    return Err("bvurem requires 2 arguments".to_string());
                }
                if let (Some(a), Some(b)) =
                    (Self::dynamic_to_bv(&args[0]), Self::dynamic_to_bv(&args[1]))
                {
                    Ok(a.bvurem(&b).into())
                } else {
                    let func_decl = self.declare_uninterpreted("bvurem", 2);
                    let ast_args: Vec<&dyn Ast> = args.iter().map(|d| d as &dyn Ast).collect();
                    Ok(func_decl.apply(&ast_args))
                }
            }

            // Left shift
            "bvshl" => {
                if args.len() != 2 {
                    return Err("bvshl requires 2 arguments".to_string());
                }
                if let (Some(a), Some(b)) =
                    (Self::dynamic_to_bv(&args[0]), Self::dynamic_to_bv(&args[1]))
                {
                    Ok(a.bvshl(&b).into())
                } else {
                    let func_decl = self.declare_uninterpreted("bvshl", 2);
                    let ast_args: Vec<&dyn Ast> = args.iter().map(|d| d as &dyn Ast).collect();
                    Ok(func_decl.apply(&ast_args))
                }
            }

            // Logical right shift
            "bvlshr" => {
                if args.len() != 2 {
                    return Err("bvlshr requires 2 arguments".to_string());
                }
                if let (Some(a), Some(b)) =
                    (Self::dynamic_to_bv(&args[0]), Self::dynamic_to_bv(&args[1]))
                {
                    Ok(a.bvlshr(&b).into())
                } else {
                    let func_decl = self.declare_uninterpreted("bvlshr", 2);
                    let ast_args: Vec<&dyn Ast> = args.iter().map(|d| d as &dyn Ast).collect();
                    Ok(func_decl.apply(&ast_args))
                }
            }

            // Arithmetic right shift
            "bvashr" => {
                if args.len() != 2 {
                    return Err("bvashr requires 2 arguments".to_string());
                }
                if let (Some(a), Some(b)) =
                    (Self::dynamic_to_bv(&args[0]), Self::dynamic_to_bv(&args[1]))
                {
                    Ok(a.bvashr(&b).into())
                } else {
                    let func_decl = self.declare_uninterpreted("bvashr", 2);
                    let ast_args: Vec<&dyn Ast> = args.iter().map(|d| d as &dyn Ast).collect();
                    Ok(func_decl.apply(&ast_args))
                }
            }

            // Unsigned less-than
            "bvult" => {
                if args.len() != 2 {
                    return Err("bvult requires 2 arguments".to_string());
                }
                if let (Some(a), Some(b)) =
                    (Self::dynamic_to_bv(&args[0]), Self::dynamic_to_bv(&args[1]))
                {
                    Ok(a.bvult(&b).into())
                } else {
                    let func_decl = self.declare_uninterpreted("bvult", 2);
                    let ast_args: Vec<&dyn Ast> = args.iter().map(|d| d as &dyn Ast).collect();
                    Ok(func_decl.apply(&ast_args))
                }
            }

            // Unsigned less-or-equal
            "bvule" => {
                if args.len() != 2 {
                    return Err("bvule requires 2 arguments".to_string());
                }
                if let (Some(a), Some(b)) =
                    (Self::dynamic_to_bv(&args[0]), Self::dynamic_to_bv(&args[1]))
                {
                    Ok(a.bvule(&b).into())
                } else {
                    let func_decl = self.declare_uninterpreted("bvule", 2);
                    let ast_args: Vec<&dyn Ast> = args.iter().map(|d| d as &dyn Ast).collect();
                    Ok(func_decl.apply(&ast_args))
                }
            }

            // Signed less-than
            "bvslt" => {
                if args.len() != 2 {
                    return Err("bvslt requires 2 arguments".to_string());
                }
                if let (Some(a), Some(b)) =
                    (Self::dynamic_to_bv(&args[0]), Self::dynamic_to_bv(&args[1]))
                {
                    Ok(a.bvslt(&b).into())
                } else {
                    let func_decl = self.declare_uninterpreted("bvslt", 2);
                    let ast_args: Vec<&dyn Ast> = args.iter().map(|d| d as &dyn Ast).collect();
                    Ok(func_decl.apply(&ast_args))
                }
            }

            // Signed less-or-equal
            "bvsle" => {
                if args.len() != 2 {
                    return Err("bvsle requires 2 arguments".to_string());
                }
                if let (Some(a), Some(b)) =
                    (Self::dynamic_to_bv(&args[0]), Self::dynamic_to_bv(&args[1]))
                {
                    Ok(a.bvsle(&b).into())
                } else {
                    let func_decl = self.declare_uninterpreted("bvsle", 2);
                    let ast_args: Vec<&dyn Ast> = args.iter().map(|d| d as &dyn Ast).collect();
                    Ok(func_decl.apply(&ast_args))
                }
            }

            // Unknown operation — declare as uninterpreted function.
            // declare_uninterpreted checks the cache first (sort consistency),
            // then the registry for a typed signature, then falls back to
            // all-Int domain. For operations with ADT-sorted arguments (e.g.
            // KleisList) that have no registry signature, we infer domain
            // sorts from the actual arguments on first declaration only.
            _ => {
                let func_decl = if !self.declared_ops.contains_key(name)
                    && self.registry.get_operation_signature(name).is_none()
                {
                    let domain: Vec<Sort> = args.iter().map(|a| a.get_sort()).collect();
                    let range = Sort::int();
                    let domain_refs: Vec<&Sort> = domain.iter().collect();
                    let fd = FuncDecl::new(name, &domain_refs, &range);
                    self.declared_ops.insert(name.to_string(), (domain, range));
                    fd
                } else {
                    self.declare_uninterpreted(name, args.len())
                };

                // Auto-promote Int→Real when the function expects Real
                let mut promoted_args: Vec<Dynamic> = args.to_vec();
                let arity = func_decl.arity();
                for i in 0..arity {
                    if let Some(expected_sort_kind) = func_decl.domain(i)
                        && let Some(arg) = promoted_args.get(i)
                    {
                        let actual_sort = arg.get_sort();
                        if expected_sort_kind != actual_sort.kind() {
                            if actual_sort.kind() == z3::SortKind::Int
                                && expected_sort_kind == z3::SortKind::Real
                            {
                                let real_val = arg.as_int().unwrap().to_real();
                                promoted_args[i] = real_val.into();
                            } else {
                                return Err(format!(
                                    "Type mismatch in call to '{}': argument {} has type {:?} but expected {:?}.\n\
                                         Hint: Check if '{}' is declared with the correct signature, or if there are \
                                         duplicate definitions with different types.",
                                    name,
                                    i + 1,
                                    actual_sort,
                                    expected_sort_kind,
                                    name
                                ));
                            }
                        }
                    }
                }

                let ast_args: Vec<&dyn Ast> = promoted_args.iter().map(|d| d as &dyn Ast).collect();
                Ok(func_decl.apply(&ast_args))
            }
        }
    }

    /// Convert a Dynamic to a Real (for rational operations)
    fn to_real(&self, d: &Dynamic) -> Result<Real, String> {
        if let Some(r) = d.as_real() {
            Ok(r)
        } else if let Some(i) = d.as_int() {
            Ok(Int::to_real(&i))
        } else {
            // Try to use it as-is and hope it works
            Err(format!("Cannot convert {:?} to Real", d))
        }
    }

    /// Declare an uninterpreted function in Z3 with proper typing
    ///
    /// Looks up the operation signature from the registry to determine:
    /// - Domain sorts (from argument types)
    /// - Range sort (from return type)
    ///
    /// Type mapping:
    /// - ℂ/Complex → Complex datatype sort
    /// - ℝ/Scalar/Real → Real sort
    /// - Bool → Bool sort  
    /// - Everything else → Int sort (uninterpreted as integers)
    fn declare_uninterpreted(&mut self, name: &str, arity: usize) -> FuncDecl {
        // Return from cached sort signature if we've seen this operation.
        // Z3 interns FuncDecl by (name, sorts), so re-creating with the
        // same sorts returns the same declaration object.
        if let Some((domain, range)) = self.declared_ops.get(name) {
            let domain_refs: Vec<&Sort> = domain.iter().collect();
            return FuncDecl::new(name, &domain_refs, range);
        }

        // Try to get the operation signature from the registry
        if let Some(type_sig) = self.registry.get_operation_signature(name) {
            return self.declare_typed_function(name, type_sig, arity);
        }

        // No signature found: default to Int → Int (uninterpreted)
        self.add_warning(format!(
            "Operation '{}' has no type signature in registry. Using untyped fallback (Int → Int). \
             Consider adding: operation {} : <args> → <return_type>",
            name, name
        ));

        let domain: Vec<_> = (0..arity).map(|_| Sort::int()).collect();
        let range = Sort::int();
        let domain_refs: Vec<&Sort> = domain.iter().collect();
        let func_decl = FuncDecl::new(name, &domain_refs, &range);
        self.declared_ops.insert(name.to_string(), (domain, range));
        func_decl
    }

    /// Declare a function with proper types from its signature
    fn declare_typed_function(
        &mut self,
        name: &str,
        type_sig: &TypeExpr,
        arity: usize,
    ) -> FuncDecl {
        if let Some((domain, range)) = self.declared_ops.get(name) {
            let domain_refs: Vec<&Sort> = domain.iter().collect();
            return FuncDecl::new(name, &domain_refs, range);
        }

        let (arg_types, ret_type) = self.extract_signature_types(type_sig);

        let domain: Vec<Sort> = if arg_types.is_empty() {
            (0..arity).map(|_| Sort::int()).collect()
        } else {
            arg_types
                .iter()
                .map(|t| self.type_expr_to_sort(t))
                .collect()
        };

        let range = self.type_expr_to_sort(&ret_type);

        let domain_strs: Vec<String> = domain.iter().map(|s| format!("{}", s)).collect();
        eprintln!(
            "   🔧 Declaring typed function: {} : {} → {}",
            name,
            domain_strs.join(" × "),
            range
        );

        let domain_refs: Vec<_> = domain.iter().collect();
        let func_decl = FuncDecl::new(name, &domain_refs, &range);
        self.declared_ops.insert(name.to_string(), (domain, range));
        func_decl
    }

    /// Extract argument types and return type from a function signature
    ///
    /// Handles curried types: `A → B → C` means args=[A, B], return=C
    fn extract_signature_types(&self, type_sig: &TypeExpr) -> (Vec<TypeExpr>, TypeExpr) {
        let mut args = Vec::new();
        let mut current = type_sig.clone();

        // Uncurry: A → B → C → D becomes args=[A, B, C], return=D
        while let TypeExpr::Function(from, to) = current {
            // Handle Product types in arguments (tuple parameters)
            match from.as_ref() {
                TypeExpr::Product(types) => args.extend(types.clone()),
                single => args.push(single.clone()),
            }
            current = *to;
        }

        // current is now the final return type (non-function)
        (args, current)
    }

    /// Convert a Kleis TypeExpr to a Z3 Sort
    fn type_expr_to_sort(&self, type_expr: &TypeExpr) -> Sort {
        match type_expr {
            TypeExpr::Named(name) => self.type_name_to_sort(name),
            TypeExpr::Parametric(name, _) => {
                // For parametric types like Vector(3, ℂ), use the base type name
                self.type_name_to_sort(name)
            }
            TypeExpr::Function(_, _) => {
                // Function types - use Int as uninterpreted
                Sort::int()
            }
            TypeExpr::Product(_) => {
                // Product types - use Int as uninterpreted
                Sort::int()
            }
            TypeExpr::Var(name) => {
                // Type variable - check if it's a known type
                self.type_name_to_sort(name)
            }
            TypeExpr::ForAll { body, .. } => {
                // Polymorphic type - use body's sort
                self.type_expr_to_sort(body)
            }
            TypeExpr::DimExpr(_) => {
                // Dimension expression - use Int
                Sort::int()
            }
        }
    }

    /// Convert a type name string to Z3 Sort
    ///
    /// Priority order:
    /// 1. Declared data types from registry
    /// 2. Type aliases from registry (resolved to underlying type)
    /// 3. Built-in primitive types
    /// 4. Default to Int for unknown/type variables
    fn type_name_to_sort(&self, name: &str) -> Sort {
        // 1. Check declared data types from registry
        if let Some(dt_sort) = self.declared_data_types.get(name) {
            return dt_sort.sort.clone();
        }

        // 2. Check type aliases from registry
        if let Some((_params, underlying)) = self.registry.get_type_alias(name) {
            // Resolve the alias (only for simple aliases without parameters)
            return self.type_expr_to_sort(underlying);
        }

        // 3. Built-in primitive types
        match name {
            // Complex type → Complex datatype sort (exact matches only)
            "ℂ" | "Complex" => {
                if let Some(ref cdt) = self.complex_datatype {
                    cdt.sort.sort.clone()
                } else {
                    Sort::real() // Fallback
                }
            }
            // Real types → Real sort (exact matches only, not single letter R)
            "ℝ" | "Real" | "Scalar" => Sort::real(),
            // Rational types → Real sort (Z3's Real is actually ℚ, not ℝ)
            "ℚ" | "Rational" | "Q" => Sort::real(),
            // Integer types → Int sort (exact matches only)
            "ℤ" | "Int" | "Integer" | "ℕ" | "Nat" | "Natural" => Sort::int(),
            // Boolean → Bool sort
            "Bool" | "Boolean" => Sort::bool(),
            // String → String sort (Z3 Seq)
            "String" | "Str" => Sort::string(),

            // Bitvector types - common widths
            "BitVec8" | "Byte" | "U8" | "I8" => Sort::bitvector(8),
            "BitVec16" | "U16" | "I16" => Sort::bitvector(16),
            "BitVec32" | "U32" | "I32" | "Word" => Sort::bitvector(32),
            "BitVec64" | "U64" | "I64" => Sort::bitvector(64),

            // Set types - Z3 sets are arrays from element type to Bool
            "Set" | "IntSet" => Sort::set(&Sort::int()),
            "RealSet" => Sort::set(&Sort::real()),
            "BoolSet" => Sort::set(&Sort::bool()),

            // 4. Everything else (type variables like S, M, G, R, T, and abstract types) → Int
            // Type variables must all map to the same sort for consistency
            _ => Sort::int(),
        }
    }

    /// Check if an operation returns Bool (based on registry, no heuristics)
    ///
    /// This is ONLY used when the operation signature is not found in the registry.
    /// In a mathematical verifier, we cannot use heuristics - if the type is unknown,
    /// we default to Int (uninterpreted) and log a warning.
    ///
    /// Operations that return Bool MUST be declared with proper type signatures.
    /// Translate quantifier to Z3 with proper forall/exists wrapper
    fn translate_quantifier(
        &mut self,
        quantifier: &QuantifierKind,
        variables: &[QuantifiedVar],
        where_clause: Option<&Expression>,
        body: &Expression,
        vars: &HashMap<String, Dynamic>,
    ) -> Result<Bool, String> {
        // Create Z3 bound variables
        let mut bound_vars: Vec<Dynamic> = Vec::new();
        let mut new_vars = vars.clone();

        for var in variables {
            let z3_var: Dynamic = if let Some(type_annotation) = &var.type_annotation {
                match type_annotation.as_str() {
                    // Boolean types
                    "Bool" | "Boolean" => Bool::fresh_const(&var.name).into(),

                    // Real types
                    "ℝ" | "Real" => Real::fresh_const(&var.name).into(),

                    // Rational types (Z3's Real is actually ℚ)
                    "ℚ" | "Rational" | "Q" => Real::fresh_const(&var.name).into(),

                    // Integer/Natural types
                    "ℤ" | "Int" | "Z" | "Integer" | "ℕ" | "Nat" | "Natural" => {
                        Int::fresh_const(&var.name).into()
                    }

                    // Complex types
                    "ℂ" | "Complex" | "C" => self
                        .fresh_complex_const(&var.name)
                        .unwrap_or_else(|| Int::fresh_const(&var.name).into()),

                    // Bitvector types - common widths
                    "BitVec8" | "Byte" | "U8" | "I8" => {
                        Dynamic::fresh_const(&var.name, &Sort::bitvector(8))
                    }
                    "BitVec16" | "U16" | "I16" => {
                        Dynamic::fresh_const(&var.name, &Sort::bitvector(16))
                    }
                    "BitVec32" | "U32" | "I32" | "Word" => {
                        Dynamic::fresh_const(&var.name, &Sort::bitvector(32))
                    }
                    "BitVec64" | "U64" | "I64" => {
                        Dynamic::fresh_const(&var.name, &Sort::bitvector(64))
                    }

                    // Set types
                    "Set" | "IntSet" => Dynamic::fresh_const(&var.name, &Sort::set(&Sort::int())),
                    "RealSet" => Dynamic::fresh_const(&var.name, &Sort::set(&Sort::real())),
                    "BoolSet" => Dynamic::fresh_const(&var.name, &Sort::set(&Sort::bool())),

                    // String type
                    "String" | "Str" => z3::ast::String::fresh_const(&var.name).into(),

                    type_name => {
                        // Check if it's a declared data type (exact match)
                        if let Some(dt_sort) = self.declared_data_types.get(type_name) {
                            Dynamic::fresh_const(&var.name, &dt_sort.sort)
                        }
                        // Parameterized types: "List(T)" → base "List"
                        else if let Some(base) = type_name.split('(').next() {
                            if let Some(dt_sort) = self.declared_data_types.get(base) {
                                Dynamic::fresh_const(&var.name, &dt_sort.sort)
                            } else {
                                self.add_warning(format!(
                                    "Unknown type '{}' for variable '{}'. Treating as Int.",
                                    type_name, var.name
                                ));
                                Int::fresh_const(&var.name).into()
                            }
                        } else {
                            Int::fresh_const(&var.name).into()
                        }
                    }
                }
            } else {
                Int::fresh_const(&var.name).into()
            };
            bound_vars.push(z3_var.clone());
            // Track for witness extraction: Kleis name → Z3 variable
            self.quantifier_vars
                .push((var.name.clone(), z3_var.clone()));
            new_vars.insert(var.name.clone(), z3_var);
        }

        // Translate body (with optional where clause)
        let body_z3 = if let Some(condition) = where_clause {
            let condition_z3 = self.kleis_to_z3(condition, &new_vars)?;
            let condition_bool = condition_z3
                .as_bool()
                .ok_or_else(|| "Where clause must be boolean".to_string())?;

            let body_dyn = self.kleis_to_z3(body, &new_vars)?;
            let body_bool = body_dyn
                .as_bool()
                .ok_or_else(|| "Quantifier body must be boolean".to_string())?;

            // where_clause ⟹ body
            condition_bool.implies(&body_bool)
        } else {
            let body_dyn = self.kleis_to_z3(body, &new_vars)?;
            body_dyn
                .as_bool()
                .ok_or_else(|| "Quantifier body must be boolean".to_string())?
        };

        // Create proper Z3 forall/exists with bound variables
        let bound_refs: Vec<&dyn Ast> = bound_vars.iter().map(|v| v as &dyn Ast).collect();

        let result = match quantifier {
            QuantifierKind::ForAll => z3::ast::forall_const(&bound_refs, &[], &body_z3),
            QuantifierKind::Exists => z3::ast::exists_const(&bound_refs, &[], &body_z3),
        };

        Ok(result)
    }

    /// Translate match expression to nested Z3 ite
    fn translate_match(
        &mut self,
        scrutinee: &Expression,
        cases: &[crate::ast::MatchCase],
        vars: &HashMap<String, Dynamic>,
    ) -> Result<Dynamic, String> {
        if cases.is_empty() {
            return Err("Match expression must have at least one case".to_string());
        }

        // Translate scrutinee
        let scrutinee_z3 = self.kleis_to_z3(scrutinee, vars)?;

        // Build nested ite from cases (last case is the default)
        // We process cases in reverse to build nested ite
        let mut result: Option<Dynamic> = None;

        for case in cases.iter().rev() {
            // Try to translate this case
            let case_result = self.translate_match_case(
                &scrutinee_z3,
                scrutinee,
                &case.pattern,
                &case.body,
                vars,
            )?;

            match (&result, case_result) {
                (None, body_z3) => {
                    // Last case (or only case) - becomes the else branch
                    result = Some(body_z3);
                }
                (Some(else_branch), body_z3) => {
                    // Build condition for this pattern
                    if let Some(condition) =
                        self.pattern_to_condition(&scrutinee_z3, scrutinee, &case.pattern, vars)?
                    {
                        // ite(condition, body, else_branch)
                        result = Some(boolean::translate_ite(&condition, &body_z3, else_branch));
                    } else {
                        // Wildcard or variable - always matches, replaces else
                        result = Some(body_z3);
                    }
                }
            }
        }

        result.ok_or_else(|| "Failed to translate match expression".to_string())
    }

    /// Translate a single match case
    fn translate_match_case(
        &mut self,
        _scrutinee_z3: &Dynamic,
        scrutinee_expr: &Expression,
        pattern: &crate::ast::Pattern,
        body: &Expression,
        vars: &HashMap<String, Dynamic>,
    ) -> Result<Dynamic, String> {
        // Extend vars with pattern bindings
        let mut extended_vars = vars.clone();
        self.bind_pattern_vars(&mut extended_vars, scrutinee_expr, pattern)?;

        // Translate body with extended bindings
        self.kleis_to_z3(body, &extended_vars)
    }

    /// Bind pattern variables to corresponding parts of scrutinee
    fn bind_pattern_vars(
        &mut self,
        vars: &mut HashMap<String, Dynamic>,
        scrutinee: &Expression,
        pattern: &crate::ast::Pattern,
    ) -> Result<(), String> {
        use crate::ast::Pattern;

        match pattern {
            Pattern::Wildcard => Ok(()),
            Pattern::Variable(name) => {
                // Bind the variable to the scrutinee value
                let scrutinee_z3 = self.kleis_to_z3(scrutinee, vars)?;
                vars.insert(name.clone(), scrutinee_z3);
                Ok(())
            }
            Pattern::Constructor { name: _, args } => {
                // For constructor patterns, we need to extract fields
                // This works when scrutinee is also a constructor application
                if let Expression::Operation {
                    name: _,
                    args: scrutinee_args,
                    ..
                } = scrutinee
                    && args.len() == scrutinee_args.len()
                {
                    for (pat, arg) in args.iter().zip(scrutinee_args.iter()) {
                        self.bind_pattern_vars(vars, arg, pat)?;
                    }
                }
                Ok(())
            }
            Pattern::Constant(_) => {
                // Constants don't bind variables
                Ok(())
            }
            // Grammar v0.8: As-pattern binds alias AND recurses
            Pattern::As {
                pattern: inner,
                binding,
            } => {
                // First bind the whole scrutinee to the alias
                let scrutinee_z3 = self.kleis_to_z3(scrutinee, vars)?;
                vars.insert(binding.clone(), scrutinee_z3);
                // Then recurse into the inner pattern
                self.bind_pattern_vars(vars, scrutinee, inner)
            }
        }
    }

    /// Bind pattern variables from a Z3 value (Grammar v0.8: for let destructuring)
    ///
    /// This function extracts bindings from patterns for use in let expressions.
    /// For constructor patterns like `Point(x, y)`, it destructures the expression
    /// and binds pattern variables to corresponding Z3 values.
    fn bind_pattern_to_z3(
        &mut self,
        pattern: &crate::ast::Pattern,
        z3_value: &Dynamic,
        original_expr: &Expression,
        vars: &mut HashMap<String, Dynamic>,
    ) -> Result<(), String> {
        use crate::ast::Pattern;

        match pattern {
            Pattern::Wildcard => Ok(()),
            Pattern::Variable(name) => {
                vars.insert(name.clone(), z3_value.clone());
                Ok(())
            }
            Pattern::Constructor { name, args } => {
                // Grammar v0.8: Constructor destructuring for let bindings
                // Check if the original expression is an Operation with matching constructor
                match original_expr {
                    Expression::Operation {
                        name: op_name,
                        args: op_args,
                        ..
                    } if op_name == name && op_args.len() == args.len() => {
                        // Recursively bind each pattern argument to the corresponding operation argument
                        for (pat, arg_expr) in args.iter().zip(op_args.iter()) {
                            let arg_z3 = self.kleis_to_z3(arg_expr, vars)?;
                            self.bind_pattern_to_z3(pat, &arg_z3, arg_expr, vars)?;
                        }
                        Ok(())
                    }
                    Expression::Object(var_name) => {
                        // Symbolic variable destructuring: create fresh Z3 variables for fields
                        //
                        // Since we don't have Z3 ADT accessors, we create fresh symbolic variables
                        // to represent "whatever the field values could be". This is sound for
                        // verification: if a property holds for all possible field values, it holds
                        // for the actual (unknown) field values.
                        for (i, pat) in args.iter().enumerate() {
                            let field_var_name = format!("{}_{}_field{}", var_name, name, i);
                            let field_z3: Dynamic = Int::fresh_const(&field_var_name).into();
                            // Create a placeholder expression for recursion
                            let placeholder = Expression::Object(field_var_name.clone());
                            self.bind_pattern_to_z3(pat, &field_z3, &placeholder, vars)?;
                        }
                        Ok(())
                    }
                    _ => {
                        // Other expression types cannot be destructured without Z3 ADT support
                        Err(format!(
                            "Cannot destructure pattern '{}({})' from expression type {:?}. \
                             Constructor destructuring requires a matching Operation or Object.",
                            name,
                            args.len(),
                            std::mem::discriminant(original_expr)
                        ))
                    }
                }
            }
            Pattern::Constant(_) => {
                // Constants don't bind variables
                Ok(())
            }
            Pattern::As {
                pattern: inner,
                binding,
            } => {
                // Bind whole value to alias
                vars.insert(binding.clone(), z3_value.clone());
                // Recurse into inner pattern
                self.bind_pattern_to_z3(inner, z3_value, original_expr, vars)
            }
        }
    }

    /// Convert a pattern to a Z3 boolean condition (None for wildcard/variable)
    fn pattern_to_condition(
        &mut self,
        scrutinee_z3: &Dynamic,
        scrutinee_expr: &Expression,
        pattern: &crate::ast::Pattern,
        vars: &HashMap<String, Dynamic>,
    ) -> Result<Option<Bool>, String> {
        use crate::ast::Pattern;

        match pattern {
            Pattern::Wildcard => Ok(None),    // Always matches
            Pattern::Variable(_) => Ok(None), // Always matches (binds)
            Pattern::Constant(val) => {
                // Check if scrutinee equals the constant
                if let Some(scrutinee_int) = scrutinee_z3.as_int() {
                    if let Ok(n) = val.parse::<i64>() {
                        let const_z3 = Int::from_i64(n);
                        Ok(Some(scrutinee_int.eq(&const_z3)))
                    } else {
                        Ok(None)
                    }
                } else {
                    Ok(None)
                }
            }
            Pattern::Constructor { name, args } => {
                // Check if scrutinee is a constructor with matching name
                if let Expression::Operation {
                    name: scrutinee_name,
                    args: scrutinee_args,
                    ..
                } = scrutinee_expr
                {
                    if scrutinee_name == name && args.len() == scrutinee_args.len() {
                        // Match constructor name - check nested patterns
                        let mut conditions = Vec::new();

                        for (pat, arg) in args.iter().zip(scrutinee_args.iter()) {
                            let arg_z3 = self.kleis_to_z3(arg, vars)?;
                            if let Some(cond) =
                                self.pattern_to_condition(&arg_z3, arg, pat, vars)?
                            {
                                conditions.push(cond);
                            }
                        }

                        if conditions.is_empty() {
                            // All sub-patterns are wildcards/variables
                            Ok(Some(Bool::from_bool(true)))
                        } else {
                            // Combine conditions with AND
                            let mut result = conditions[0].clone();
                            for cond in &conditions[1..] {
                                result = Bool::and(&[&result, cond]);
                            }
                            Ok(Some(result))
                        }
                    } else {
                        // Different constructor - doesn't match
                        Ok(Some(Bool::from_bool(false)))
                    }
                } else if let Expression::Const(val) = scrutinee_expr {
                    // Scrutinee is a literal constant
                    if name == val {
                        Ok(Some(Bool::from_bool(true)))
                    } else {
                        Ok(Some(Bool::from_bool(false)))
                    }
                } else if args.is_empty() {
                    // NULLARY CONSTRUCTOR PATTERN with symbolic scrutinee
                    // This is the key fix for symbolic ADT matching!
                    // Example: match p { Owner => 4 | ... } where p is a variable
                    //
                    // Check if this constructor is a known identity element
                    // If so, compare scrutinee_z3 == identity_element[name]
                    if let Some(constructor_z3) = self.identity_elements.get(name) {
                        // Use Z3 equality to compare the symbolic scrutinee
                        // with the constructor identity element
                        let eq = comparison::translate_equals(scrutinee_z3, constructor_z3)?;
                        Ok(Some(eq))
                    } else {
                        // Constructor not registered as identity element
                        // This shouldn't happen if ADT was properly loaded
                        eprintln!(
                            "   ⚠️ Warning: Constructor '{}' not found in identity elements",
                            name
                        );
                        Ok(None)
                    }
                } else {
                    // LIMITATION: Constructor patterns with arguments on symbolic scrutinees
                    // Example: match p { Cons(x, xs) => ... } where p is a symbolic variable
                    //
                    // Proper handling requires Z3 ADT (Algebraic Data Type) sorts:
                    // 1. Declare datatype: (declare-datatypes ((List T)) ((nil) (cons (head T) (tail List))))
                    // 2. Use accessors: (head p), (tail p)
                    // 3. Use recognizers: (is-cons p)
                    //
                    // Current workaround: Return None, causing match to fall through to else branch
                    // This is correct for verification (conservative) but limits expressiveness
                    eprintln!(
                        "   ⚠️  Limitation: Constructor '{}' with args on symbolic scrutinee not supported",
                        name
                    );
                    Ok(None)
                }
            }
            // Grammar v0.8: As-pattern - just recurse into inner pattern for condition
            Pattern::As { pattern: inner, .. } => {
                self.pattern_to_condition(scrutinee_z3, scrutinee_expr, inner, vars)
            }
        }
    }

    /// Get solver statistics
    pub fn stats(&self) -> SolverStats {
        SolverStats {
            loaded_structures: self.loaded_structures.len(),
            declared_operations: self.declared_ops.len(),
            assertion_count: self.solver.get_assertions().len(),
        }
    }

    // =========================================================================
    // Complex Number Support (Hybrid Translation)
    // =========================================================================

    /// Initialize the complex constant 'i' = complex(0, 1)
    /// NOTE: We don't put 'i' in identity_elements because it conflicts with
    /// 'i' used as a loop variable in Sum/Product tests. Instead, we handle
    /// 'i' specially in translate_object_i() below.
    fn initialize_complex_i(&mut self) {
        // Complex numbers initialized - 'i' is handled specially in translate_object_i()
    }

    /// Get the complex constant i = complex(0, 1)
    fn get_complex_i(&self) -> Option<Dynamic> {
        self.complex_datatype.as_ref().map(|cdt| {
            let zero = Real::from_rational(0, 1);
            let one = Real::from_rational(1, 1);
            cdt.constructor()
                .apply(&[&zero as &dyn Ast, &one as &dyn Ast])
        })
    }

    /// Create a concrete complex number from two Real values
    #[allow(dead_code)]
    fn make_complex(&self, re: &Real, im: &Real) -> Option<Dynamic> {
        self.complex_datatype
            .as_ref()
            .map(|cdt| cdt.constructor().apply(&[re as &dyn Ast, im as &dyn Ast]))
    }

    /// Extract real part from a complex Dynamic
    #[allow(dead_code)]
    fn extract_re(&self, z: &Dynamic) -> Option<Dynamic> {
        self.complex_datatype
            .as_ref()
            .map(|cdt| cdt.accessor_re().apply(&[z as &dyn Ast]))
    }

    /// Extract imaginary part from a complex Dynamic
    #[allow(dead_code)]
    fn extract_im(&self, z: &Dynamic) -> Option<Dynamic> {
        self.complex_datatype
            .as_ref()
            .map(|cdt| cdt.accessor_im().apply(&[z as &dyn Ast]))
    }

    /// Check if a Dynamic is of Complex sort
    fn is_complex_sort(&self, d: &Dynamic) -> bool {
        if let Some(ref cdt) = self.complex_datatype {
            d.sort_kind() == z3::SortKind::Datatype
                && d.get_sort().to_string() == cdt.sort.sort.to_string()
        } else {
            false
        }
    }

    /// Create a fresh Complex constant for quantified variables
    /// Uses Dynamic::fresh_const with Complex sort for proper Z3 bound variables
    fn fresh_complex_const(&self, name: &str) -> Option<Dynamic> {
        self.complex_datatype.as_ref().map(|cdt| {
            // Use Dynamic::fresh_const with the Complex sort
            // This creates a proper Z3 bound variable that works with forall_const
            Dynamic::fresh_const(name, &cdt.sort.sort)
        })
    }

    /// Verify an existential quantifier ∃(vars). body by direct satisfiability check.
    ///
    /// Unlike universals (which negate and check for counterexamples), existentials
    /// are checked directly: translate the body with free variables and check Sat.
    /// - **Sat** → Valid (existential is true), with a **witness** (satisfying assignment)
    /// - **Unsat** → Invalid (no satisfying assignment exists)
    /// - **Unknown** → Z3 can't decide
    fn verify_existential(
        &mut self,
        variables: &[QuantifiedVar],
        body: &Expression,
        where_clause: Option<&Expression>,
    ) -> Result<VerificationResult, String> {
        self.quantifier_vars.clear();
        self.solver.push();

        // Flatten nested existentials: ∃a. ∃b. ∃c. P(a,b,c) → free vars {a,b,c}, body P
        // Without flattening, inner ∃ variables become Z3-bound (exists_const)
        // and don't appear in the model, making witness extraction fail.
        let mut all_vars: Vec<QuantifiedVar> = variables.to_vec();
        let mut where_clauses: Vec<&Expression> = Vec::new();
        if let Some(wc) = where_clause {
            where_clauses.push(wc);
        }
        let mut innermost_body = body;
        while let Expression::Quantifier {
            quantifier: QuantifierKind::Exists,
            variables: inner_vars,
            body: inner_body,
            where_clause: inner_where,
            ..
        } = innermost_body
        {
            all_vars.extend(inner_vars.iter().cloned());
            if let Some(wc) = inner_where {
                where_clauses.push(wc);
            }
            innermost_body = inner_body;
        }

        // Create free variables (NOT bound by exists_const) so they appear in the model
        let mut var_map: HashMap<String, Dynamic> = HashMap::new();
        for var in &all_vars {
            let z3_var: Dynamic = if let Some(type_annotation) = &var.type_annotation {
                match type_annotation.as_str() {
                    "Bool" | "Boolean" => Bool::fresh_const(&var.name).into(),
                    "ℝ" | "Real" => Real::fresh_const(&var.name).into(),
                    "ℚ" | "Rational" | "Q" => Real::fresh_const(&var.name).into(),
                    "ℤ" | "Int" | "Z" | "Integer" | "ℕ" | "Nat" | "Natural" => {
                        Int::fresh_const(&var.name).into()
                    }
                    "String" | "Str" => z3::ast::String::fresh_const(&var.name).into(),
                    type_name => {
                        if let Some(dt_sort) = self.declared_data_types.get(type_name) {
                            Dynamic::fresh_const(&var.name, &dt_sort.sort)
                        } else {
                            Int::fresh_const(&var.name).into()
                        }
                    }
                }
            } else {
                Int::fresh_const(&var.name).into()
            };
            // Track for witness extraction
            self.quantifier_vars
                .push((var.name.clone(), z3_var.clone()));
            var_map.insert(var.name.clone(), z3_var);
        }

        // Translate the innermost body with ALL free variables
        let body_z3 = self.kleis_to_z3(innermost_body, &var_map)?;
        let body_bool = body_z3
            .as_bool()
            .ok_or_else(|| "Existential body must be boolean".to_string())?;

        // Combine all where clauses with the body
        let mut conjuncts: Vec<Bool> = Vec::new();
        for wc in &where_clauses {
            let wc_z3 = self.kleis_to_z3(wc, &var_map)?;
            let wc_bool = wc_z3
                .as_bool()
                .ok_or_else(|| "Where clause must be boolean".to_string())?;
            conjuncts.push(wc_bool);
        }
        conjuncts.push(body_bool);
        let conjunct_refs: Vec<&Bool> = conjuncts.iter().collect();
        let formula = Bool::and(&conjunct_refs);

        // Assert directly (NOT negated) — we want to find a satisfying assignment
        self.solver.assert(&formula);

        let z3_debug = std::env::var("KLEIS_Z3_DEBUG").unwrap_or_default() == "1";
        let timeout_ms: u32 = std::env::var("KLEIS_Z3_TIMEOUT_MS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(5000);
        let wall_timeout =
            std::time::Duration::from_millis((timeout_ms as u64).saturating_add(2000));

        if z3_debug {
            eprintln!(
                "   [Z3 DEBUG] verify_existential: calling solver.check() ({}ms watchdog)...",
                wall_timeout.as_millis()
            );
        }
        let t0 = std::time::Instant::now();
        let check_result = solver_check_with_watchdog(&self.solver, wall_timeout);
        if z3_debug {
            eprintln!(
                "   [Z3 DEBUG] verify_existential: solver.check() returned {:?} in {}ms",
                check_result,
                t0.elapsed().as_millis()
            );
            eprintln!(
                "   [Z3 DEBUG] existential stats:\n{}",
                self.solver.get_statistics()
            );
        }

        let result = match check_result {
            SatResult::Sat => {
                // Existential is satisfiable → Valid, with a witness
                let witness = if let Some(model) = self.solver.get_model() {
                    super::witness::model_to_witness(
                        &model,
                        &self.quantifier_vars,
                        &self.converter,
                        &self.declared_data_types,
                    )
                } else {
                    Witness::from_raw("Satisfiable (no model details)".to_string())
                };
                VerificationResult::ValidWithWitness { witness }
            }
            SatResult::Unsat => {
                // No satisfying assignment → existential is false
                VerificationResult::Invalid {
                    witness: Witness::from_raw(
                        "No satisfying assignment exists for the existential".to_string(),
                    ),
                }
            }
            SatResult::Unknown => {
                let reason = self
                    .solver
                    .get_reason_unknown()
                    .unwrap_or_else(|| "no reason".to_string());
                if z3_debug {
                    eprintln!(
                        "   [Z3 DEBUG] verify_existential: Unknown reason: {}",
                        reason
                    );
                }
                VerificationResult::Unknown
            }
        };

        self.solver.pop(1);
        Ok(result)
    }
}

impl<'r> SolverBackend for Z3Backend<'r> {
    fn name(&self) -> &str {
        "Z3"
    }

    fn capabilities(&self) -> &SolverCapabilities {
        &self.capabilities
    }

    fn verify_axiom(&mut self, axiom: &Expression) -> Result<VerificationResult, String> {
        if self.memout {
            return Ok(VerificationResult::Unknown);
        }

        let z3_debug = std::env::var("KLEIS_Z3_DEBUG").unwrap_or_default() == "1";

        // For existential quantifiers, use direct satisfiability check to extract witnesses.
        // ∃(x,y). P(x,y) — translate body with free variables and check Sat directly.
        // This produces a model with concrete values (the satisfying witness).
        if let Expression::Quantifier {
            quantifier: QuantifierKind::Exists,
            variables,
            body,
            where_clause,
            ..
        } = axiom
        {
            if z3_debug {
                eprintln!("   [Z3 DEBUG] verify_axiom: existential path");
            }
            return self.verify_existential(variables, body, where_clause.as_deref());
        }

        // --- Universal / non-quantified: standard negate-and-check ---
        if z3_debug {
            eprintln!("   [Z3 DEBUG] verify_axiom: universal/negate-and-check path");
        }
        self.quantifier_vars.clear();
        self.solver.push();

        // Translate to Z3 (populates self.quantifier_vars via translate_quantifier)
        let z3_expr = self.kleis_to_z3(axiom, &HashMap::new())?;
        let z3_bool = z3_expr
            .as_bool()
            .ok_or_else(|| "Axiom must be a boolean expression".to_string())?;

        // Assert negation: if ¬φ is Unsat, then φ is Valid
        self.solver.assert(z3_bool.not());

        let timeout_ms: u32 = std::env::var("KLEIS_Z3_TIMEOUT_MS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(5000);
        let wall_timeout =
            std::time::Duration::from_millis((timeout_ms as u64).saturating_add(2000));

        if z3_debug {
            eprintln!(
                "   [Z3 DEBUG] verify_axiom: calling solver.check() ({}ms watchdog)...",
                wall_timeout.as_millis()
            );
        }
        let t0 = std::time::Instant::now();
        let check_result = solver_check_with_watchdog(&self.solver, wall_timeout);
        if z3_debug {
            eprintln!(
                "   [Z3 DEBUG] verify_axiom: solver.check() returned {:?} in {}ms",
                check_result,
                t0.elapsed().as_millis()
            );
            eprintln!(
                "   [Z3 DEBUG] verify stats:\n{}",
                self.solver.get_statistics()
            );
        }

        let result = match check_result {
            SatResult::Unsat => VerificationResult::Valid,
            SatResult::Sat => {
                let witness = if let Some(model) = self.solver.get_model() {
                    super::witness::model_to_witness(
                        &model,
                        &self.quantifier_vars,
                        &self.converter,
                        &self.declared_data_types,
                    )
                } else {
                    Witness::from_raw("No model available".to_string())
                };
                VerificationResult::Invalid { witness }
            }
            SatResult::Unknown => {
                let reason = self
                    .solver
                    .get_reason_unknown()
                    .unwrap_or_else(|| "no reason".to_string());
                if z3_debug {
                    eprintln!("   [Z3 DEBUG] verify_axiom: Unknown reason: {}", reason);
                }
                VerificationResult::Unknown
            }
        };

        self.solver.pop(1);
        Ok(result)
    }

    fn check_satisfiability(&mut self, expr: &Expression) -> Result<SatisfiabilityResult, String> {
        if self.memout {
            return Ok(SatisfiabilityResult::Unknown);
        }
        // Clear quantifier variable tracking for this satisfiability pass
        self.quantifier_vars.clear();

        // Decompose constructor equalities into element-wise conjunctions
        // before translating to Z3. This avoids quantified injectivity axioms
        // and the E-matching divergence they cause.
        let decomposed = expr.decompose_constructor_equalities();

        // Enable List ADT if the expression involves list constructors
        let ops = decomposed.collect_operation_names();
        if ops.contains("cons") || ops.contains("nil") || matches!(expr, Expression::List(_)) {
            self.enable_list_adt();
        }

        let z3_debug = std::env::var("KLEIS_Z3_DEBUG").unwrap_or_default() == "1";

        // Load relevant computational axioms for operations in the expression.
        // Injectivity axioms are skipped — handled by decomposition + List ADT.
        // Skip if axioms were already loaded (e.g., via initialize_from_registry).
        if self.loaded_structures.is_empty()
            && let Err(e) = self.load_axioms_for_expression(&decomposed)
            && z3_debug
        {
            eprintln!("   [Z3 DEBUG] axiom loading warning: {}", e);
        }

        // Use push/pop for incremental solving
        self.solver.push();

        // Translate to Z3 (populates self.quantifier_vars via translate_quantifier)
        let z3_expr = self.kleis_to_z3(&decomposed, &HashMap::new())?;
        let z3_bool = z3_expr
            .as_bool()
            .ok_or_else(|| "Expression must be a boolean proposition".to_string())?;

        // Assert the expression directly (not negated)
        self.solver.assert(&z3_bool);
        let timeout_ms: u32 = std::env::var("KLEIS_Z3_TIMEOUT_MS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(5000);
        let wall_timeout =
            std::time::Duration::from_millis((timeout_ms as u64).saturating_add(2000));
        if z3_debug {
            eprintln!(
                "   [Z3 DEBUG] check_satisfiability: solver.check() with {}ms watchdog...",
                wall_timeout.as_millis()
            );
        }
        let result = match solver_check_with_watchdog(&self.solver, wall_timeout) {
            SatResult::Sat => {
                let witness = if let Some(model) = self.solver.get_model() {
                    if z3_debug {
                        eprintln!("   [Z3 DEBUG] === RAW MODEL ===\n{}", model);
                        eprintln!(
                            "   [Z3 DEBUG] free_variables: {:?}",
                            self.free_variables.keys().collect::<Vec<_>>()
                        );
                        eprintln!(
                            "   [Z3 DEBUG] quantifier_vars: {:?}",
                            self.quantifier_vars
                                .iter()
                                .map(|(n, _)| n.as_str())
                                .collect::<Vec<_>>()
                        );
                        for (name, z3_var) in &self.free_variables {
                            let evald = model.eval(z3_var, true);
                            eprintln!(
                                "   [Z3 DEBUG] free_var '{}': z3_var={}, eval={:?}",
                                name, z3_var, evald
                            );
                        }
                    }
                    let mut all_vars = self.quantifier_vars.clone();
                    for (name, z3_var) in &self.free_variables {
                        all_vars.push((name.clone(), z3_var.clone()));
                    }
                    super::witness::model_to_witness(
                        &model,
                        &all_vars,
                        &self.converter,
                        &self.declared_data_types,
                    )
                } else {
                    Witness::from_raw("Satisfiable (no model details)".to_string())
                };
                SatisfiabilityResult::Satisfiable { witness }
            }
            SatResult::Unsat => SatisfiabilityResult::Unsatisfiable,
            SatResult::Unknown => {
                if std::env::var("KLEIS_Z3_DEBUG").unwrap_or_default() == "1" {
                    let reason = self
                        .solver
                        .get_reason_unknown()
                        .unwrap_or_else(|| "no reason".to_string());
                    eprintln!(
                        "   [Z3 DEBUG] check_satisfiability: Unknown reason: {}",
                        reason
                    );
                }
                SatisfiabilityResult::Unknown
            }
        };

        // Pop the assertion
        self.solver.pop(1);

        Ok(result)
    }

    fn evaluate(&mut self, expr: &Expression) -> Result<Expression, String> {
        // Translate Kleis expression to Z3
        let z3_expr = self.kleis_to_z3(expr, &HashMap::new())?;

        // For evaluation, we need a concrete value
        // Use self.solver which has axioms already asserted
        // Push a scope so we can pop after evaluation
        self.solver.push();

        // For constant expressions, we can try to extract the value directly
        // For symbolic expressions, we need a model

        // Try to get concrete value directly
        if let Some(int_val) = z3_expr.as_int()
            && let Some(value) = int_val.as_i64()
        {
            self.solver.pop(1);
            return Ok(Expression::Const(value.to_string()));
        }

        if let Some(bool_val) = z3_expr.as_bool()
            && let Some(value) = bool_val.as_bool()
        {
            self.solver.pop(1);
            return Ok(Expression::Const(value.to_string()));
        }

        if let Some(real_val) = z3_expr.as_real()
            && let Some((num, den)) = real_val.as_rational()
        {
            self.solver.pop(1);
            if den == 1 {
                return Ok(Expression::Const(num.to_string()));
            } else {
                let decimal = num as f64 / den as f64;
                return Ok(Expression::Const(decimal.to_string()));
            }
        }

        // For symbolic expressions, try to get a satisfying model
        // WARNING: With quantified axioms loaded, Z3's E-matching can cause
        // exponential blowup. The 30-second timeout (set in Z3Backend::new)
        // protects against infinite hangs, but evaluation may still time out.

        // Create a fresh variable and assert it equals our expression
        let result_var = Int::fresh_const("eval_result");

        // Try to cast z3_expr to Int and assert equality
        if let Some(int_expr) = z3_expr.as_int() {
            self.solver.assert(result_var.eq(&int_expr));

            let eval_timeout_ms: u32 = std::env::var("KLEIS_Z3_TIMEOUT_MS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(5000);
            let eval_wall_timeout =
                std::time::Duration::from_millis((eval_timeout_ms as u64).saturating_add(2000));
            match solver_check_with_watchdog(&self.solver, eval_wall_timeout) {
                SatResult::Sat => {
                    let result = self
                        .solver
                        .get_model()
                        .and_then(|model| model.eval(&result_var, true))
                        .map(|evaluated| {
                            let z3_dynamic: Dynamic = evaluated.into();
                            self.converter.to_expression(&z3_dynamic)
                        });
                    self.solver.pop(1);
                    return match result {
                        Some(r) => r,
                        None => {
                            Err("Z3 returned Sat but could not extract model value".to_string())
                        }
                    };
                }
                SatResult::Unsat => {
                    self.solver.pop(1);
                    return Err("Cannot evaluate expression - unsatisfiable".to_string());
                }
                SatResult::Unknown => {
                    let reason = self
                        .solver
                        .get_reason_unknown()
                        .unwrap_or_else(|| "unknown".to_string());
                    self.solver.pop(1);
                    return Err(format!("Cannot evaluate expression (reason: {})", reason));
                }
            }
        }

        self.solver.pop(1);

        Ok(Expression::Const(z3_expr.to_string()))
    }

    fn simplify(&mut self, expr: &Expression) -> Result<Expression, String> {
        // Translate Kleis expression to Z3
        let z3_expr = self.kleis_to_z3(expr, &HashMap::new())?;

        // Use Z3's simplify method
        let simplified = z3_expr.simplify();

        // Convert simplified Z3 expression back to Kleis Expression
        // CRITICAL: This maintains the abstraction boundary!

        // Check if it's a concrete value we can extract
        if let Some(int_val) = simplified.as_int() {
            if let Some(value) = int_val.as_i64() {
                return Ok(Expression::Const(value.to_string()));
            }
            // Large integer or symbolic
            return Ok(Expression::Const(int_val.to_string()));
        }

        if let Some(bool_val) = simplified.as_bool() {
            if let Some(value) = bool_val.as_bool() {
                return Ok(Expression::Const(value.to_string()));
            }
            // Symbolic boolean
            return Ok(Expression::Const(bool_val.to_string()));
        }

        if let Some(real_val) = simplified.as_real() {
            if let Some((num, den)) = real_val.as_rational() {
                if den == 1 {
                    return Ok(Expression::Const(num.to_string()));
                } else {
                    let decimal = num as f64 / den as f64;
                    return Ok(Expression::Const(decimal.to_string()));
                }
            }
            return Ok(Expression::Const(real_val.to_string()));
        }

        // For complex expressions, use the result converter to reconstruct Kleis AST
        self.converter.to_expression(&simplified)
    }

    fn are_equivalent(&mut self, expr1: &Expression, expr2: &Expression) -> Result<bool, String> {
        self.solver.push();

        let z3_expr1 = match self.kleis_to_z3(expr1, &HashMap::new()) {
            Ok(e) => e,
            Err(e) => {
                self.solver.pop(1);
                return Err(e);
            }
        };
        let z3_expr2 = match self.kleis_to_z3(expr2, &HashMap::new()) {
            Ok(e) => e,
            Err(e) => {
                self.solver.pop(1);
                return Err(e);
            }
        };

        let equality = if z3_expr1.sort_kind() == z3_expr2.sort_kind() {
            z3_expr1.eq(&z3_expr2)
        } else {
            let e1_real = z3_expr1
                .as_real()
                .or_else(|| z3_expr1.as_int().map(|i| i.to_real()));
            let e2_real = z3_expr2
                .as_real()
                .or_else(|| z3_expr2.as_int().map(|i| i.to_real()));

            if let (Some(r1), Some(r2)) = (e1_real, e2_real) {
                r1.eq(&r2)
            } else {
                self.solver.pop(1);
                return Err("Cannot compare expressions of incompatible types".to_string());
            }
        };

        self.solver.assert(equality.not());

        let timeout_ms: u32 = std::env::var("KLEIS_Z3_TIMEOUT_MS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(5000);
        let wall_timeout =
            std::time::Duration::from_millis((timeout_ms as u64).saturating_add(2000));
        let check_result = solver_check_with_watchdog(&self.solver, wall_timeout);
        self.solver.pop(1);

        match check_result {
            SatResult::Unsat => Ok(true),
            SatResult::Sat => Ok(false),
            SatResult::Unknown => {
                let reason = self
                    .solver
                    .get_reason_unknown()
                    .unwrap_or_else(|| "unknown".to_string());
                Err(format!(
                    "Equivalence check inconclusive (reason: {})",
                    reason
                ))
            }
        }
    }

    fn load_structure_axioms(
        &mut self,
        structure_name: &str,
        axioms: &[Expression],
    ) -> Result<(), String> {
        if self.memout {
            return Err("Z3 memory exhausted (memout)".to_string());
        }
        if self.loaded_structures.contains(structure_name) {
            return Ok(()); // Already loaded
        }

        for axiom in axioms {
            let z3_expr = self.kleis_to_z3(axiom, &HashMap::new())?;
            if let Some(z3_bool) = z3_expr.as_bool() {
                self.solver.assert(&z3_bool);
            } else {
                return Err(format!(
                    "Axiom in {} is not a boolean expression",
                    structure_name
                ));
            }
        }

        self.loaded_structures.insert(structure_name.to_string());
        Ok(())
    }

    fn check_consistency(&mut self) -> Result<bool, String> {
        if self.memout {
            return Err("Z3 memory exhausted (memout)".to_string());
        }

        let z3_debug = std::env::var("KLEIS_Z3_DEBUG").unwrap_or_default() == "1";
        let timeout_ms: u32 = std::env::var("KLEIS_Z3_TIMEOUT_MS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(5000);

        // Phase 1: Quick check with watchdog
        let wall_timeout =
            std::time::Duration::from_millis((timeout_ms as u64).saturating_add(2000));
        if z3_debug {
            eprintln!(
                "   [Z3 DEBUG] check_consistency Phase 1: solver.check() with {}ms timeout ({}ms watchdog)...",
                timeout_ms,
                wall_timeout.as_millis()
            );
        }
        let t0 = std::time::Instant::now();
        self.solver.push();
        let bare_result = solver_check_with_watchdog(&self.solver, wall_timeout);
        self.solver.pop(1);
        if z3_debug {
            let reason = if matches!(bare_result, SatResult::Unknown) {
                self.solver.get_reason_unknown().unwrap_or_default()
            } else {
                String::new()
            };
            eprintln!(
                "   [Z3 DEBUG] Phase 1 done in {}ms: {:?} {}",
                t0.elapsed().as_millis(),
                bare_result,
                reason
            );
            eprintln!(
                "   [Z3 DEBUG] Phase 1 stats:\n{}",
                self.solver.get_statistics()
            );
        }

        match bare_result {
            SatResult::Sat => return Ok(true),
            SatResult::Unsat => return Ok(false),
            SatResult::Unknown => {
                let reason = self.solver.get_reason_unknown().unwrap_or_default();
                if reason.contains("memout") {
                    self.memout = true;
                    return Err("Z3 memory exhausted (memout) during consistency check".to_string());
                }
            }
        }

        // Phase 2: Retry with extended timeout (3x) and MBQI enabled.
        //
        // Z3's default E-matching is trigger-based and incomplete for
        // universally quantified axioms. MBQI (Model-Based Quantifier
        // Instantiation) is a different algorithm that systematically
        // explores model candidates without needing explicit triggers.
        let phase2_timeout = timeout_ms.saturating_mul(3);
        if z3_debug {
            eprintln!(
                "   [Z3 DEBUG] check_consistency Phase 2: MBQI with {}ms timeout...",
                phase2_timeout
            );
        }
        let t1 = std::time::Instant::now();
        let mut params = z3::Params::new();
        params.set_u32("timeout", phase2_timeout);
        params.set_bool("mbqi", true);
        self.solver.set_params(&params);

        let wall_timeout2 =
            std::time::Duration::from_millis((phase2_timeout as u64).saturating_add(2000));
        self.solver.push();
        let extended_result = solver_check_with_watchdog(&self.solver, wall_timeout2);
        self.solver.pop(1);
        if z3_debug {
            let reason = if matches!(extended_result, SatResult::Unknown) {
                self.solver.get_reason_unknown().unwrap_or_default()
            } else {
                String::new()
            };
            eprintln!(
                "   [Z3 DEBUG] Phase 2 done in {}ms: {:?} {}",
                t1.elapsed().as_millis(),
                extended_result,
                reason
            );
            eprintln!(
                "   [Z3 DEBUG] Phase 2 stats:\n{}",
                self.solver.get_statistics()
            );
        }

        // Restore normal solver parameters
        let mut restore_params = z3::Params::new();
        restore_params.set_u32("timeout", timeout_ms);
        restore_params.set_u32("solver2_timeout", timeout_ms);
        self.solver.set_params(&restore_params);

        match extended_result {
            SatResult::Sat => Ok(true),
            SatResult::Unsat => Ok(false),
            SatResult::Unknown => {
                let reason = self
                    .solver
                    .get_reason_unknown()
                    .unwrap_or_else(|| "timeout or resource limit".to_string());
                if reason.contains("memout") {
                    self.memout = true;
                }
                Err(format!(
                    "Z3 returned Unknown when checking axiom consistency (reason: {})",
                    reason
                ))
            }
        }
    }

    fn push(&mut self) {
        self.solver.push();
    }

    fn pop(&mut self, levels: u32) {
        self.solver.pop(levels);
    }

    fn reset(&mut self) {
        // Create a new solver instance
        self.solver = Solver::new();
        self.declared_ops.clear();
        self.loaded_structures.clear();
        self.identity_elements.clear();
        self.structure_elements.clear();
        self.identity_element_owners.clear();
        self.current_structure_scope = None;
    }

    fn load_identity_element(&mut self, name: &str, type_expr: &TypeExpr) {
        if self.memout {
            return;
        }

        // If a structure scope is active, store in the per-structure map
        if let Some(scope) = self.current_structure_scope.clone() {
            // Check if already loaded in this structure's scope
            if self
                .structure_elements
                .get(&scope)
                .is_some_and(|m| m.contains_key(name))
            {
                return;
            }

            // Compute sort before borrowing the structure map
            let sort = self.type_expr_to_sort(type_expr);
            let z3_const: Dynamic = Dynamic::fresh_const(name, &sort);

            // Collect distinctness constraints against same-structure elements
            {
                if let Some(struct_map) = self.structure_elements.get(&scope) {
                    for existing_z3 in struct_map.values() {
                        if z3_const.get_sort() == existing_z3.get_sort() {
                            #[allow(deprecated)]
                            let distinct = z3_const._eq(existing_z3).not();
                            self.solver.assert(&distinct);
                        }
                    }
                }
            }

            // Insert into per-structure map
            self.structure_elements
                .entry(scope.clone())
                .or_default()
                .insert(name.to_string(), z3_const.clone());

            // Also register globally if no collision
            if !self.identity_elements.contains_key(name) {
                for existing_z3 in self.identity_elements.values() {
                    if z3_const.get_sort() == existing_z3.get_sort() {
                        #[allow(deprecated)]
                        let distinct = z3_const._eq(existing_z3).not();
                        self.solver.assert(&distinct);
                    }
                }
                self.identity_elements.insert(name.to_string(), z3_const);
                self.identity_element_owners.insert(name.to_string(), scope);
            } else {
                let owner = self
                    .identity_element_owners
                    .get(name)
                    .cloned()
                    .unwrap_or_else(|| "<unknown>".to_string());
                if owner != scope {
                    eprintln!(
                        "   ⚠️  Element '{}' in structure '{}' collides with \
                         same-named element in '{}'. \
                         Each structure gets an independent Z3 constant.",
                        name, scope, owner
                    );
                }
            }
            return;
        }

        // No scope active — global registration (original behavior for ADT constructors, etc.)
        if !self.identity_elements.contains_key(name) {
            let sort = self.type_expr_to_sort(type_expr);
            let z3_const: Dynamic = Dynamic::fresh_const(name, &sort);

            // Assert distinct from all existing global identity elements of the same sort
            for existing_z3 in self.identity_elements.values() {
                if z3_const.get_sort() == existing_z3.get_sort() {
                    #[allow(deprecated)]
                    let distinct = z3_const._eq(existing_z3).not();
                    self.solver.assert(&distinct);
                }
            }

            self.identity_elements.insert(name.to_string(), z3_const);
        }
    }

    fn set_structure_scope(&mut self, structure_name: Option<&str>) {
        self.current_structure_scope = structure_name.map(|s| s.to_string());
    }

    fn is_declared_constructor(&self, name: &str) -> bool {
        self.is_declared_constructor_internal(name)
    }

    fn assert_expression(&mut self, expr: &Expression) -> Result<(), String> {
        let z3_expr = self.kleis_to_z3(expr, &HashMap::new())?;
        let z3_bool = z3_expr
            .as_bool()
            .ok_or_else(|| "Expression must be boolean for assertion".to_string())?;
        self.solver.assert(&z3_bool);
        Ok(())
    }

    fn define_function(
        &mut self,
        name: &str,
        params: &[String],
        body: &Expression,
    ) -> Result<(), String> {
        // Create fresh Z3 variables for parameters
        let mut z3_vars = HashMap::new();
        let mut param_ints = Vec::new();

        for param in params {
            let z3_var = Int::fresh_const(param);
            param_ints.push(z3_var.clone());
            z3_vars.insert(param.clone(), z3_var.into());
        }

        // Translate function body
        let body_z3 = self.kleis_to_z3(body, &z3_vars)?;

        // Declare function
        let func_decl = self.declare_uninterpreted(name, params.len());

        // Create application and assert definition
        let ast_args: Vec<&dyn Ast> = param_ints.iter().map(|p| p as &dyn Ast).collect();
        let func_app = func_decl.apply(&ast_args);
        let definition = func_app.eq(&body_z3);
        self.solver.assert(&definition);

        Ok(())
    }
}

/// Get all variables bound by a pattern
fn pattern_bound_variables(pattern: &Pattern) -> Vec<String> {
    match pattern {
        Pattern::Wildcard => vec![],
        Pattern::Variable(name) => vec![name.clone()],
        Pattern::Constructor { args, .. } => {
            args.iter().flat_map(pattern_bound_variables).collect()
        }
        Pattern::Constant(_) => vec![],
        Pattern::As { pattern, binding } => {
            let mut vars = pattern_bound_variables(pattern);
            vars.push(binding.clone());
            vars
        }
    }
}

/// Substitute variables in an expression with their values
/// This is used to expand defined functions before translating to Z3
fn substitute_expr(expr: &Expression, subst: &HashMap<String, Expression>) -> Expression {
    match expr {
        Expression::Object(name) => {
            if let Some(replacement) = subst.get(name) {
                replacement.clone()
            } else {
                expr.clone()
            }
        }
        Expression::Const(_) | Expression::String(_) => expr.clone(),
        Expression::Placeholder { .. } => expr.clone(),
        Expression::Operation { name, args, span } => Expression::Operation {
            name: name.clone(),
            args: args.iter().map(|a| substitute_expr(a, subst)).collect(),
            span: span.clone(),
        },
        Expression::Quantifier {
            quantifier,
            variables,
            where_clause,
            body,
        } => {
            // Don't substitute bound variables
            let mut new_subst = subst.clone();
            for qvar in variables {
                new_subst.remove(&qvar.name);
            }
            Expression::Quantifier {
                quantifier: quantifier.clone(),
                variables: variables.clone(),
                where_clause: where_clause
                    .as_ref()
                    .map(|w| Box::new(substitute_expr(w, &new_subst))),
                body: Box::new(substitute_expr(body, &new_subst)),
            }
        }
        Expression::Conditional {
            condition,
            then_branch,
            else_branch,
            span,
        } => Expression::Conditional {
            condition: Box::new(substitute_expr(condition, subst)),
            then_branch: Box::new(substitute_expr(then_branch, subst)),
            else_branch: Box::new(substitute_expr(else_branch, subst)),
            span: span.clone(),
        },
        Expression::Lambda { params, body, span } => {
            // Don't substitute bound lambda parameters
            let mut new_subst = subst.clone();
            for param in params {
                new_subst.remove(&param.name);
            }
            Expression::Lambda {
                params: params.clone(),
                body: Box::new(substitute_expr(body, &new_subst)),
                span: span.clone(),
            }
        }
        Expression::Let {
            pattern,
            type_annotation,
            value,
            body,
            span,
        } => {
            let new_value = substitute_expr(value, subst);
            // Don't substitute variables bound by the pattern
            let mut new_subst = subst.clone();
            for var_name in pattern_bound_variables(pattern) {
                new_subst.remove(&var_name);
            }
            Expression::Let {
                pattern: pattern.clone(),
                type_annotation: type_annotation.clone(),
                value: Box::new(new_value),
                body: Box::new(substitute_expr(body, &new_subst)),
                span: span.clone(),
            }
        }
        Expression::Match {
            scrutinee,
            cases,
            span,
        } => Expression::Match {
            scrutinee: Box::new(substitute_expr(scrutinee, subst)),
            cases: cases
                .iter()
                .map(|case| {
                    // Don't substitute variables bound by the pattern
                    let mut new_subst = subst.clone();
                    for var_name in pattern_bound_variables(&case.pattern) {
                        new_subst.remove(&var_name);
                    }
                    MatchCase {
                        pattern: case.pattern.clone(),
                        guard: case.guard.as_ref().map(|g| substitute_expr(g, &new_subst)),
                        body: substitute_expr(&case.body, &new_subst),
                    }
                })
                .collect(),
            span: span.clone(),
        },
        Expression::List(items) => {
            Expression::List(items.iter().map(|i| substitute_expr(i, subst)).collect())
        }
        Expression::Ascription {
            expr: inner,
            type_annotation,
        } => Expression::Ascription {
            expr: Box::new(substitute_expr(inner, subst)),
            type_annotation: type_annotation.clone(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_z3_backend_creation() {
        let registry = StructureRegistry::new();
        let backend = Z3Backend::new(&registry);
        assert!(backend.is_ok());
    }

    #[test]
    fn test_backend_name() {
        let registry = StructureRegistry::new();
        let backend = Z3Backend::new(&registry).unwrap();
        assert_eq!(backend.name(), "Z3");
    }

    #[test]
    fn test_capabilities_loaded() {
        let registry = StructureRegistry::new();
        let backend = Z3Backend::new(&registry).unwrap();

        assert!(backend.capabilities().has_operation("plus"));
        assert!(backend.capabilities().has_operation("equals"));
        assert!(backend.capabilities().has_theory("arithmetic"));
    }

    #[test]
    fn test_push_pop_no_panic() {
        let registry = StructureRegistry::new();
        let mut backend = Z3Backend::new(&registry).unwrap();

        backend.push();
        backend.pop(1);
    }

    /// Helper: assert a boolean expression into the solver.
    fn assert_bool(backend: &mut Z3Backend, expr: &Expression) {
        backend
            .assert_expression(expr)
            .expect("assert_expression failed");
    }

    /// Helper: build `x = c` where x is a free Int variable and c is a constant.
    fn eq_const(var: &str, val: &str) -> Expression {
        Expression::Operation {
            name: "equals".to_string(),
            args: vec![
                Expression::Object(var.to_string()),
                Expression::Const(val.to_string()),
            ],
            span: None,
        }
    }

    /// Helper: build an existential `∃(x:ℤ). body`.
    fn exists_int(var: &str, body: Expression) -> Expression {
        Expression::Quantifier {
            quantifier: QuantifierKind::Exists,
            variables: vec![QuantifiedVar {
                name: var.to_string(),
                type_annotation: Some("ℤ".to_string()),
            }],
            body: Box::new(body),
            where_clause: None,
        }
    }

    /// Helper: build `a AND b`.
    fn and_expr(a: Expression, b: Expression) -> Expression {
        Expression::Operation {
            name: "and".to_string(),
            args: vec![a, b],
            span: None,
        }
    }

    /// Helper: returns true if the solver considers the expression valid
    /// (holds for all models, or satisfiable for existentials).
    fn is_valid(backend: &mut Z3Backend, expr: &Expression) -> bool {
        match backend.verify_axiom(expr) {
            Ok(VerificationResult::Valid) | Ok(VerificationResult::ValidWithWitness { .. }) => true,
            _ => false,
        }
    }

    // ---- Z3 push/pop behavior tests ----
    // These tests empirically verify push/pop semantics instead of assuming them.

    /// Verify that an assertion made at the base level persists (baseline).
    #[test]
    fn test_z3_base_assertion_persists() {
        let registry = StructureRegistry::new();
        let mut backend = Z3Backend::new(&registry).unwrap();

        // Assert x = 5 at base level
        assert_bool(&mut backend, &eq_const("x", "5"));

        // ∃(y:ℤ). x = 5 ∧ y = y  — should be satisfiable because x = 5 is asserted
        let check = exists_int("y", and_expr(eq_const("x", "5"), eq_const("y", "0")));
        assert!(
            is_valid(&mut backend, &check),
            "Base-level assertion should be visible to verify_axiom"
        );
    }

    /// Verify that pop(1) removes assertions made after the matching push().
    #[test]
    fn test_z3_pop_removes_assertions_after_push() {
        let registry = StructureRegistry::new();
        let mut backend = Z3Backend::new(&registry).unwrap();

        backend.push();
        // Assert x = 5 inside the pushed scope
        assert_bool(&mut backend, &eq_const("x", "5"));
        backend.pop(1);

        // After pop, x = 5 should no longer be asserted.
        // So ∃(y:ℤ). x = 7 ∧ y = 0 should be satisfiable (x is unconstrained).
        let check = exists_int("y", and_expr(eq_const("x", "7"), eq_const("y", "0")));
        assert!(
            is_valid(&mut backend, &check),
            "After pop, the assertion x=5 should be gone; x=7 should be satisfiable"
        );
    }

    /// Verify that assertions made BEFORE push() survive pop().
    #[test]
    fn test_z3_pre_push_assertions_survive_pop() {
        let registry = StructureRegistry::new();
        let mut backend = Z3Backend::new(&registry).unwrap();

        // Assert x = 5 at base level
        assert_bool(&mut backend, &eq_const("x", "5"));

        backend.push();
        // Assert y = 10 inside pushed scope
        assert_bool(&mut backend, &eq_const("y", "10"));
        backend.pop(1);

        // x = 5 should still hold after pop
        let check = exists_int("z", eq_const("x", "5"));
        assert!(
            is_valid(&mut backend, &check),
            "Assertion before push should survive pop"
        );

        // y = 10 should NOT hold after pop — y is unconstrained again
        let check2 = exists_int("z", eq_const("y", "99"));
        assert!(
            is_valid(&mut backend, &check2),
            "Assertion inside push scope should be gone after pop; y=99 should be satisfiable"
        );
    }

    /// Verify that push without pop leaves assertions in place.
    /// This is the pattern used in ensure_structure_loaded on success.
    #[test]
    fn test_z3_push_without_pop_keeps_assertions() {
        let registry = StructureRegistry::new();
        let mut backend = Z3Backend::new(&registry).unwrap();

        backend.push();
        assert_bool(&mut backend, &eq_const("x", "42"));
        // Deliberately do NOT pop — this is the success path

        // x = 42 should still be visible
        let check = exists_int("y", eq_const("x", "42"));
        assert!(
            is_valid(&mut backend, &check),
            "Assertions after push without pop should remain visible"
        );
    }

    /// Verify that a later pop removes only the innermost scope's assertions,
    /// not the ones from an earlier push that was left open.
    #[test]
    fn test_z3_nested_push_pop_scoping() {
        let registry = StructureRegistry::new();
        let mut backend = Z3Backend::new(&registry).unwrap();

        // Level 0: assert a = 1
        assert_bool(&mut backend, &eq_const("a", "1"));

        // Level 1: push + assert b = 2 (left open, simulating successful load)
        backend.push();
        assert_bool(&mut backend, &eq_const("b", "2"));

        // Level 2: push + assert c = 3, then pop (simulating failed load rollback)
        backend.push();
        assert_bool(&mut backend, &eq_const("c", "3"));
        backend.pop(1);

        // a = 1 should still hold (base level)
        let check_a = exists_int("z", eq_const("a", "1"));
        assert!(
            is_valid(&mut backend, &check_a),
            "Base assertion a=1 should survive"
        );

        // b = 2 should still hold (level 1, not popped)
        let check_b = exists_int("z", eq_const("b", "2"));
        assert!(
            is_valid(&mut backend, &check_b),
            "Level-1 assertion b=2 should survive (not popped)"
        );

        // c = 3 should be gone (level 2, popped)
        let check_c_free = exists_int("z", eq_const("c", "99"));
        assert!(
            is_valid(&mut backend, &check_c_free),
            "Level-2 assertion c=3 should be gone after pop; c=99 should be satisfiable"
        );
    }

    /// Verify that a contradictory assertion inside push() makes the solver UNSAT
    /// within that scope, but after pop() the solver recovers.
    #[test]
    fn test_z3_contradiction_inside_push_recovers_after_pop() {
        let registry = StructureRegistry::new();
        let mut backend = Z3Backend::new(&registry).unwrap();

        // Assert x = 5 at base level
        assert_bool(&mut backend, &eq_const("x", "5"));

        backend.push();
        // Add contradiction: x = 5 AND x = 7 (impossible)
        assert_bool(&mut backend, &eq_const("x", "7"));
        // Solver is now UNSAT inside this scope
        backend.pop(1);

        // After pop, the contradiction is gone. x = 5 should still hold.
        let check = exists_int("y", eq_const("x", "5"));
        assert!(
            is_valid(&mut backend, &check),
            "After popping a contradictory scope, solver should recover and x=5 should hold"
        );
    }

    /// Verify pop(2) removes two levels of assertions at once.
    #[test]
    fn test_z3_pop_multiple_levels() {
        let registry = StructureRegistry::new();
        let mut backend = Z3Backend::new(&registry).unwrap();

        // Base: a = 1
        assert_bool(&mut backend, &eq_const("a", "1"));

        // Level 1: b = 2
        backend.push();
        assert_bool(&mut backend, &eq_const("b", "2"));

        // Level 2: c = 3
        backend.push();
        assert_bool(&mut backend, &eq_const("c", "3"));

        // Pop both levels at once
        backend.pop(2);

        // a = 1 should survive (base)
        let check_a = exists_int("z", eq_const("a", "1"));
        assert!(
            is_valid(&mut backend, &check_a),
            "Base assertion a=1 should survive pop(2)"
        );

        // b and c should both be gone
        let check_b_free = exists_int("z", eq_const("b", "99"));
        assert!(
            is_valid(&mut backend, &check_b_free),
            "Level-1 assertion b=2 should be gone after pop(2)"
        );

        let check_c_free = exists_int("z", eq_const("c", "99"));
        assert!(
            is_valid(&mut backend, &check_c_free),
            "Level-2 assertion c=3 should be gone after pop(2)"
        );
    }

    /// Verify the exact pattern used in ensure_structure_loaded:
    ///   push → load succeeds → DON'T pop (assertions persist)
    ///   push → load fails → pop (assertions rolled back)
    /// Then check that a subsequent verify_axiom sees the successful load's
    /// axioms but NOT the failed load's partial axioms.
    #[test]
    fn test_z3_ensure_structure_loaded_pattern() {
        let registry = StructureRegistry::new();
        let mut backend = Z3Backend::new(&registry).unwrap();

        // Simulate successful load of structure A: a = 100
        backend.push();
        assert_bool(&mut backend, &eq_const("a", "100"));
        // Success: don't pop

        // Simulate failed load of structure B: b = 200 (partial), then error → pop
        backend.push();
        assert_bool(&mut backend, &eq_const("b", "200"));
        backend.pop(1); // Rollback

        // Simulate successful load of structure C: c = 300
        backend.push();
        assert_bool(&mut backend, &eq_const("c", "300"));
        // Success: don't pop

        // a = 100 should hold (successful load A)
        let check_a = exists_int("z", eq_const("a", "100"));
        assert!(
            is_valid(&mut backend, &check_a),
            "Successful load A's axioms should persist"
        );

        // c = 300 should hold (successful load C)
        let check_c = exists_int("z", eq_const("c", "300"));
        assert!(
            is_valid(&mut backend, &check_c),
            "Successful load C's axioms should persist"
        );

        // b should be unconstrained (failed load B was rolled back)
        let check_b_free = exists_int("z", eq_const("b", "999"));
        assert!(
            is_valid(&mut backend, &check_b_free),
            "Failed load B's axioms should be rolled back; b=999 should be satisfiable"
        );
    }

    #[test]
    fn test_evaluate_returns_kleis_ast() {
        let registry = StructureRegistry::new();
        let mut backend = Z3Backend::new(&registry).unwrap();

        // Simple arithmetic: 2 + 3
        let expr = Expression::Operation {
            name: "plus".to_string(),
            args: vec![
                Expression::Const("2".to_string()),
                Expression::Const("3".to_string()),
            ],
            span: None,
        };

        let result = backend.evaluate(&expr).unwrap();

        // Result MUST be Kleis Expression, not Z3 type!
        match result {
            Expression::Const(s) => {
                assert_eq!(s, "5", "2 + 3 should evaluate to 5");
            }
            _ => panic!("Expected Expression::Const, got {:?}", result),
        }
    }

    #[test]
    fn test_simplify_returns_kleis_ast() {
        let registry = StructureRegistry::new();
        let mut backend = Z3Backend::new(&registry).unwrap();

        // Expression: x + 0 (should simplify to x in ideal case, but at minimum returns Expression)
        let expr = Expression::Operation {
            name: "plus".to_string(),
            args: vec![
                Expression::Const("42".to_string()),
                Expression::Const("0".to_string()),
            ],
            span: None,
        };

        let result = backend.simplify(&expr).unwrap();

        // Result MUST be Kleis Expression, not Z3 type!
        match result {
            Expression::Const(s) => {
                assert_eq!(s, "42", "42 + 0 should simplify to 42");
            }
            _ => panic!("Expected Expression::Const, got {:?}", result),
        }
    }

    #[test]
    fn test_evaluate_concrete_constant() {
        let registry = StructureRegistry::new();
        let mut backend = Z3Backend::new(&registry).unwrap();

        // Already a constant
        let expr = Expression::Const("123".to_string());
        let result = backend.evaluate(&expr).unwrap();

        assert_eq!(result, Expression::Const("123".to_string()));
    }

    #[test]
    fn test_conditional_true_branch() {
        let registry = StructureRegistry::new();
        let mut backend = Z3Backend::new(&registry).unwrap();

        // if true then 42 else 0
        let expr = Expression::Conditional {
            condition: Box::new(Expression::Operation {
                name: "equals".to_string(),
                args: vec![
                    Expression::Const("1".to_string()),
                    Expression::Const("1".to_string()),
                ],
                span: None,
            }),
            then_branch: Box::new(Expression::Const("42".to_string())),
            else_branch: Box::new(Expression::Const("0".to_string())),
            span: None,
        };

        let result = backend.evaluate(&expr).unwrap();
        assert_eq!(result, Expression::Const("42".to_string()));
    }

    #[test]
    fn test_conditional_false_branch() {
        let registry = StructureRegistry::new();
        let mut backend = Z3Backend::new(&registry).unwrap();

        // if false then 42 else 0
        let expr = Expression::Conditional {
            condition: Box::new(Expression::Operation {
                name: "equals".to_string(),
                args: vec![
                    Expression::Const("1".to_string()),
                    Expression::Const("2".to_string()),
                ],
                span: None,
            }),
            then_branch: Box::new(Expression::Const("42".to_string())),
            else_branch: Box::new(Expression::Const("0".to_string())),
            span: None,
        };

        let result = backend.evaluate(&expr).unwrap();
        assert_eq!(result, Expression::Const("0".to_string()));
    }

    #[test]
    fn test_conditional_with_arithmetic() {
        let registry = StructureRegistry::new();
        let mut backend = Z3Backend::new(&registry).unwrap();

        // if 5 > 3 then 10 + 1 else 20 + 1
        let expr = Expression::Conditional {
            condition: Box::new(Expression::Operation {
                name: "greater_than".to_string(),
                args: vec![
                    Expression::Const("5".to_string()),
                    Expression::Const("3".to_string()),
                ],
                span: None,
            }),
            then_branch: Box::new(Expression::Operation {
                name: "plus".to_string(),
                args: vec![
                    Expression::Const("10".to_string()),
                    Expression::Const("1".to_string()),
                ],
                span: None,
            }),
            else_branch: Box::new(Expression::Operation {
                name: "plus".to_string(),
                args: vec![
                    Expression::Const("20".to_string()),
                    Expression::Const("1".to_string()),
                ],
                span: None,
            }),
            span: None,
        };

        let result = backend.evaluate(&expr).unwrap();
        assert_eq!(result, Expression::Const("11".to_string()));
    }

    #[test]
    fn test_conditional_nested() {
        let registry = StructureRegistry::new();
        let mut backend = Z3Backend::new(&registry).unwrap();

        // if 1 > 2 then 100 else (if 2 > 1 then 200 else 300)
        let expr = Expression::Conditional {
            condition: Box::new(Expression::Operation {
                name: "greater_than".to_string(),
                args: vec![
                    Expression::Const("1".to_string()),
                    Expression::Const("2".to_string()),
                ],
                span: None,
            }),
            then_branch: Box::new(Expression::Const("100".to_string())),
            else_branch: Box::new(Expression::Conditional {
                condition: Box::new(Expression::Operation {
                    name: "greater_than".to_string(),
                    args: vec![
                        Expression::Const("2".to_string()),
                        Expression::Const("1".to_string()),
                    ],
                    span: None,
                }),
                then_branch: Box::new(Expression::Const("200".to_string())),
                else_branch: Box::new(Expression::Const("300".to_string())),
                span: None,
            }),
            span: None,
        };

        let result = backend.evaluate(&expr).unwrap();
        assert_eq!(result, Expression::Const("200".to_string()));
    }

    #[test]
    fn test_simplify_conditional() {
        let registry = StructureRegistry::new();
        let mut backend = Z3Backend::new(&registry).unwrap();

        // if true then 5 else 10 should simplify to 5
        let expr = Expression::Conditional {
            condition: Box::new(Expression::Operation {
                name: "equals".to_string(),
                args: vec![
                    Expression::Const("1".to_string()),
                    Expression::Const("1".to_string()),
                ],
                span: None,
            }),
            then_branch: Box::new(Expression::Const("5".to_string())),
            else_branch: Box::new(Expression::Const("10".to_string())),
            span: None,
        };

        let result = backend.simplify(&expr).unwrap();
        assert_eq!(result, Expression::Const("5".to_string()));
    }

    #[test]
    fn test_float_literal_translation() {
        let registry = StructureRegistry::new();
        let mut backend = Z3Backend::new(&registry).unwrap();

        // Float literal "1.0" should be translated to a Z3 Real
        let expr = Expression::Const("1.0".to_string());
        let z3_result = backend.kleis_to_z3(&expr, &HashMap::new());
        assert!(
            z3_result.is_ok(),
            "Float literal 1.0 should translate successfully"
        );

        let dynamic = z3_result.unwrap();
        assert!(
            dynamic.as_real().is_some(),
            "Float literal 1.0 should produce a Z3 Real sort"
        );
    }

    #[test]
    fn test_float_literal_zero() {
        let registry = StructureRegistry::new();
        let mut backend = Z3Backend::new(&registry).unwrap();

        let expr = Expression::Const("0.0".to_string());
        let z3_result = backend.kleis_to_z3(&expr, &HashMap::new());
        assert!(
            z3_result.is_ok(),
            "Float literal 0.0 should translate successfully"
        );

        let dynamic = z3_result.unwrap();
        assert!(
            dynamic.as_real().is_some(),
            "Float literal 0.0 should produce a Z3 Real sort"
        );
    }

    #[test]
    fn test_int_to_real_coercion_in_uninterpreted_function() {
        use crate::kleis_ast::TypeExpr;

        // Register an operation with Real → Real signature to simulate
        // the real-world case of `neg_cos : ℝ → ℝ` being called as neg_cos(0).
        let mut registry = StructureRegistry::new();
        registry.register_toplevel_operation(
            "test_real_fn".to_string(),
            TypeExpr::Function(
                Box::new(TypeExpr::Named("ℝ".to_string())),
                Box::new(TypeExpr::Named("ℝ".to_string())),
            ),
        );
        let mut backend = Z3Backend::new(&registry).unwrap();

        // Call with an Int literal — the coercion should promote Int → Real
        let expr_int_arg = Expression::Operation {
            name: "test_real_fn".to_string(),
            args: vec![Expression::Const("0".to_string())],
            span: None,
        };
        let result = backend.kleis_to_z3(&expr_int_arg, &HashMap::new());
        assert!(
            result.is_ok(),
            "Int arg should be auto-promoted to Real when function expects Real: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_type_mismatch_rejected_for_incompatible_sorts() {
        let registry = StructureRegistry::new();
        let mut backend = Z3Backend::new(&registry).unwrap();

        // Declare function with Int arg
        let expr_int = Expression::Operation {
            name: "test_strict_fn".to_string(),
            args: vec![Expression::Const("42".to_string())],
            span: None,
        };
        let first = backend.kleis_to_z3(&expr_int, &HashMap::new());
        assert!(first.is_ok(), "First call with Int arg should succeed");

        // Call with String arg — should fail (String != Int, not auto-promotable)
        let expr_str = Expression::Operation {
            name: "test_strict_fn".to_string(),
            args: vec![Expression::String("hello".to_string())],
            span: None,
        };
        let second = backend.kleis_to_z3(&expr_str, &HashMap::new());
        assert!(
            second.is_err(),
            "String arg where Int expected should produce a type mismatch error"
        );
    }

    /// Test that Z3 can detect axiom inconsistency via bare solver.check().
    ///
    /// Reproduces the POT non_separable bug: flow_add_id + non_separable
    /// creates a trivial contradiction. Tests multiple approaches to
    /// determine which reliably triggers Z3's E-matching.
    #[test]
    fn test_consistency_check_detects_quantifier_inconsistency() {
        let solver = Solver::new();

        let flow_sort = Sort::uninterpreted("Flow".into());
        let gk_sort = Sort::uninterpreted("GK".into());
        let da_sort = Sort::uninterpreted("DA".into());
        let sf_sort = Sort::uninterpreted("SF".into());
        let bool_sort = Sort::bool();

        let is_admissible = FuncDecl::new("is_admissible", &[&gk_sort], &bool_sort);
        let flow_add = FuncDecl::new("flow_add", &[&flow_sort, &flow_sort], &flow_sort);
        let project_at = FuncDecl::new("project_at", &[&gk_sort, &flow_sort, &da_sort], &sf_sort);

        let flow_zero = Dynamic::fresh_const("flow_zero", &flow_sort);
        let psi_ab = Dynamic::fresh_const("psi_AB", &flow_sort);
        let k_univ = Dynamic::fresh_const("K_univ", &gk_sort);

        // Axiom 1: ∀(a : Flow). flow_add(a, flow_zero) = a
        let a_flow = Dynamic::fresh_const("a", &flow_sort);
        let flow_add_a_zero = flow_add.apply(&[&a_flow, &flow_zero]);
        let axiom1 = z3::ast::forall_const(&[&a_flow], &[], &a_flow.eq(&flow_add_a_zero));
        solver.assert(&axiom1);

        // Axiom 2: is_admissible(K_univ)
        solver.assert(&is_admissible.apply(&[&k_univ]).as_bool().unwrap());

        // Axiom 3: non_separable — the buggy axiom
        let pa = Dynamic::fresh_const("pA", &flow_sort);
        let pb = Dynamic::fresh_const("pB", &flow_sort);
        let g_var = Dynamic::fresh_const("G", &gk_sort);
        let a_var = Dynamic::fresh_const("a", &da_sort);
        let b_var = Dynamic::fresh_const("b", &da_sort);

        let g_admissible = is_admissible.apply(&[&g_var]).as_bool().unwrap();
        let sum = flow_add.apply(&[&pa, &pb]);
        let proj_psi_a = project_at.apply(&[&g_var, &psi_ab, &a_var]);
        let proj_sum_a = project_at.apply(&[&g_var, &sum, &a_var]);
        let proj_psi_b = project_at.apply(&[&g_var, &psi_ab, &b_var]);
        let proj_sum_b = project_at.apply(&[&g_var, &sum, &b_var]);

        let conj = Bool::and(&[&proj_psi_a.eq(&proj_sum_a), &proj_psi_b.eq(&proj_sum_b)]);
        let non_sep_body = g_admissible.implies(&conj.not());
        let axiom3 = z3::ast::forall_const(&[&pa, &pb, &g_var, &a_var, &b_var], &[], &non_sep_body);
        solver.assert(&axiom3);

        // TEST 1: Bare check()
        let bare_result = solver.check();
        eprintln!("TEST 1 - Bare solver.check(): {:?}", bare_result);

        // TEST 2: Seed with ground term flow_add(psi_AB, flow_zero)
        solver.push();
        let ground_sum = flow_add.apply(&[&psi_ab, &flow_zero]);
        solver.assert(&psi_ab.eq(&ground_sum));
        let seeded_result = solver.check();
        eprintln!(
            "TEST 2 - Seeded with flow_add(psi_AB, flow_zero)=psi_AB: {:?}",
            seeded_result
        );
        solver.pop(1);

        // TEST 3: Seed with project_at ground terms too
        solver.push();
        let ground_sum3 = flow_add.apply(&[&psi_ab, &flow_zero]);
        solver.assert(&psi_ab.eq(&ground_sum3));
        let da_c = Dynamic::fresh_const("da_c", &da_sort);
        let da_d = Dynamic::fresh_const("da_d", &da_sort);
        let _p1 = project_at.apply(&[&k_univ, &psi_ab, &da_c]);
        let _p2 = project_at.apply(&[&k_univ, &ground_sum3, &da_d]);
        let full_seed_result = solver.check();
        eprintln!("TEST 3 - Full ground seeding: {:?}", full_seed_result);
        solver.pop(1);

        // TEST 4: Canary 1≠2
        solver.push();
        let one = Int::from_i64(1);
        let two = Int::from_i64(2);
        solver.assert(&one.eq(&two).not());
        let canary_result = solver.check();
        eprintln!("TEST 4 - Canary (1≠2): {:?}", canary_result);
        solver.pop(1);

        eprintln!(
            "\nSummary: bare={:?}, seeded={:?}, full_seed={:?}, canary={:?}",
            bare_result, seeded_result, full_seed_result, canary_result
        );

        let any_detected = bare_result == SatResult::Unsat
            || seeded_result == SatResult::Unsat
            || full_seed_result == SatResult::Unsat
            || canary_result == SatResult::Unsat;

        assert!(
            any_detected,
            "Z3 should detect the axiom inconsistency via at least one method"
        );
    }
}
