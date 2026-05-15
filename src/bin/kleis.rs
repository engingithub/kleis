//! Kleis - Unified Binary
//!
//! This is the main entry point for Kleis, providing multiple modes:
//!
//! - `kleis server` - Unified server (LSP + DAP + REPL) for IDE integration
//! - `kleis eval` - Evaluate expressions from command line
//! - `kleis check` - Check a file for parse/type errors
//! - `kleis repl` - Interactive REPL in terminal
//!
//! ## Usage
//!
//! ```bash
//! # IDE integration (VS Code, etc.)
//! kleis server
//!
//! # Command line evaluation
//! kleis eval "1 + 2"
//! kleis eval -f script.kleis
//!
//! # Check files
//! kleis check myfile.kleis
//!
//! # Interactive REPL
//! kleis repl
//! kleis repl --load stdlib/prelude.kleis
//! ```

use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};

/// Kleis - A symbolic mathematics language
#[derive(Parser)]
#[command(name = "kleis")]
#[command(author, version, about, long_about = None)]
#[command(after_long_help = "\
ENVIRONMENT VARIABLES (Z3 solver):
  KLEIS_Z3_DEBUG=1          Enable Z3 debug mode: per-axiom timing, quantifier
                            instantiation profiling, solver statistics after each
                            check, and get_reason_unknown() diagnostics.

  KLEIS_Z3_TIMEOUT_MS=N     Set Z3 solver timeout in milliseconds (default: 0 = none).
                            CAUTION: Z3 can crash internally when a global timeout
                            fires mid-processing. The watchdog is the safe timeout.
                            Only set for diagnosing specific divergence scenarios.

  KLEIS_Z3_RLIMIT=N         Set Z3 deterministic resource limit in work units
                            (default: 0 = unlimited). Unlike timeouts, rlimit is
                            reproducible across runs. Try 5000000 for debugging.

  KLEIS_Z3_MEMORY_MB=N      Set Z3 memory limit in megabytes (default: 0 = unlimited).
                            Z3 legitimately uses several GB for complex theories.
                            Only set for diagnostics: Z3 returns Unknown(memout)
                            gracefully instead of crashing when the limit is hit.

WATCHDOG:
  Every solver.check() call is protected by a scoped watchdog thread that calls
  Context::interrupt() if Z3 exceeds the wall-clock limit (timeout + 2 seconds).
  This prevents hangs from quantifier instantiation loops that ignore Z3's
  internal timeout. Interrupted checks return Unknown with reason \"canceled\".

EXAMPLES:
  kleis test file.kleis                         # Normal test run
  KLEIS_Z3_DEBUG=1 kleis test file.kleis        # With full Z3 diagnostics
  KLEIS_Z3_TIMEOUT_MS=2000 kleis test file.kleis  # Faster timeout (2s)
  KLEIS_Z3_RLIMIT=1000000 kleis test file.kleis   # Cap solver work units
")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the unified server (LSP + DAP) for IDE integration
    Server {
        /// Enable verbose logging
        #[arg(short, long)]
        verbose: bool,
    },

    /// Evaluate an expression and print the result
    Eval {
        /// Expression to evaluate
        expression: Option<String>,

        /// File to evaluate
        #[arg(short, long)]
        file: Option<PathBuf>,
    },

    /// Check a file for parse and type errors
    Check {
        /// File to check
        file: PathBuf,
    },

    /// Run example blocks as tests (v0.93)
    Test {
        /// File containing example blocks to test
        file: PathBuf,

        /// Run only examples matching this name
        #[arg(short, long)]
        example: Option<String>,

        /// Show detailed output for each example
        #[arg(short, long)]
        verbose: bool,

        /// Emit raw output (no summary banners, strings unquoted)
        #[arg(long)]
        raw_output: bool,
    },

    /// Start an interactive REPL
    Repl {
        /// Files to load on startup
        #[arg(short, long)]
        load: Vec<PathBuf>,
    },

    /// Start standalone DAP server over stdio (for IDE debugging without LSP)
    Dap {
        /// Enable verbose logging (to stderr)
        #[arg(short, long)]
        verbose: bool,
    },

    /// Start MCP (Model Context Protocol) server for formal policy enforcement
    ///
    /// Exposes Kleis formal verification as tools that LLM agents must call
    /// before performing actions. Uses JSON-RPC 2.0 over stdio.
    Mcp {
        /// Path to the Kleis policy file
        #[arg(short, long)]
        policy: PathBuf,

        /// Enable verbose logging (to stderr)
        #[arg(short, long)]
        verbose: bool,
    },

    /// Start Theory MCP server for interactive theory building (ADR-031)
    ///
    /// Lets agents co-author Kleis theories interactively, submitting
    /// structures, definitions, and data types with Z3 consistency checking.
    TheoryMcp {
        /// Enable verbose logging (to stderr)
        #[arg(short, long)]
        verbose: bool,
    },

    /// Start Review MCP server for code review via formal coding standards
    ///
    /// Checks source code against formal coding standards defined in a Kleis
    /// policy file. Agents or CI can submit code snippets for review.
    ReviewMcp {
        /// Path to the Kleis review policy file
        #[arg(short, long)]
        policy: PathBuf,

        /// Enable verbose logging (to stderr)
        #[arg(short, long)]
        verbose: bool,
    },

    /// Review source files against formal coding standards (CLI / CI mode)
    ///
    /// Runs all check_* rules from the policy against each file and prints
    /// pass/fail results. Exits with code 1 if any file fails.
    /// Suitable for GitLab CI/CD, GitHub Actions, or pre-commit hooks.
    Review {
        /// Source files to review
        files: Vec<PathBuf>,

        /// Path to the Kleis review policy file
        #[arg(short, long)]
        policy: PathBuf,

        /// Show only failed checks (suppress passing rules)
        #[arg(short, long)]
        failures_only: bool,

        /// Enable verbose logging (to stderr)
        #[arg(short, long)]
        verbose: bool,

        /// Run LLM advisory review after formal checks (requires KLEIS_LLM_API_KEY)
        #[arg(short, long)]
        advise: bool,

        /// Base branch for diff-aware rules (e.g., main, origin/main).
        /// When provided, diff_check_* functions receive both current and base file content.
        /// Accepts any git ref: branch, tag, SHA, or remote ref.
        #[arg(short = 'B', long)]
        base_branch: Option<String>,

        /// Extra file paths to include in the review (repeatable).
        /// Use for non-source files like Cargo.toml or requirements.txt.
        #[arg(long = "include")]
        include: Vec<PathBuf>,

        /// Auto-discover extra files from the policy's review_extra_files() function.
        /// Resolves patterns relative to the git repo root of the target files.
        /// Enabled by default when the policy defines review_extra_files().
        #[arg(long, default_value_t = true)]
        include_from_policy: bool,

        /// Change intent — a short description of what this change is trying to achieve.
        /// Made available to review rules via the review_intent() built-in and
        /// appended to the LLM advisory prompt for intent-coherence checking.
        #[arg(short = 'I', long)]
        intent: Option<String>,
    },
}

#[tokio::main]
async fn main() {
    // Initialize file-based logging (avoids stdio interference with DAP/LSP)
    kleis::logging::init_default_logging();

    let config = std::sync::Arc::new(kleis::config::load());

    let cli = Cli::parse();

    match cli.command {
        Commands::Server { verbose } => {
            run_server(verbose, config.clone()).await;
        }
        Commands::Eval { expression, file } => {
            run_eval(expression, file);
        }
        Commands::Check { file } => {
            run_check(file);
        }
        Commands::Test {
            file,
            example,
            verbose,
            raw_output,
        } => {
            run_test(file, example, verbose, raw_output);
        }
        Commands::Repl { load } => {
            run_repl(load);
        }
        Commands::Dap { verbose } => {
            run_dap(verbose);
        }
        Commands::Mcp { policy, verbose } => {
            run_mcp(policy, verbose);
        }
        Commands::TheoryMcp { verbose } => {
            run_theory_mcp(verbose, &config);
        }
        Commands::ReviewMcp { policy, verbose } => {
            run_review_mcp(policy, verbose);
        }
        Commands::Review {
            files,
            policy,
            failures_only,
            verbose,
            advise,
            base_branch,
            include,
            include_from_policy,
            intent,
        } => {
            run_review(
                files,
                policy,
                failures_only,
                verbose,
                advise,
                base_branch,
                include,
                include_from_policy,
                intent,
            );
        }
    }
}

/// Run the unified server (LSP + DAP)
async fn run_server(verbose: bool, _config: std::sync::Arc<kleis::config::KleisConfig>) {
    if verbose {
        eprintln!("[kleis] Starting unified server (LSP + DAP)");
    }

    // For now, just run the LSP server
    // We'll add DAP spawning via custom LSP requests
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    // Import the LSP types we need
    use tower_lsp::{LspService, Server};

    // Create and run the language server
    // Note: We'll refactor this to use SharedContext once we have that
    let (service, socket) = LspService::new(|client| KleisUnifiedServer::new(client, verbose));

    Server::new(stdin, stdout, socket).serve(service).await;
}

/// Run a one-shot evaluation
fn run_eval(expression: Option<String>, file: Option<PathBuf>) {
    use kleis::evaluator::Evaluator;
    use kleis::kleis_parser::{parse_kleis, parse_kleis_program};

    let mut evaluator = Evaluator::new();
    let has_file = file.is_some();

    // If a file is provided, load it first
    if let Some(file_path) = file {
        match std::fs::read_to_string(&file_path) {
            Ok(source) => match parse_kleis_program(&source) {
                Ok(program) => {
                    if let Err(e) = evaluator.load_program(&program) {
                        eprintln!("Error loading {}: {}", file_path.display(), e);
                        std::process::exit(1);
                    }
                }
                Err(e) => {
                    eprintln!("Parse error in {}: {}", file_path.display(), e);
                    std::process::exit(1);
                }
            },
            Err(e) => {
                eprintln!("Cannot read {}: {}", file_path.display(), e);
                std::process::exit(1);
            }
        }
    }

    // Evaluate expression if provided
    if let Some(expr_str) = expression {
        match parse_kleis(&expr_str) {
            Ok(expr) => {
                // Use eval_concrete for numerical computation (eigenvalues, svd, etc.)
                match evaluator.eval_concrete(&expr) {
                    Ok(result) => {
                        // Pretty-print the result
                        println!("{}", format_result(&result));
                    }
                    Err(e) => {
                        eprintln!("Evaluation error: {}", e);
                        std::process::exit(1);
                    }
                }
            }
            Err(e) => {
                eprintln!("Parse error: {}", e);
                std::process::exit(1);
            }
        }
    } else if !has_file {
        eprintln!("Error: Provide an expression or --file");
        std::process::exit(1);
    }
}

/// Format an expression result for display
fn format_result(expr: &kleis::ast::Expression) -> String {
    use kleis::ast::Expression;

    match expr {
        Expression::Const(s) => s.clone(),
        Expression::String(s) => format!("\"{}\"", s),
        Expression::Object(s) => s.clone(),
        Expression::List(items) => {
            let items_str: Vec<String> = items.iter().map(format_result).collect();
            format!("[{}]", items_str.join(", "))
        }
        Expression::Operation { name, args, .. } => {
            // Handle complex numbers: complex(re, im)
            if name == "complex" && args.len() == 2 {
                let re = format_result(&args[0]);
                let im = format_result(&args[1]);
                // Parse imaginary part to handle sign
                if let Some(stripped) = im.strip_prefix('-') {
                    format!("{} - {}i", re, stripped)
                } else if im == "0" || im == "0.0" {
                    re // Just return real part if imag is zero
                } else {
                    format!("{} + {}i", re, im)
                }
            } else if args.is_empty() {
                name.clone()
            } else {
                let args_str: Vec<String> = args.iter().map(format_result).collect();
                format!("{}({})", name, args_str.join(", "))
            }
        }
        _ => format!("{:?}", expr), // Fallback for other cases
    }
}

/// Check a file for errors
fn run_check(file: PathBuf) {
    use kleis::evaluator::Evaluator;
    use kleis::kleis_parser::parse_kleis_program;

    match std::fs::read_to_string(&file) {
        Ok(source) => match parse_kleis_program(&source) {
            Ok(program) => {
                // Try to load into evaluator (validates definitions)
                let mut evaluator = Evaluator::new();
                if let Err(e) = evaluator.load_program(&program) {
                    eprintln!("{}: error: {}", file.display(), e);
                    std::process::exit(1);
                }
                let (funcs, data, structs, _) = evaluator.definition_counts();
                println!(
                    "{}: OK ({} functions, {} data types, {} structures)",
                    file.display(),
                    funcs,
                    data,
                    structs
                );
            }
            Err(e) => {
                eprintln!(
                    "{}: parse error:\n   {}",
                    file.display(),
                    e.format_with_source(&source)
                );
                std::process::exit(1);
            }
        },
        Err(e) => {
            eprintln!("{}: {}", file.display(), e);
            std::process::exit(1);
        }
    }
}

/// Run example blocks as tests (v0.93)
fn run_test(file: PathBuf, example_filter: Option<String>, verbose: bool, raw_output: bool) {
    use kleis::evaluator::Evaluator;
    use kleis::kleis_ast::TopLevel;
    use kleis::kleis_parser::parse_kleis_program_with_file;
    use std::collections::HashSet;

    // Canonicalize the file path
    let canonical = file.canonicalize().unwrap_or_else(|_| file.clone());
    let file_path_str = canonical.to_string_lossy().to_string();

    // Read the file
    let source = match std::fs::read_to_string(&canonical) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{}: {}", file.display(), e);
            std::process::exit(1);
        }
    };

    // Parse the program with file path
    let program = match parse_kleis_program_with_file(&source, &file_path_str) {
        Ok(p) => p,
        Err(e) => {
            eprintln!(
                "{}: parse error:\n   {}",
                file.display(),
                e.format_with_source(&source)
            );
            std::process::exit(1);
        }
    };

    // Load the program AND its imports into evaluator
    let mut evaluator = Evaluator::new();
    let mut loaded_files: HashSet<PathBuf> = HashSet::new();

    // Load imports first (recursively)
    if let Err(e) = load_imports_recursive(&program, &canonical, &mut evaluator, &mut loaded_files)
    {
        eprintln!("{}: import error: {}", file.display(), e);
        std::process::exit(1);
    }

    // Then load the main file
    if let Err(e) = evaluator.load_program_with_file(&program, Some(canonical.clone())) {
        eprintln!("{}: error: {}", file.display(), e);
        std::process::exit(1);
    }

    // Count example blocks
    let examples: Vec<_> = program
        .items
        .iter()
        .filter_map(|item| {
            if let TopLevel::ExampleBlock(ex) = item {
                Some(ex)
            } else {
                None
            }
        })
        .collect();

    if examples.is_empty() {
        println!("{}: no example blocks found", file.display());
        return;
    }

    // Run examples
    let mut passed = 0;
    let mut failed = 0;
    let mut skipped = 0;

    for example in &examples {
        // Apply filter if specified
        if let Some(ref filter) = example_filter
            && !example.name.contains(filter.as_str())
        {
            skipped += 1;
            if verbose && !raw_output {
                println!("⏭️  {} (skipped)", example.name);
            }
            continue;
        }

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            evaluator.eval_example_block(example)
        }));
        let result = match result {
            Ok(r) => r,
            Err(panic_info) => {
                let msg = if let Some(s) = panic_info.downcast_ref::<String>() {
                    s.clone()
                } else if let Some(s) = panic_info.downcast_ref::<&str>() {
                    s.to_string()
                } else {
                    "unknown panic".to_string()
                };
                kleis::evaluator::ExampleResult {
                    name: example.name.clone(),
                    passed: false,
                    assertions_passed: 0,
                    assertions_total: 0,
                    error: Some(format!("Z3 panic (likely out of memory): {}", msg)),
                    witness: None,
                }
            }
        };

        if result.passed {
            passed += 1;
            if !raw_output {
                if verbose {
                    println!(
                        "✅ {}: passed ({}/{} assertions)",
                        result.name, result.assertions_passed, result.assertions_total
                    );
                } else {
                    println!("✅ {}", result.name);
                }
            }
        } else {
            failed += 1;
            if !raw_output {
                println!("❌ {}", result.name);
                if let Some(error) = &result.error {
                    println!("   {}", error);
                }
            }
        }
    }

    // Print summary
    if !raw_output {
        println!();
        let total = passed + failed;
        if failed == 0 {
            println!(
                "✅ {} example{} passed",
                total,
                if total == 1 { "" } else { "s" }
            );
            if skipped > 0 {
                println!("   ({} skipped by filter)", skipped);
            }
        } else {
            println!(
                "❌ {}/{} examples passed ({} failed)",
                passed, total, failed
            );
            std::process::exit(1);
        }
    } else if failed > 0 {
        // In raw mode, still propagate failure via exit code
        std::process::exit(1);
    }
}

/// Recursively load imports for a program
fn load_imports_recursive(
    program: &kleis::kleis_ast::Program,
    file_path: &Path,
    evaluator: &mut kleis::evaluator::Evaluator,
    loaded_files: &mut std::collections::HashSet<PathBuf>,
) -> std::result::Result<(), String> {
    use kleis::kleis_ast::TopLevel;
    use kleis::kleis_parser::parse_kleis_program_with_file;

    // Get base directory for resolving imports
    let base_dir = file_path.parent().unwrap_or(Path::new("."));

    // Process imports
    for item in &program.items {
        if let TopLevel::Import(import_path_str) = item {
            // Resolve the import path
            let import_path = Path::new(import_path_str);
            let resolved = if import_path.is_absolute() {
                import_path.to_path_buf()
            } else if import_path_str.starts_with("stdlib/") {
                // stdlib imports: try relative to project root
                PathBuf::from(import_path_str)
            } else {
                // Relative import: resolve from importing file's directory
                base_dir.join(import_path)
            };

            // Canonicalize
            let canonical = resolved
                .canonicalize()
                .map_err(|e| format!("Cannot resolve import '{}': {}", import_path_str, e))?;

            // Skip if already loaded
            if loaded_files.contains(&canonical) {
                continue;
            }
            loaded_files.insert(canonical.clone());

            // Read and parse the import
            let source = std::fs::read_to_string(&canonical)
                .map_err(|e| format!("Cannot read import '{}': {}", import_path_str, e))?;
            let file_path_str = canonical.to_string_lossy().to_string();
            let import_program = parse_kleis_program_with_file(&source, &file_path_str)
                .map_err(|e| format!("Parse error in '{}': {}", import_path_str, e))?;

            // Recursively load this import's imports first
            load_imports_recursive(&import_program, &canonical, evaluator, loaded_files)?;

            // Then load the import itself
            evaluator.load_program_with_file(&import_program, Some(canonical.clone()))?;
        }
    }

    Ok(())
}

/// Run the interactive REPL
fn run_repl(load: Vec<PathBuf>) {
    // For now, just print a message - the full REPL is in repl.rs
    // We'll integrate it properly later
    eprintln!("Starting Kleis REPL...");
    if !load.is_empty() {
        eprintln!("Loading: {:?}", load);
    }
    eprintln!("TODO: Integrate with existing REPL implementation");
    eprintln!("For now, use: cargo run --bin repl");
}

/// Run standalone DAP server over stdio
/// This is used when the LSP server is not available
fn run_dap(verbose: bool) {
    use kleis::dap::run_stdio_server;

    if verbose {
        eprintln!("[kleis-dap] Starting standalone DAP server over stdio");
    }

    if let Err(e) = run_stdio_server() {
        eprintln!("[kleis-dap] Error: {}", e);
        std::process::exit(1);
    }
}

/// Run MCP (Model Context Protocol) server for formal policy enforcement
fn run_mcp(policy: PathBuf, verbose: bool) {
    use kleis::mcp::server::McpServer;

    if verbose {
        eprintln!("[kleis-mcp] Starting MCP Policy Server");
        eprintln!("[kleis-mcp] Policy file: {}", policy.display());
    }

    let server = match McpServer::new(&policy, verbose) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[kleis-mcp] Failed to load policy: {}", e);
            std::process::exit(1);
        }
    };

    if let Err(e) = server.run() {
        eprintln!("[kleis-mcp] Server error: {}", e);
        std::process::exit(1);
    }
}

/// Run Theory MCP server for interactive theory building (ADR-031)
fn run_theory_mcp(verbose: bool, config: &kleis::config::KleisConfig) {
    use kleis::theory_mcp::server::TheoryMcpServer;

    if verbose {
        eprintln!("[kleis-theory] Starting Theory MCP Server");
        eprintln!(
            "[kleis-theory] Workspace: {}, Save dir: {}",
            config.theory.workspace_dir, config.theory.save_dir
        );
    }

    let mut server = match TheoryMcpServer::new(&config.theory, verbose) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[kleis-theory] Failed to start: {}", e);
            std::process::exit(1);
        }
    };

    if let Err(e) = server.run() {
        eprintln!("[kleis-theory] Server error: {}", e);
        std::process::exit(1);
    }
}

/// Run Review MCP server for code review via formal coding standards
fn run_review_mcp(policy: PathBuf, verbose: bool) {
    use kleis::review_mcp::server::ReviewMcpServer;

    if verbose {
        eprintln!("[kleis-review] Starting Review MCP Server");
        eprintln!("[kleis-review] Policy file: {}", policy.display());
    }

    let server = match ReviewMcpServer::new(&policy, verbose) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[kleis-review] Failed to load policy: {}", e);
            std::process::exit(1);
        }
    };

    if let Err(e) = server.run() {
        eprintln!("[kleis-review] Server error: {}", e);
        std::process::exit(1);
    }
}

/// Get the git repository root directory
fn git_repo_root_for(dir: &Path) -> Option<PathBuf> {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .current_dir(dir)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(PathBuf::from(
        String::from_utf8_lossy(&output.stdout).trim(),
    ))
}

fn git_ref_exists_in(git_ref: &str, repo_root: &Path) -> bool {
    std::process::Command::new("git")
        .args(["rev-parse", "--verify", git_ref])
        .current_dir(repo_root)
        .output()
        .is_ok_and(|o| o.status.success())
}

fn git_show_file(git_ref: &str, repo_relative_path: &str, repo_root: &Path) -> Option<String> {
    let ref_path = format!("{}:{}", git_ref, repo_relative_path);
    let output = std::process::Command::new("git")
        .args(["show", &ref_path])
        .current_dir(repo_root)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Compute a file path relative to the git repo root
fn repo_relative_path(file_path: &Path, repo_root: &Path) -> Option<String> {
    let abs = if file_path.is_absolute() {
        file_path.to_path_buf()
    } else {
        std::env::current_dir().ok()?.join(file_path)
    };
    let canonical = abs.canonicalize().ok()?;
    let root_canonical = repo_root.canonicalize().ok()?;
    canonical
        .strip_prefix(&root_canonical)
        .ok()
        .map(|p| p.to_string_lossy().to_string())
}

fn language_from_path(path: &Path) -> String {
    match path.extension().and_then(|e| e.to_str()) {
        Some("rs") => "Rust".to_string(),
        Some("py") => "Python".to_string(),
        Some("go") => "Go".to_string(),
        Some("ts") | Some("tsx") => "TypeScript".to_string(),
        Some("js") | Some("jsx") => "JavaScript".to_string(),
        Some("java") => "Java".to_string(),
        Some("kleis") => "Kleis".to_string(),
        Some(ext) => ext.to_string(),
        None => "unknown".to_string(),
    }
}

/// Run code review from the command line (CI/CD mode)
#[allow(clippy::too_many_arguments)]
fn run_review(
    mut files: Vec<PathBuf>,
    policy: PathBuf,
    failures_only: bool,
    verbose: bool,
    advise: bool,
    base_branch: Option<String>,
    include: Vec<PathBuf>,
    include_from_policy: bool,
    intent: Option<String>,
) {
    use kleis::review_mcp::advisory::{self, AdvisoryConfig};
    use kleis::review_mcp::engine::{self, ReviewEngine};

    if files.is_empty() {
        eprintln!("No files specified. Usage: kleis review [FILES...] --policy <policy.kleis>");
        std::process::exit(1);
    }

    if let Some(ref intent_str) = intent {
        engine::set_review_intent(intent_str);
        if verbose {
            eprintln!("[kleis-review] Intent: {}", intent_str);
        }
    }

    let kleis_cfg = kleis::config::load();
    let mut llm_config = if advise {
        match AdvisoryConfig::from_config(&kleis_cfg.llm) {
            Some(cfg) => {
                if verbose {
                    eprintln!(
                        "[kleis-review] Advisory mode: {} via {}",
                        cfg.model, cfg.endpoint
                    );
                }
                Some(cfg)
            }
            None => {
                eprintln!("--advise requires KLEIS_LLM_API_KEY environment variable");
                std::process::exit(1);
            }
        }
    } else {
        None
    };

    // Derive git repo root from the first target file, not cwd.
    // This allows reviewing files in a different repo than the one containing
    let diff_context: Option<(String, PathBuf)> = base_branch.and_then(|ref_name| {
        let first_file = files.first()?;
        let file_dir = if first_file.is_absolute() {
            first_file.parent().unwrap_or(Path::new(".")).to_path_buf()
        } else {
            std::env::current_dir()
                .ok()?
                .join(first_file)
                .parent()
                .unwrap_or(Path::new("."))
                .to_path_buf()
        };
        let repo_root = match git_repo_root_for(&file_dir) {
            Some(r) => r,
            None => {
                eprintln!("[kleis-review] warning: target files not in a git repository, skipping diff rules");
                return None;
            }
        };
        if !git_ref_exists_in(&ref_name, &repo_root) {
            eprintln!(
                "[kleis-review] warning: base branch '{}' not found in {}, skipping diff rules",
                ref_name,
                repo_root.display()
            );
            return None;
        }
        if verbose {
            eprintln!("[kleis-review] Base branch: {} (verified)", ref_name);
            eprintln!("[kleis-review] Repo root: {}", repo_root.display());
        }
        Some((ref_name, repo_root))
    });

    if verbose {
        eprintln!("[kleis-review] Policy: {}", policy.display());
    }

    let engine = match ReviewEngine::load(&policy) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("Failed to load review policy '{}': {}", policy.display(), e);
            std::process::exit(1);
        }
    };

    // Append --include files
    if !include.is_empty() {
        if verbose {
            eprintln!("[kleis-review] --include: {} extra file(s)", include.len());
        }
        files.extend(include);
    }

    // Auto-discover extra files from the policy's review_extra_files()
    if include_from_policy && let Some(extra) = engine.extra_review_files() {
        let repo_root = diff_context
            .as_ref()
            .map(|(_, root)| root.clone())
            .or_else(|| {
                files.first().and_then(|f| {
                    let dir = if f.is_absolute() {
                        f.parent().unwrap_or(Path::new(".")).to_path_buf()
                    } else {
                        std::env::current_dir()
                            .ok()?
                            .join(f)
                            .parent()
                            .unwrap_or(Path::new("."))
                            .to_path_buf()
                    };
                    git_repo_root_for(&dir)
                })
            });

        if let Some(root) = repo_root {
            for pattern in &extra {
                let candidate = root.join(pattern);
                if candidate.exists() {
                    if verbose {
                        eprintln!(
                            "[kleis-review] auto-include from policy: {}",
                            candidate.display()
                        );
                    }
                    files.push(candidate);
                } else if verbose {
                    eprintln!(
                        "[kleis-review] auto-include: {} not found (skipped)",
                        candidate.display()
                    );
                }
            }
        }
    }

    if verbose {
        eprintln!("[kleis-review] Files: {}", files.len());
    }

    let formal_rule_names: Vec<String> =
        engine.list_rules().iter().map(|r| r.name.clone()).collect();

    let has_diff_checks = engine.has_diff_checks();
    let has_diff_filter = engine.has_diff_file_filter();

    if verbose && diff_context.is_some() {
        eprintln!(
            "[kleis-review] diff_check_* functions: {}",
            if has_diff_checks { "found" } else { "none" }
        );
        eprintln!(
            "[kleis-review] diff_file_filter: {}",
            if has_diff_filter {
                "defined"
            } else {
                "not defined (diff checks run for all files)"
            }
        );
    }

    // Load per-language LLM guidelines once at startup.
    // Uses first target file's language for initial resolution; reloads if language changes.
    if let Some(ref mut cfg) = llm_config
        && let Some(first) = files.first()
    {
        let lang = language_from_path(first);
        cfg.load_guidelines(&lang, &kleis_cfg.llm.guidelines_file);
        if verbose {
            if cfg.guidelines.is_some() {
                eprintln!("[kleis-review] Loaded {} guidelines for LLM prompt", lang);
            } else {
                eprintln!(
                    "[kleis-review] No guidelines file found for {}, using generic prompt",
                    lang
                );
            }
        }
    }

    let rt = tokio::runtime::Handle::current();

    let mut total_passed = 0usize;
    let mut total_failed = 0usize;
    let mut failed_files: Vec<String> = Vec::new();

    for file_path in &files {
        let path_str = file_path.to_string_lossy();
        engine::set_review_path(&path_str);

        let language = language_from_path(file_path);

        // --- Standard check_* rules ---
        let result = match engine.check_file(&path_str, &language) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("  ERROR: {} — {}", path_str, e);
                total_failed += 1;
                failed_files.push(path_str.to_string());
                continue;
            }
        };

        let mut file_passed = result.passed;
        let has_advisories = result
            .verdicts
            .iter()
            .any(|v| !v.passed && v.severity == kleis::review_mcp::engine::RuleSeverity::Advisory);

        let emoji = if !result.passed {
            "❌"
        } else if has_advisories {
            "⚠️"
        } else {
            "✅"
        };
        println!("{} {}", emoji, path_str);

        for verdict in &result.verdicts {
            if failures_only && verdict.passed {
                continue;
            }
            let v_emoji = if verdict.passed {
                "  ✅"
            } else if verdict.severity == kleis::review_mcp::engine::RuleSeverity::Advisory {
                "  ⚠️"
            } else {
                "  ❌"
            };
            println!("{} {} — {}", v_emoji, verdict.rule_name, verdict.message);
        }

        // --- Diff-aware diff_check_* rules ---
        if let Some((ref base_ref, ref repo_root)) = diff_context
            && has_diff_checks
        {
            let should_diff = if has_diff_filter {
                engine.eval_diff_file_filter(&path_str)
            } else {
                true
            };

            if should_diff {
                if let Some(rel_path) = repo_relative_path(file_path, repo_root) {
                    match git_show_file(base_ref, &rel_path, repo_root) {
                        Some(base_content) => {
                            let current_content =
                                std::fs::read_to_string(file_path).unwrap_or_default();
                            if verbose {
                                eprintln!(
                                    "[kleis-review] diff_file_filter(\"{}\") = true, fetched {} bytes from {}",
                                    path_str,
                                    base_content.len(),
                                    base_ref
                                );
                            }
                            let diff_result =
                                engine.check_diff(&current_content, &base_content, &path_str);
                            for verdict in &diff_result.verdicts {
                                if !verdict.passed
                                    && verdict.severity
                                        == kleis::review_mcp::engine::RuleSeverity::Error
                                {
                                    file_passed = false;
                                }
                                if failures_only && verdict.passed {
                                    continue;
                                }
                                let v_emoji = if verdict.passed {
                                    "  ✅"
                                } else if verdict.severity
                                    == kleis::review_mcp::engine::RuleSeverity::Advisory
                                {
                                    "  ⚠️"
                                } else {
                                    "  ❌"
                                };
                                println!("{} {} — {}", v_emoji, verdict.rule_name, verdict.message);
                            }
                        }
                        None => {
                            if verbose {
                                eprintln!(
                                    "[kleis-review] {} not found on {}, skipping diff rules",
                                    rel_path, base_ref
                                );
                            }
                        }
                    }
                }
            } else if verbose {
                eprintln!(
                    "[kleis-review] diff_file_filter(\"{}\") = false, skipping diff rules",
                    path_str
                );
            }
        }

        if file_passed {
            total_passed += 1;
        } else {
            total_failed += 1;
            failed_files.push(path_str.to_string());
        }

        // --- LLM advisory ---
        if let Some(ref cfg) = llm_config {
            let source = match std::fs::read_to_string(file_path) {
                Ok(s) => s,
                Err(_) => continue,
            };
            let formal_messages: Vec<String> = result
                .verdicts
                .iter()
                .filter(|v| !v.passed)
                .map(|v| format!("{} — {}", v.rule_name, v.message))
                .collect();
            match tokio::task::block_in_place(|| {
                rt.block_on(advisory::get_advisories(
                    cfg,
                    &source,
                    &path_str,
                    &language,
                    &formal_messages,
                    &formal_rule_names,
                    intent.as_deref(),
                ))
            }) {
                Ok(advisories) if advisories.is_empty() => {
                    println!("  [advisory] no findings");
                }
                Ok(advisories) => {
                    for a in &advisories {
                        let icon = if a.severity == "warning" {
                            "⚠️ "
                        } else {
                            "ℹ️ "
                        };
                        let loc = a.line.map(|l| format!(" (line {l})")).unwrap_or_default();
                        println!("  {} [advisory] {} — {}{}", icon, a.check, a.reason, loc);
                        if let Some(ref ev) = a.evidence {
                            println!("     | {}", ev.trim());
                        }
                    }
                }
                Err(e) => {
                    eprintln!("  [advisory] LLM error: {}", e);
                }
            }
        }
    }

    println!();
    println!(
        "Review complete: {} passed, {} failed (out of {} files)",
        total_passed,
        total_failed,
        files.len()
    );

    if !failed_files.is_empty() {
        println!();
        println!("Failed files:");
        for f in &failed_files {
            println!("  - {}", f);
        }
        std::process::exit(1);
    }
}

// ============================================================================
// Unified Language Server
// ============================================================================

use dashmap::DashMap;
use kleis::context::Document as CachedDocument;
use kleis::evaluator::Evaluator;
use kleis::kleis_ast::Program;
use kleis::structure_registry::StructureRegistry;
use kleis::type_checker::TypeChecker;
use ropey::Rope;
use std::collections::HashMap;
use std::net::TcpListener;
use std::sync::{Arc, Mutex, RwLock};
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

/// Thread-safe AST cache shared between LSP and DAP
/// This contains only the parsed ASTs and source code, NOT the evaluator
type AstCache = Arc<RwLock<HashMap<PathBuf, CachedDocument>>>;

/// Shared context between LSP, DAP, and REPL
#[allow(dead_code)]
/// Server-side shared context
///
/// Architecture:
/// - `ast_cache`: Thread-safe AST cache shared between LSP and DAP
/// - `evaluator`: Per-thread evaluator (LSP has its own, each DAP session creates its own)
/// - `structure_registry`: Shared structure registry
struct SharedContext {
    /// Thread-safe AST cache (shared between LSP and DAP threads)
    ast_cache: AstCache,
    /// LSP's evaluator (not shared with DAP - each DAP session creates its own)
    evaluator: Arc<Mutex<Evaluator>>,
    /// Type checker
    type_checker: Arc<Mutex<TypeChecker>>,
    /// Structure registry
    structure_registry: Arc<Mutex<StructureRegistry>>,
}

impl SharedContext {
    fn new() -> Self {
        Self {
            ast_cache: Arc::new(RwLock::new(HashMap::new())),
            evaluator: Arc::new(Mutex::new(Evaluator::new())),
            type_checker: Arc::new(Mutex::new(TypeChecker::new())),
            structure_registry: Arc::new(Mutex::new(StructureRegistry::new())),
        }
    }
}

/// Document state
#[allow(dead_code)]
struct Document {
    content: Rope,
    ast: Option<Program>,
}

/// The unified Kleis server
struct KleisUnifiedServer {
    client: Client,
    documents: DashMap<Url, Document>,
    shared: SharedContext,
    verbose: bool,
    /// Port where DAP is listening (if started)
    dap_port: Arc<Mutex<Option<u16>>>,
}

impl KleisUnifiedServer {
    fn new(client: Client, verbose: bool) -> Self {
        Self {
            client,
            documents: DashMap::new(),
            shared: SharedContext::new(),
            verbose,
            dap_port: Arc::new(Mutex::new(None)),
        }
    }

    fn log(&self, msg: &str) {
        if self.verbose {
            eprintln!("[kleis] {}", msg);
        }
    }

    /// Start the DAP server on a dynamic port
    fn start_dap_server(&self) -> std::result::Result<u16, String> {
        // Bind to port 0 - OS assigns available port
        let listener = TcpListener::bind("127.0.0.1:0")
            .map_err(|e| format!("Failed to bind DAP server: {}", e))?;

        let port = listener
            .local_addr()
            .map_err(|e| format!("Failed to get port: {}", e))?
            .port();

        self.log(&format!("DAP server starting on port {}", port));

        // Clone AST cache for the DAP thread (thread-safe)
        let ast_cache = self.shared.ast_cache.clone();
        let verbose = self.verbose;

        // Spawn DAP server thread
        std::thread::spawn(move || {
            if let Err(e) = run_dap_on_listener(listener, ast_cache, verbose) {
                eprintln!("[kleis-dap] Error: {}", e);
            }
        });

        // Store the port
        *self.dap_port.lock().unwrap() = Some(port);

        Ok(port)
    }

    /// Parse and validate a document
    ///
    /// Updates the thread-safe AST cache and loads into the LSP evaluator.
    fn parse_document(&self, uri: &Url, content: &str) -> (Option<Program>, Vec<Diagnostic>) {
        use kleis::kleis_parser::parse_kleis_program_with_file;

        let mut diagnostics = Vec::new();

        // Convert URI to canonical path
        let path = uri
            .to_file_path()
            .unwrap_or_else(|_| PathBuf::from(uri.path()));
        let canonical = path.canonicalize().unwrap_or_else(|_| path.clone());

        // Parse the document
        let file_path_str = canonical.to_string_lossy().to_string();
        let parse_result = parse_kleis_program_with_file(content, &file_path_str);

        let program = match parse_result {
            Ok(p) => Some(p),
            Err(e) => {
                let line = content[..e.position.min(content.len())]
                    .chars()
                    .filter(|c| *c == '\n')
                    .count() as u32;

                diagnostics.push(Diagnostic {
                    range: Range {
                        start: Position { line, character: 0 },
                        end: Position {
                            line,
                            character: 100,
                        },
                    },
                    severity: Some(DiagnosticSeverity::ERROR),
                    message: e.message.clone(),
                    ..Default::default()
                });
                None
            }
        };

        // Update the thread-safe AST cache
        if let Ok(mut cache) = self.shared.ast_cache.write() {
            // Extract imports from program
            let imports: std::collections::HashSet<PathBuf> = program
                .as_ref()
                .map(|p| {
                    p.items
                        .iter()
                        .filter_map(|item| {
                            if let kleis::kleis_ast::TopLevel::Import(import_path) = item {
                                // Resolve import path relative to current file
                                resolve_import_path(import_path, &canonical)
                            } else {
                                None
                            }
                        })
                        .collect()
                })
                .unwrap_or_default();

            // Mark dependents as dirty (cascade invalidation)
            invalidate_dependents(&mut cache, &canonical);

            // Store in cache
            let cached_doc = CachedDocument {
                source: content.to_string(),
                program: program.clone(),
                diagnostics: vec![], // LSP handles diagnostics separately
                imports,
                dirty: false,
            };
            cache.insert(canonical.clone(), cached_doc);
        }

        // Load into LSP's evaluator
        if let Some(ref prog) = program
            && let Ok(mut eval) = self.shared.evaluator.lock()
            && let Err(e) = eval.load_program(prog)
        {
            diagnostics.push(Diagnostic {
                range: Range::default(),
                severity: Some(DiagnosticSeverity::ERROR),
                message: format!("Load error: {}", e),
                ..Default::default()
            });
        }

        (program, diagnostics)
    }
}

/// Resolve an import path relative to the importing file
///
/// For stdlib imports, checks in order:
/// 1. KLEIS_ROOT environment variable
/// 2. Walk up from importing file to find stdlib/
fn resolve_import_path(import_path: &str, from_file: &Path) -> Option<PathBuf> {
    // Handle stdlib imports
    if import_path.starts_with("stdlib/") {
        // First, check KLEIS_ROOT environment variable
        if let Ok(kleis_root) = std::env::var("KLEIS_ROOT") {
            let candidate = PathBuf::from(&kleis_root).join(import_path);
            if candidate.exists() {
                return candidate.canonicalize().ok();
            }
        }

        // Try relative to project root (walk up from current file)
        if let Some(parent) = from_file.parent() {
            let mut dir = parent.to_path_buf();
            for _ in 0..10 {
                let candidate = dir.join(import_path);
                if candidate.exists() {
                    return candidate.canonicalize().ok();
                }
                if let Some(p) = dir.parent() {
                    dir = p.to_path_buf();
                } else {
                    break;
                }
            }
        }
        return None;
    }

    // Handle relative paths
    if let Some(parent) = from_file.parent() {
        let resolved = parent.join(import_path);
        if resolved.exists() {
            return resolved.canonicalize().ok();
        }
        // Try with .kleis extension
        let with_ext = parent.join(format!("{}.kleis", import_path));
        if with_ext.exists() {
            return with_ext.canonicalize().ok();
        }
    }

    None
}

/// Mark a document and all its dependents as dirty in the cache
fn invalidate_dependents(cache: &mut HashMap<PathBuf, CachedDocument>, path: &PathBuf) {
    // Mark the changed document as dirty
    if let Some(doc) = cache.get_mut(path) {
        doc.dirty = true;
    }

    // Keep iterating until no new documents are marked dirty
    loop {
        let mut newly_dirtied = false;

        // Collect paths of dirty documents
        let dirty_paths: std::collections::HashSet<PathBuf> = cache
            .iter()
            .filter(|(_, doc)| doc.dirty)
            .map(|(p, _)| p.clone())
            .collect();

        // For each non-dirty document, check if it imports any dirty document
        for (doc_path, doc) in cache.iter_mut() {
            if doc.dirty {
                continue;
            }

            // If this document imports any dirty file, mark it dirty
            if doc.imports.iter().any(|imp| dirty_paths.contains(imp)) {
                doc.dirty = true;
                newly_dirtied = true;
            }

            // Also mark dirty if the document path itself is the changed one
            if doc_path == path && !doc.dirty {
                doc.dirty = true;
                newly_dirtied = true;
            }
        }

        if !newly_dirtied {
            break;
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for KleisUnifiedServer {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        self.log("Initializing Kleis unified server");

        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec![".".to_string(), ":".to_string()]),
                    ..Default::default()
                }),
                execute_command_provider: Some(ExecuteCommandOptions {
                    commands: vec!["kleis.startDebugSession".to_string()],
                    ..Default::default()
                }),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.log("Kleis unified server initialized");
    }

    async fn shutdown(&self) -> Result<()> {
        self.log("Shutting down");
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let content = params.text_document.text;

        let (ast, diagnostics) = self.parse_document(&uri, &content);

        self.documents.insert(
            uri.clone(),
            Document {
                content: Rope::from_str(&content),
                ast,
            },
        );

        self.client
            .publish_diagnostics(uri, diagnostics, None)
            .await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;

        if let Some(change) = params.content_changes.into_iter().next() {
            let content = change.text;
            let (ast, diagnostics) = self.parse_document(&uri, &content);

            self.documents.insert(
                uri.clone(),
                Document {
                    content: Rope::from_str(&content),
                    ast,
                },
            );

            self.client
                .publish_diagnostics(uri, diagnostics, None)
                .await;
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        self.documents.remove(&params.text_document.uri);
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        // Basic hover - show "Kleis" for now
        // TODO: Integrate with type checker for real hover info
        let _uri = params.text_document_position_params.text_document.uri;
        let _position = params.text_document_position_params.position;

        Ok(Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: "**Kleis** - Unified Server".to_string(),
            }),
            range: None,
        }))
    }

    async fn completion(&self, _: CompletionParams) -> Result<Option<CompletionResponse>> {
        // Basic completions
        let items = vec![
            CompletionItem {
                label: "define".to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                detail: Some("Define a function".to_string()),
                ..Default::default()
            },
            CompletionItem {
                label: "let".to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                detail: Some("Local binding".to_string()),
                ..Default::default()
            },
        ];

        Ok(Some(CompletionResponse::Array(items)))
    }

    async fn execute_command(
        &self,
        params: ExecuteCommandParams,
    ) -> Result<Option<serde_json::Value>> {
        match params.command.as_str() {
            "kleis.startDebugSession" => {
                self.log("Received kleis.startDebugSession command");

                // Extract program path from arguments
                let program_path = params
                    .arguments
                    .first()
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                // Start DAP server on dynamic port
                match self.start_dap_server() {
                    Ok(port) => {
                        self.log(&format!("DAP server started on port {}", port));
                        Ok(Some(serde_json::json!({
                            "port": port,
                            "program": program_path
                        })))
                    }
                    Err(e) => {
                        self.log(&format!("Failed to start DAP: {}", e));
                        Ok(Some(serde_json::json!({
                            "error": e
                        })))
                    }
                }
            }
            other => {
                self.log(&format!("Unknown command: {}", other));
                Ok(None)
            }
        }
    }
}

/// Run the DAP server on an existing listener
/// DAP session state - uses real evaluator with DebugHook for stepping
struct DapState {
    /// Thread-safe AST cache (shared with LSP)
    ast_cache: AstCache,
    /// DAP's own evaluator (not shared with LSP)
    evaluator: Evaluator,
    /// Current file being debugged (updated from StopEvent)
    current_file: Option<PathBuf>,
    /// Current line (updated from StopEvent)
    current_line: u32,

    // === Channel-based debugging (real evaluator integration) ===
    /// Controller for channel-based communication with DebugHook
    /// Also holds shared breakpoints (can be updated mid-session from this thread)
    controller: Option<kleis::debug::DapDebugController>,
    /// Handle to evaluation thread
    eval_thread: Option<std::thread::JoinHandle<()>>,
    /// Parsed program (for finding example blocks to debug)
    program: Option<kleis::kleis_ast::Program>,
    /// Last stop event received from the evaluator
    last_stop_event: Option<kleis::debug::StopEvent>,
    /// Debug hook (created in launch, moved to eval thread in configurationDone)
    pending_hook: Option<kleis::debug::DapDebugHook>,
    /// Files already loaded (to prevent import cycles)
    loaded_files: std::collections::HashSet<PathBuf>,
    /// Loaded imports (program + file path) - needed for eval thread
    loaded_imports: Vec<(kleis::kleis_ast::Program, PathBuf)>,
}

impl DapState {
    fn new(ast_cache: AstCache) -> Self {
        Self {
            ast_cache,
            evaluator: Evaluator::new(),
            current_file: None,
            current_line: 1,
            controller: None,
            eval_thread: None,
            program: None,
            last_stop_event: None,
            pending_hook: None,
            loaded_files: std::collections::HashSet::new(),
            loaded_imports: Vec::new(),
        }
    }
}

impl DapState {
    /// Load file using the shared AST cache
    ///
    /// Gets the AST from the cache (which may already have it from LSP)
    /// and loads it into the evaluator.
    fn load_file(&mut self, path: &str) -> std::result::Result<(), String> {
        use kleis::kleis_parser::parse_kleis_program_with_file;

        // Canonicalize path for VS Code (needs absolute paths in stack traces)
        let path_buf = PathBuf::from(path);
        let canonical = path_buf.canonicalize().unwrap_or_else(|_| path_buf.clone());

        self.current_file = Some(canonical.clone());
        self.current_line = 1;

        // Try to get AST from cache, or parse if needed
        let program = {
            let cache = self
                .ast_cache
                .read()
                .map_err(|e| format!("Failed to lock cache: {}", e))?;

            if let Some(doc) = cache.get(&canonical) {
                if !doc.dirty && doc.program.is_some() {
                    // Cache hit: use cached AST
                    doc.program.clone()
                } else {
                    None
                }
            } else {
                None
            }
        };

        let program = if let Some(p) = program {
            p
        } else {
            // Not in cache or dirty: read from disk and parse
            let source = std::fs::read_to_string(&canonical)
                .map_err(|e| format!("Cannot read file: {}", e))?;
            let file_path_str = canonical.to_string_lossy().to_string();
            let parsed = parse_kleis_program_with_file(&source, &file_path_str)
                .map_err(|e| format!("Parse error: {}", e.message))?;

            // Update cache with parsed AST
            if let Ok(mut cache) = self.ast_cache.write() {
                let imports: std::collections::HashSet<PathBuf> = parsed
                    .items
                    .iter()
                    .filter_map(|item| {
                        if let kleis::kleis_ast::TopLevel::Import(import_path) = item {
                            resolve_import_path(import_path, &canonical)
                        } else {
                            None
                        }
                    })
                    .collect();

                let cached_doc = CachedDocument {
                    source,
                    program: Some(parsed.clone()),
                    diagnostics: vec![],
                    imports,
                    dirty: false,
                };
                cache.insert(canonical.clone(), cached_doc);
            }

            parsed
        };

        // Store program for later use (finding example blocks)
        self.program = Some(program.clone());

        // Load program into DAP's evaluator with file path for cross-file debugging
        self.evaluator
            .load_program_with_file(&program, self.current_file.clone())?;

        // Recursively load imported files
        for item in &program.items {
            if let kleis::kleis_ast::TopLevel::Import(import_path) = item
                && let Some(resolved) = resolve_import_path(import_path, &canonical)
            {
                eprintln!("[kleis-dap] Loading import: {}", resolved.display());
                self.load_import(&resolved)?;
            }
        }

        Ok(())
    }

    /// Load an imported file into the evaluator
    fn load_import(&mut self, path: &PathBuf) -> std::result::Result<(), String> {
        use kleis::kleis_parser::parse_kleis_program_with_file;

        // Check if already loaded (avoid cycles)
        if self.loaded_files.contains(path) {
            return Ok(());
        }
        self.loaded_files.insert(path.clone());

        // Parse the imported file
        let source = std::fs::read_to_string(path)
            .map_err(|e| format!("Cannot read import '{}': {}", path.display(), e))?;
        let file_path_str = path.to_string_lossy().to_string();
        let program = parse_kleis_program_with_file(&source, &file_path_str)
            .map_err(|e| format!("Parse error in '{}': {}", path.display(), e.message))?;

        // Load functions from the imported file with that file's path
        self.evaluator
            .load_program_with_file(&program, Some(path.clone()))?;

        // Store for later use in eval thread
        self.loaded_imports.push((program.clone(), path.clone()));

        // Recursively load this file's imports
        for item in &program.items {
            if let kleis::kleis_ast::TopLevel::Import(import_path) = item
                && let Some(resolved) = resolve_import_path(import_path, path)
            {
                self.load_import(&resolved)?;
            }
        }

        Ok(())
    }

    /// Check if a line is a valid breakpoint location (has executable code)
    /// Uses the AST to determine valid locations
    #[allow(dead_code)]
    fn is_valid_breakpoint_line(&self, line: u32) -> bool {
        self.is_valid_breakpoint_line_in_file(line, None)
    }

    /// Check if a line is a valid breakpoint location in a specific file
    fn is_valid_breakpoint_line_in_file(&self, line: u32, file_path: Option<&PathBuf>) -> bool {
        // Helper to check a program for valid breakpoint lines
        let check_program = |program: &kleis::kleis_ast::Program| -> bool {
            for item in &program.items {
                if let kleis::kleis_ast::TopLevel::ExampleBlock(example) = item {
                    for stmt in &example.statements {
                        let stmt_line = match stmt {
                            kleis::kleis_ast::ExampleStatement::Let { location, .. } => {
                                location.as_ref().map(|l| l.line)
                            }
                            kleis::kleis_ast::ExampleStatement::Assert { location, .. } => {
                                location.as_ref().map(|l| l.line)
                            }
                            kleis::kleis_ast::ExampleStatement::Expr { location, .. } => {
                                location.as_ref().map(|l| l.line)
                            }
                        };
                        if stmt_line == Some(line) {
                            return true;
                        }
                    }
                }
                if let kleis::kleis_ast::TopLevel::FunctionDef(func) = item
                    && let Some(ref span) = func.span
                    && span.line == line
                {
                    return true;
                }
            }
            false
        };

        // If no file path given or file matches main program, check main program
        if (file_path.is_none() || file_path == self.current_file.as_ref())
            && let Some(ref program) = self.program
            && check_program(program)
        {
            return true;
        }

        // Check imported files
        if let Some(file_path) = file_path {
            for (import_program, import_path) in &self.loaded_imports {
                if import_path == file_path && check_program(import_program) {
                    return true;
                }
            }
        }

        false
    }
}

fn run_dap_on_listener(
    listener: TcpListener,
    ast_cache: AstCache,
    verbose: bool,
) -> std::result::Result<(), String> {
    use std::io::{BufRead, BufReader, Read, Write};

    if verbose {
        eprintln!("[kleis-dap] Waiting for connection...");
    }

    // Accept one connection
    let (stream, addr) = listener
        .accept()
        .map_err(|e| format!("Accept failed: {}", e))?;

    if verbose {
        eprintln!("[kleis-dap] Connected from {}", addr);
    }

    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut writer = stream;

    // Create DAP state with shared AST cache and its own evaluator
    let mut state = DapState::new(ast_cache.clone());

    // Simple DAP message loop
    loop {
        // Read Content-Length header
        let mut header = String::new();
        if reader.read_line(&mut header).map_err(|e| e.to_string())? == 0 {
            break; // EOF
        }

        let content_length: usize = if header.starts_with("Content-Length:") {
            header
                .trim_start_matches("Content-Length:")
                .trim()
                .parse()
                .unwrap_or(0)
        } else {
            continue;
        };

        // Skip blank line
        let mut blank = String::new();
        reader.read_line(&mut blank).ok();

        // Read content
        let mut content = vec![0u8; content_length];
        reader.read_exact(&mut content).map_err(|e| e.to_string())?;

        // Parse and handle request
        if let Ok(request) = serde_json::from_slice::<serde_json::Value>(&content) {
            if verbose && let Some(cmd) = request.get("command").and_then(|c| c.as_str()) {
                eprintln!("[kleis-dap] Request: {}", cmd);
            }

            // Handle the request - may return multiple messages (response + events)
            let messages = handle_dap_request(&request, &mut state);

            // Send all messages
            for msg in messages {
                let msg_str = serde_json::to_string(&msg).unwrap();
                let header = format!("Content-Length: {}\r\n\r\n", msg_str.len());
                writer.write_all(header.as_bytes()).ok();
                writer.write_all(msg_str.as_bytes()).ok();
                writer.flush().ok();
            }

            // Check for terminate
            if request.get("command").and_then(|c| c.as_str()) == Some("disconnect") {
                break;
            }
        }
    }

    if verbose {
        eprintln!("[kleis-dap] Session ended");
    }

    Ok(())
}

/// Handle a DAP request with actual evaluator integration
/// Returns a vector of messages to send (response + any events)
/// The evaluator is now part of state (state.evaluator)
fn handle_dap_request(request: &serde_json::Value, state: &mut DapState) -> Vec<serde_json::Value> {
    let seq = request.get("seq").and_then(|s| s.as_i64()).unwrap_or(0);
    let command = request
        .get("command")
        .and_then(|c| c.as_str())
        .unwrap_or("");

    match command {
        "initialize" => {
            // Return both the response AND the initialized event
            // The initialized event tells VS Code we're ready for configuration
            vec![
                serde_json::json!({
                    "seq": 1,
                    "type": "response",
                    "request_seq": seq,
                    "success": true,
                    "command": "initialize",
                    "body": {
                        "supportsConfigurationDoneRequest": true,
                        "supportsEvaluateForHovers": true,
                        "supportsConditionalBreakpoints": true,
                        "supportsStepInTargetsRequest": true
                    }
                }),
                // CRITICAL: The initialized EVENT - without this, VS Code won't proceed!
                serde_json::json!({
                    "seq": 2,
                    "type": "event",
                    "event": "initialized"
                }),
            ]
        }
        "launch" => {
            use kleis::debug::DapDebugHook;

            // Load program from file
            let program_path = request
                .get("arguments")
                .and_then(|a| a.get("program"))
                .and_then(|p| p.as_str());

            eprintln!("[kleis-dap] Launch request, program: {:?}", program_path);

            if let Some(program_path) = program_path {
                // Load file into state - this parses executable lines and loads into evaluator
                if let Err(e) = state.load_file(program_path) {
                    return vec![serde_json::json!({
                        "seq": 1,
                        "type": "response",
                        "request_seq": seq,
                        "success": false,
                        "command": "launch",
                        "message": e
                    })];
                }

                eprintln!(
                    "[kleis-dap] Loaded file, starting at line {}",
                    state.current_line
                );

                // Note: load_file already stores the program in state.program

                // Create the debug hook and controller for channel-based communication
                // Breakpoints are shared between controller and hook via Arc<RwLock>
                let (mut hook, controller) = DapDebugHook::new();

                // Set the current file on the hook
                if let Some(ref file_path) = state.current_file {
                    hook.set_file(file_path.clone());
                }

                // Store controller and hook
                // Note: Breakpoints are shared via controller.breakpoints (Arc<RwLock>)
                // setBreakpoints handler will update them, evaluator sees changes immediately
                state.controller = Some(controller);
                state.pending_hook = Some(hook);

                // Note: We'll set the hook on the evaluator and spawn the eval thread
                // in configurationDone (VS Code sends breakpoints between launch and configurationDone)

                eprintln!("[kleis-dap] Hook and controller created, waiting for configurationDone");
            } else {
                eprintln!("[kleis-dap] No program path provided!");
                return vec![serde_json::json!({
                    "seq": 1,
                    "type": "response",
                    "request_seq": seq,
                    "success": false,
                    "command": "launch",
                    "message": "No program path provided"
                })];
            }
            vec![serde_json::json!({
                "seq": 1,
                "type": "response",
                "request_seq": seq,
                "success": true,
                "command": "launch"
            })]
        }
        "attach" => {
            vec![serde_json::json!({
                "seq": 1,
                "type": "response",
                "request_seq": seq,
                "success": true,
                "command": "attach"
            })]
        }
        "threads" => {
            vec![serde_json::json!({
                "seq": 1,
                "type": "response",
                "request_seq": seq,
                "success": true,
                "command": "threads",
                "body": {
                    "threads": [{
                        "id": 1,
                        "name": "main"
                    }]
                }
            })]
        }
        "stackTrace" => {
            // Try to use real stack frames from the last stop event
            let stack_frames: Vec<serde_json::Value> =
                if let Some(ref stop_event) = state.last_stop_event {
                    stop_event
                        .stack
                        .iter()
                        .enumerate()
                        .map(|(i, frame)| {
                            let file_path = frame
                                .location
                                .file
                                .as_ref()
                                .map(|p| p.to_string_lossy().to_string())
                                .unwrap_or_default();
                            let file_name = frame
                                .location
                                .file
                                .as_ref()
                                .and_then(|p| p.file_name())
                                .map(|n| n.to_string_lossy().to_string())
                                .unwrap_or_else(|| "unknown".to_string());

                            serde_json::json!({
                                "id": i + 1,
                                "name": frame.name,
                                "source": {
                                    "name": file_name,
                                    "path": file_path
                                },
                                "line": frame.location.line,
                                "column": frame.location.column.max(1)
                            })
                        })
                        .collect()
                } else {
                    // Fallback to simulated single frame
                    let file_path = state
                        .current_file
                        .as_ref()
                        .map(|p| p.to_string_lossy().to_string())
                        .unwrap_or_default();
                    let file_name = state
                        .current_file
                        .as_ref()
                        .and_then(|p| p.file_name())
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| "unknown".to_string());

                    vec![serde_json::json!({
                        "id": 1,
                        "name": "<top-level>",
                        "source": {
                            "name": file_name,
                            "path": file_path
                        },
                        "line": state.current_line,
                        "column": 1
                    })]
                };

            let total_frames = stack_frames.len();

            vec![serde_json::json!({
                "seq": 1,
                "type": "response",
                "request_seq": seq,
                "success": true,
                "command": "stackTrace",
                "body": {
                    "stackFrames": stack_frames,
                    "totalFrames": total_frames
                }
            })]
        }
        "scopes" => {
            vec![serde_json::json!({
                "seq": 1,
                "type": "response",
                "request_seq": seq,
                "success": true,
                "command": "scopes",
                "body": {
                    "scopes": [{
                        "name": "Globals",
                        "variablesReference": 1,
                        "expensive": false
                    }]
                }
            })]
        }
        "variables" => {
            let mut variables = Vec::new();

            // Try to get bindings from the current stack frame in the stop event
            // Bindings now include type information (TypedBinding)
            if let Some(ref stop_event) = state.last_stop_event
                && let Some(frame) = stop_event.stack.first()
            {
                for (name, binding) in &frame.bindings {
                    // DAP supports a "type" field for variables
                    let type_str = binding.ty.as_ref().map(kleis::debug::format_type);
                    let mut var_json = serde_json::json!({
                        "name": name,
                        "value": &binding.value,
                        "variablesReference": 0
                    });
                    // Add type field if available
                    if let Some(ref ty) = type_str {
                        var_json["type"] = serde_json::Value::String(ty.clone());
                    }
                    // Add verification badge if available
                    if let Some(verified) = binding.verified {
                        let badge = if verified { " ✓" } else { " ✗" };
                        if let Some(val) = var_json.get("value").and_then(|v| v.as_str()) {
                            var_json["value"] =
                                serde_json::Value::String(format!("{}{}", val, badge));
                        }
                    }
                    variables.push(var_json);
                }
            }

            // Also include evaluator's global bindings (as fallback or supplement)
            for (name, value) in state.evaluator.get_all_bindings() {
                // Avoid duplicates
                let already_exists = variables
                    .iter()
                    .any(|v| v.get("name").and_then(|n| n.as_str()) == Some(name));
                if !already_exists {
                    variables.push(serde_json::json!({
                        "name": name,
                        "value": format!("{:?}", value),
                        "variablesReference": 0
                    }));
                }
            }

            // Show function count
            let func_count = state.evaluator.list_functions().len();
            variables.push(serde_json::json!({
                "name": "<functions>",
                "value": format!("{} defined", func_count),
                "variablesReference": 0
            }));

            vec![serde_json::json!({
                "seq": 1,
                "type": "response",
                "request_seq": seq,
                "success": true,
                "command": "variables",
                "body": {
                    "variables": variables
                }
            })]
        }
        "evaluate" => {
            // Evaluate an expression using DAP's evaluator
            let result = if let Some(expr_str) = request
                .get("arguments")
                .and_then(|a| a.get("expression"))
                .and_then(|e| e.as_str())
            {
                match kleis::kleis_parser::parse_kleis(expr_str) {
                    Ok(expr) => match state.evaluator.eval(&expr) {
                        Ok(result) => format!("{:?}", result),
                        Err(e) => format!("Error: {}", e),
                    },
                    Err(e) => format!("Parse error: {}", e),
                }
            } else {
                "No expression".to_string()
            };
            vec![serde_json::json!({
                "seq": 1,
                "type": "response",
                "request_seq": seq,
                "success": true,
                "command": "evaluate",
                "body": {
                    "result": result,
                    "variablesReference": 0
                }
            })]
        }
        "setBreakpoints" => {
            use kleis::debug::Breakpoint as DebugBreakpoint;

            // Extract requested breakpoint lines and validate them
            let mut breakpoints_response = Vec::new();

            // Get file path from source in arguments
            let file_path = request
                .get("arguments")
                .and_then(|a| a.get("source"))
                .and_then(|s| s.get("path"))
                .and_then(|p| p.as_str())
                .map(PathBuf::from);

            // Clear old breakpoints for this file and add new ones
            // Uses shared breakpoints via controller (thread-safe, visible to evaluator)
            if let Some(ref controller) = state.controller {
                if let Some(ref path) = file_path {
                    // Clear breakpoints for this file
                    if let Ok(mut bps) = controller.breakpoints.write() {
                        bps.retain(|bp| &bp.file != path);
                    }
                }

                if let Some(args) = request.get("arguments")
                    && let Some(bps) = args.get("breakpoints").and_then(|b| b.as_array())
                {
                    for bp in bps {
                        if let Some(line) = bp.get("line").and_then(|l| l.as_u64()) {
                            let line = line as u32;
                            let is_valid =
                                state.is_valid_breakpoint_line_in_file(line, file_path.as_ref());

                            if is_valid
                                && let Some(ref path) = file_path
                                && let Ok(mut shared_bps) = controller.breakpoints.write()
                            {
                                shared_bps.push(DebugBreakpoint::new(path.clone(), line));
                            }

                            let mut bp_resp = serde_json::json!({
                                "verified": is_valid,
                                "line": line
                            });
                            if !is_valid {
                                bp_resp["message"] = serde_json::Value::String(
                                    "Cannot set breakpoint on this line (no executable code)"
                                        .to_string(),
                                );
                            }
                            breakpoints_response.push(bp_resp);
                        }
                    }
                }
            } else {
                // No controller yet (before launch) - just validate
                if let Some(args) = request.get("arguments")
                    && let Some(bps) = args.get("breakpoints").and_then(|b| b.as_array())
                {
                    for bp in bps {
                        if let Some(line) = bp.get("line").and_then(|l| l.as_u64()) {
                            let line = line as u32;
                            let is_valid =
                                state.is_valid_breakpoint_line_in_file(line, file_path.as_ref());
                            let mut bp_resp = serde_json::json!({
                                "verified": is_valid,
                                "line": line
                            });
                            if !is_valid {
                                bp_resp["message"] = serde_json::Value::String(
                                    "Cannot set breakpoint on this line (no executable code)"
                                        .to_string(),
                                );
                            }
                            breakpoints_response.push(bp_resp);
                        }
                    }
                }
            }

            vec![serde_json::json!({
                "seq": 1,
                "type": "response",
                "request_seq": seq,
                "success": true,
                "command": "setBreakpoints",
                "body": { "breakpoints": breakpoints_response }
            })]
        }
        "configurationDone" => {
            // Take the pending hook and spawn the evaluation thread
            if let Some(hook) = state.pending_hook.take() {
                // Get the program to evaluate
                let program = state.program.clone();
                let current_file = state.current_file.clone();

                if let Some(program) = program {
                    // Create a fresh evaluator for the eval thread
                    let mut eval_evaluator = Evaluator::new();

                    // Load the program with file path for cross-file debugging
                    if let Err(e) = eval_evaluator.load_program_with_file(&program, current_file) {
                        eprintln!("[kleis-dap] Failed to load program: {}", e);
                    }

                    // Load all imported files into the eval evaluator
                    for (import_program, import_path) in &state.loaded_imports {
                        if let Err(e) = eval_evaluator
                            .load_program_with_file(import_program, Some(import_path.clone()))
                        {
                            eprintln!(
                                "[kleis-dap] Failed to load import '{}': {}",
                                import_path.display(),
                                e
                            );
                        }
                    }

                    // Set the debug hook on the evaluator
                    eval_evaluator.set_debug_hook(Box::new(hook));

                    // Spawn the evaluation thread
                    let handle = std::thread::spawn(move || {
                        eprintln!("[kleis-dap] Eval thread started");

                        // Find and evaluate example blocks
                        for item in &program.items {
                            if let kleis::kleis_ast::TopLevel::ExampleBlock(example) = item {
                                eprintln!(
                                    "[kleis-dap] Evaluating example block: {}",
                                    if example.name.is_empty() {
                                        "(anonymous)"
                                    } else {
                                        &example.name
                                    }
                                );
                                let result = eval_evaluator.eval_example_block(example);
                                if result.passed {
                                    eprintln!("[kleis-dap] Example passed");
                                } else {
                                    eprintln!("[kleis-dap] Example failed: {:?}", result.error);
                                }
                                break; // Only evaluate first example for now
                            }
                        }

                        eprintln!("[kleis-dap] Eval thread finished");
                    });

                    state.eval_thread = Some(handle);
                    eprintln!("[kleis-dap] Eval thread spawned");

                    // Wait for the first stop event from the hook
                    if let Some(ref controller) = state.controller {
                        // Try to receive with a short timeout
                        match controller
                            .event_rx
                            .recv_timeout(std::time::Duration::from_millis(500))
                        {
                            Ok(stop_event) => {
                                eprintln!(
                                    "[kleis-dap] Received stop event: {:?} at line {}",
                                    stop_event.reason, stop_event.location.line
                                );
                                state.current_line = stop_event.location.line;
                                if let Some(ref file) = stop_event.location.file {
                                    state.current_file = Some(file.clone());
                                }
                                state.last_stop_event = Some(stop_event);
                            }
                            Err(e) => {
                                eprintln!(
                                    "[kleis-dap] No stop event received (may have completed quickly): {:?}",
                                    e
                                );
                            }
                        }
                    }
                } else {
                    eprintln!("[kleis-dap] No program to evaluate");
                }
            } else {
                eprintln!("[kleis-dap] No pending hook (launch not called?)");
            }

            // Response + stopped event + output events
            vec![
                // Response
                serde_json::json!({
                    "seq": 1,
                    "type": "response",
                    "request_seq": seq,
                    "success": true,
                    "command": "configurationDone"
                }),
                // Output to Debug Console
                serde_json::json!({
                    "seq": 2,
                    "type": "event",
                    "event": "output",
                    "body": {
                        "category": "console",
                        "output": "🐛 Kleis Debugger\nPaused at entry point. Use Step Over (F10) to step through code.\n"
                    }
                }),
                // Stopped event - THIS IS CRITICAL for VS Code to show Paused state
                serde_json::json!({
                    "seq": 3,
                    "type": "event",
                    "event": "stopped",
                    "body": {
                        "reason": "entry",
                        "description": "Paused on entry",
                        "threadId": 1,
                        "allThreadsStopped": true
                    }
                }),
            ]
        }
        "continue" => {
            use kleis::debug::DebugAction;

            // Try to use the real evaluator via channels
            if let Some(ref controller) = state.controller {
                eprintln!("[kleis-dap] Sending Continue action");

                if let Err(e) = controller.command_tx.send(DebugAction::Continue) {
                    eprintln!("[kleis-dap] Failed to send Continue: {:?}", e);
                } else {
                    // Wait for the next stop event (breakpoint or end)
                    match controller
                        .event_rx
                        .recv_timeout(std::time::Duration::from_secs(30))
                    {
                        Ok(stop_event) => {
                            eprintln!(
                                "[kleis-dap] Received stop event: {:?} at line {}",
                                stop_event.reason, stop_event.location.line
                            );
                            state.current_line = stop_event.location.line;
                            if let Some(ref file) = stop_event.location.file {
                                state.current_file = Some(file.clone());
                            }
                            state.last_stop_event = Some(stop_event.clone());

                            let reason = match stop_event.reason {
                                kleis::debug::StopReason::Breakpoint => "breakpoint",
                                kleis::debug::StopReason::Step => "step",
                                kleis::debug::StopReason::Entry => "entry",
                                kleis::debug::StopReason::Pause => "pause",
                            };

                            // Build description with expression info
                            let description =
                                if let Some(ref expr_desc) = stop_event.expression_desc {
                                    format!("Evaluating: {}", expr_desc)
                                } else {
                                    format!("Stopped at line {}", stop_event.location.line)
                                };
                            eprintln!("[kleis-dap] {}", description);

                            return vec![
                                serde_json::json!({
                                    "seq": 1,
                                    "type": "response",
                                    "request_seq": seq,
                                    "success": true,
                                    "command": "continue"
                                }),
                                serde_json::json!({
                                    "seq": 2,
                                    "type": "event",
                                    "event": "stopped",
                                    "body": {
                                        "reason": reason,
                                        "description": description,
                                        "text": stop_event.expression_desc.clone().unwrap_or_default(),
                                        "threadId": 1,
                                        "allThreadsStopped": true
                                    }
                                }),
                            ];
                        }
                        Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                            eprintln!("[kleis-dap] Evaluator finished");
                            return vec![
                                serde_json::json!({
                                    "seq": 1,
                                    "type": "response",
                                    "request_seq": seq,
                                    "success": true,
                                    "command": "continue"
                                }),
                                serde_json::json!({
                                    "seq": 2,
                                    "type": "event",
                                    "event": "output",
                                    "body": {
                                        "category": "console",
                                        "output": "\n✅ Execution completed\n"
                                    }
                                }),
                                serde_json::json!({
                                    "seq": 3,
                                    "type": "event",
                                    "event": "terminated"
                                }),
                            ];
                        }
                        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                            eprintln!("[kleis-dap] Timeout waiting for stop");
                        }
                    }
                }
            }

            // Fallback: no controller available, terminate
            eprintln!("[kleis-dap] No controller available for continue");
            vec![
                serde_json::json!({
                    "seq": 1,
                    "type": "response",
                    "request_seq": seq,
                    "success": true,
                    "command": "continue"
                }),
                serde_json::json!({
                    "seq": 2,
                    "type": "event",
                    "event": "output",
                    "body": {
                        "category": "console",
                        "output": "⚠️ No debug session active\n"
                    }
                }),
                serde_json::json!({
                    "seq": 3,
                    "type": "event",
                    "event": "terminated"
                }),
            ]
        }
        "next" | "stepIn" | "stepOut" => {
            use kleis::debug::DebugAction;

            // Determine which action to send
            let action = match command {
                "next" => DebugAction::StepOver,
                "stepIn" => DebugAction::StepInto,
                "stepOut" => DebugAction::StepOut,
                _ => DebugAction::Continue,
            };

            // Try to use the real evaluator via channels
            if let Some(ref controller) = state.controller {
                eprintln!("[kleis-dap] Sending {:?} action", action);

                // Send the action to the evaluator thread
                if let Err(e) = controller.command_tx.send(action) {
                    eprintln!("[kleis-dap] Failed to send action: {:?}", e);
                    // Fall back to simulated stepping
                } else {
                    // Wait for the stop event
                    match controller
                        .event_rx
                        .recv_timeout(std::time::Duration::from_secs(5))
                    {
                        Ok(stop_event) => {
                            eprintln!(
                                "[kleis-dap] Received stop event: {:?} at line {}",
                                stop_event.reason, stop_event.location.line
                            );
                            state.current_line = stop_event.location.line;
                            if let Some(ref file) = stop_event.location.file {
                                state.current_file = Some(file.clone());
                            }
                            state.last_stop_event = Some(stop_event.clone());

                            let reason = match stop_event.reason {
                                kleis::debug::StopReason::Breakpoint => "breakpoint",
                                kleis::debug::StopReason::Step => "step",
                                kleis::debug::StopReason::Entry => "entry",
                                kleis::debug::StopReason::Pause => "pause",
                            };

                            // Build description with expression info
                            let description =
                                if let Some(ref expr_desc) = stop_event.expression_desc {
                                    format!("Evaluating: {}", expr_desc)
                                } else {
                                    format!("Stopped at line {}", stop_event.location.line)
                                };
                            eprintln!("[kleis-dap] {}", description);

                            return vec![
                                serde_json::json!({
                                    "seq": 1,
                                    "type": "response",
                                    "request_seq": seq,
                                    "success": true,
                                    "command": command
                                }),
                                serde_json::json!({
                                    "seq": 2,
                                    "type": "event",
                                    "event": "stopped",
                                    "body": {
                                        "reason": reason,
                                        "description": description,
                                        "text": stop_event.expression_desc.clone().unwrap_or_default(),
                                        "threadId": 1,
                                        "allThreadsStopped": true
                                    }
                                }),
                            ];
                        }
                        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                            eprintln!("[kleis-dap] Timeout waiting for stop event");
                        }
                        Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                            eprintln!("[kleis-dap] Evaluator thread disconnected (finished)");
                            // Program completed
                            return vec![
                                serde_json::json!({
                                    "seq": 1,
                                    "type": "response",
                                    "request_seq": seq,
                                    "success": true,
                                    "command": command
                                }),
                                serde_json::json!({
                                    "seq": 2,
                                    "type": "event",
                                    "event": "output",
                                    "body": {
                                        "category": "console",
                                        "output": "\n✅ Execution completed\n"
                                    }
                                }),
                                serde_json::json!({
                                    "seq": 3,
                                    "type": "event",
                                    "event": "terminated"
                                }),
                            ];
                        }
                    }
                }
            }

            // Fallback: no controller available, terminate
            eprintln!("[kleis-dap] No controller available for step command");
            vec![
                serde_json::json!({
                    "seq": 1,
                    "type": "response",
                    "request_seq": seq,
                    "success": true,
                    "command": command
                }),
                serde_json::json!({
                    "seq": 2,
                    "type": "event",
                    "event": "output",
                    "body": {
                        "category": "console",
                        "output": "⚠️ No debug session active\n"
                    }
                }),
                serde_json::json!({
                    "seq": 3,
                    "type": "event",
                    "event": "terminated"
                }),
            ]
        }
        "disconnect" | "terminate" => {
            vec![serde_json::json!({
                "seq": 1,
                "type": "response",
                "request_seq": seq,
                "success": true,
                "command": command
            })]
        }
        _ => {
            vec![serde_json::json!({
                "seq": 1,
                "type": "response",
                "request_seq": seq,
                "success": true,
                "command": command
            })]
        }
    }
}
