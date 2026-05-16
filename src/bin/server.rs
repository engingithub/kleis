#![allow(warnings)]
#![allow(clippy::all, unreachable_patterns)]
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]
use axum::{
    Router,
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse, Json},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;

// For now, we'll work with LaTeX strings directly
// Later we'll add proper Expression parsing

/// Recursively load imports from a program into the evaluator
fn load_imports_recursive(
    program: &kleis::kleis_ast::Program,
    file_path: &std::path::Path,
    evaluator: &mut kleis::evaluator::Evaluator,
    loaded_files: &mut std::collections::HashSet<std::path::PathBuf>,
) -> Result<(), String> {
    use kleis::kleis_ast::TopLevel;
    use kleis::kleis_parser::parse_kleis_program_with_file;

    let base_dir = file_path.parent().unwrap_or(std::path::Path::new("."));

    for item in &program.items {
        if let TopLevel::Import(import_path_str) = item {
            let import_path = std::path::Path::new(import_path_str);
            let resolved = if import_path.is_absolute() {
                import_path.to_path_buf()
            } else if import_path_str.starts_with("stdlib/") {
                // Check KLEIS_ROOT environment variable first
                if let Ok(kleis_root) = std::env::var("KLEIS_ROOT") {
                    let candidate = std::path::PathBuf::from(&kleis_root).join(import_path_str);
                    if candidate.exists() {
                        candidate
                    } else {
                        std::path::PathBuf::from(import_path_str)
                    }
                } else {
                    std::path::PathBuf::from(import_path_str)
                }
            } else {
                base_dir.join(import_path)
            };

            let canonical = match resolved.canonicalize() {
                Ok(c) => c,
                Err(_) => {
                    // Try without canonicalize for stdlib paths
                    if resolved.exists() {
                        resolved.clone()
                    } else {
                        continue; // Skip missing imports silently
                    }
                }
            };

            if loaded_files.contains(&canonical) {
                continue;
            }
            loaded_files.insert(canonical.clone());

            let source = match std::fs::read_to_string(&canonical) {
                Ok(s) => s,
                Err(_) => continue, // Skip unreadable files
            };

            let file_path_str = canonical.to_string_lossy().to_string();
            let import_program = match parse_kleis_program_with_file(&source, &file_path_str) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("⚠️  Parse error in '{}': {}", import_path_str, e);
                    continue;
                }
            };

            // Recursively load imports from the imported file
            load_imports_recursive(&import_program, &canonical, evaluator, loaded_files)?;

            // Load the program into evaluator
            evaluator.load_program_with_file(&import_program, Some(canonical.clone()))?;
        }
    }
    Ok(())
}

/// Load stdlib files into a StructureRegistry for Z3 verification.
fn load_stdlib_registry(extra_imports: &[String]) -> kleis::structure_registry::StructureRegistry {
    use kleis::evaluator::Evaluator;
    use kleis::kleis_parser::parse_kleis_program_with_file;
    use kleis::structure_registry::StructureRegistry;

    let mut evaluator = Evaluator::new();
    let mut registry = StructureRegistry::default();
    let mut loaded_files = std::collections::HashSet::new();

    let mut all_files: Vec<String> = vec![
        "stdlib/minimal_prelude.kleis".to_string(),
        "stdlib/lists.kleis".to_string(),
        "stdlib/matrices.kleis".to_string(),
    ];
    for import in extra_imports {
        if !all_files.contains(import) {
            all_files.push(import.clone());
        }
    }

    for file_path_str in &all_files {
        let file_path = std::path::Path::new(file_path_str);
        match std::fs::read_to_string(file_path) {
            Ok(source) => {
                match parse_kleis_program_with_file(&source, file_path_str) {
                    Ok(program) => {
                        // First, recursively load all imports
                        if let Err(e) = load_imports_recursive(
                            &program,
                            file_path,
                            &mut evaluator,
                            &mut loaded_files,
                        ) {
                            eprintln!("⚠️  Error loading imports from {}: {}", file_path_str, e);
                        }

                        // Then load the file itself
                        let canonical = file_path
                            .canonicalize()
                            .unwrap_or_else(|_| file_path.to_path_buf());
                        if !loaded_files.contains(&canonical) {
                            loaded_files.insert(canonical.clone());
                            if let Err(e) =
                                evaluator.load_program_with_file(&program, Some(canonical))
                            {
                                eprintln!("⚠️  Error loading {}: {}", file_path_str, e);
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("⚠️  Parse error in {}: {}", file_path_str, e);
                    }
                }
            }
            Err(_) => {
                // File doesn't exist, skip silently
            }
        }
    }

    // Build registry from evaluator
    evaluator.build_registry(&mut registry);
    registry
}

#[derive(Debug, Serialize, Deserialize)]
struct RenderRequest {
    latex: String,
    format: Option<String>, // "latex", "unicode", "svg"
}

#[derive(Debug, Serialize)]
struct RenderResponse {
    output: String,
    format: String,
    success: bool,
    error: Option<String>,
}

#[derive(Debug, Serialize)]
struct OperationInfo {
    name: String,
    description: String,
    example_latex: String,
}

#[derive(Debug, Serialize)]
struct TemplateInfo {
    name: String,
    category: Option<String>,
    glyph: Option<String>,
    svg: Option<String>,
    #[serde(skip_serializing_if = "std::collections::HashMap::is_empty")]
    metadata: std::collections::HashMap<String, String>,
}

#[derive(Debug, Serialize)]
struct GalleryResponse {
    examples: Vec<GalleryExample>,
}

#[derive(Debug, Serialize)]
struct GalleryExample {
    title: String,
    latex: String,
}

#[derive(Debug, Deserialize)]
struct ParseRequest {
    latex: String,
}

#[derive(Debug, Serialize)]
struct ParseResponse {
    ast: serde_json::Value,
    success: bool,
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RenderASTRequest {
    ast: serde_json::Value,
    format: Option<String>, // "latex", "unicode", "html"
}

#[derive(Debug, Serialize)]
struct RenderASTResponse {
    output: String,
    format: String,
    success: bool,
    error: Option<String>,
}

// Type check request/response
#[derive(Debug, Deserialize)]
struct TypeCheckRequest {
    ast: serde_json::Value,
}

#[derive(Debug, Serialize)]
struct TypeCheckResponse {
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    type_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    suggestion: Option<String>,
}

// Kleis rendering request/response
#[derive(Debug, Deserialize)]
struct RenderKleisRequest {
    ast: serde_json::Value,
}

#[derive(Debug, Serialize)]
struct RenderKleisResponse {
    kleis: String,
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

// Typst code export request/response (for PhD candidates to copy/paste)
#[derive(Debug, Deserialize)]
struct ExportTypstRequest {
    ast: serde_json::Value,
}

#[derive(Debug, Serialize)]
struct ExportTypstResponse {
    typst: String,
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

// Z3 verification request/response
#[derive(Debug, Deserialize)]
struct VerifyRequest {
    ast: serde_json::Value,
}

#[derive(Debug, Serialize)]
struct VerifyResponse {
    success: bool,
    result: String, // "valid", "invalid", "unknown", "error"
    kleis_syntax: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    counterexample: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Debug, Serialize)]
struct CheckSatResponse {
    success: bool,
    result: String, // "satisfiable", "unsatisfiable", "unknown", "error", "incomplete"
    kleis_syntax: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    example: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct VerifyGraphRequest {
    domain: String,
    components: Vec<VerifyGraphComponent>,
    incidence: VerifyGraphIncidence,
    port_labels: Vec<String>,
}

#[derive(Debug, Deserialize, Clone)]
struct VerifyGraphComponent {
    #[serde(rename = "type")]
    comp_type: String,
    component_type: Option<String>,
    params: Option<std::collections::HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Deserialize, Clone)]
struct VerifyGraphIncidence {
    entries: Vec<VerifyGraphEntry>,
    v: usize,
    p: usize,
}

#[derive(Debug, Deserialize, Clone)]
struct VerifyGraphEntry {
    net: usize,
    port: usize,
    value: i64,
}

#[derive(Debug, Serialize)]
struct VerifyGraphResponse {
    success: bool,
    results: Vec<VerifyGraphExampleResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    preamble: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
struct VerifyGraphExampleResult {
    name: String,
    passed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

// Shared application state
#[derive(Clone)]
struct AppState {
    // TypeChecker loaded from stdlib/matrices.kleis
    type_checker: Arc<std::sync::Mutex<Option<kleis::type_checker::TypeChecker>>>,
    // StructureRegistry preloaded with stdlib for Z3 verification
    registry: Arc<kleis::structure_registry::StructureRegistry>,
}

#[tokio::main]
async fn main() {
    let config = kleis::config::load();

    // Initialize TypeChecker with stdlib (includes minimal_prelude + matrices + tensors + quantum)
    let type_checker = match kleis::type_checker::TypeChecker::with_stdlib() {
        Ok(checker) => {
            println!(
                "✅ TypeChecker initialized with stdlib (minimal_prelude + matrices + tensors + quantum)"
            );
            Some(checker)
        }
        Err(e) => {
            eprintln!("⚠️  Failed to initialize TypeChecker with stdlib: {}", e);
            None
        }
    };

    // Collect @import paths from .kleist template files
    let kleist_imports = {
        let dir = std::path::Path::new("std_template_lib");
        if dir.exists() {
            match kleis::kleist_parser::load_kleist_directory(dir) {
                Ok(file) => file.imports,
                Err(_) => vec![],
            }
        } else {
            vec![]
        }
    };
    if !kleist_imports.is_empty() {
        println!(
            "📦 Found {} @import(s) from .kleist templates: {:?}",
            kleist_imports.len(),
            kleist_imports
        );
    }

    let registry = load_stdlib_registry(&kleist_imports);
    println!(
        "✅ StructureRegistry initialized from stdlib/ with {} structures, {} operations",
        registry.structure_count(),
        registry.operation_count()
    );

    let state = Arc::new(AppState {
        type_checker: Arc::new(std::sync::Mutex::new(type_checker)),
        registry: Arc::new(registry),
    });

    // Build router
    let app = Router::new()
        .route("/", get(index_handler))
        .route("/api/render", post(render_handler))
        .route("/api/render_ast", post(render_ast_handler))
        .route("/api/render_typst", post(render_typst_handler))
        .route("/api/parse", post(parse_handler))
        .route("/api/type_check", post(type_check_handler))
        .route("/api/render_kleis", post(render_kleis_handler))
        .route("/api/export_typst", post(export_typst_handler))
        .route("/api/verify", post(verify_handler))
        .route("/api/check_sat", post(check_sat_handler))
        .route("/api/verify_graph", post(verify_graph_handler))
        .route("/api/simulate_setup", post(simulate_setup_handler))
        .route("/api/simulate_graph", post(simulate_graph_handler))
        .route("/api/operations", get(operations_handler))
        .route("/api/templates", get(templates_handler))
        .route("/api/palette", get(palette_handler))
        .route("/api/gallery", get(gallery_handler))
        .route("/health", get(health_handler))
        .nest_service("/static", ServeDir::new("static"))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let bind_addr = format!("{}:{}", config.server.host, config.server.port);
    let listener = tokio::net::TcpListener::bind(&bind_addr)
        .await
        .unwrap_or_else(|_| panic!("Failed to bind to {}", bind_addr));

    println!("🚀 Kleis Server starting...");
    println!("📡 Server running at: http://{}", bind_addr);
    println!("📚 Gallery available at: http://{}/api/gallery", bind_addr);
    println!("🧪 Health check: http://{}/health", bind_addr);
    println!();
    println!("Press Ctrl+C to stop");

    axum::serve(listener, app)
        .await
        .expect("Server failed to start");
}

// Handler for root path - serves a simple web UI
async fn index_handler() -> impl IntoResponse {
    // Serve dynamically so changes are picked up without recompiling
    match tokio::fs::read_to_string("static/index.html").await {
        Ok(content) => (StatusCode::OK, Html(content)),
        Err(_) => (
            StatusCode::NOT_FOUND,
            Html("<h1>index.html not found. Make sure static/ directory exists.</h1>".to_string()),
        ),
    }
}

// Handler for rendering equations
async fn render_handler(
    State(_state): State<Arc<AppState>>,
    Json(req): Json<RenderRequest>,
) -> impl IntoResponse {
    // Parse LaTeX → Expression → Render
    match kleis::parser::parse_latex(&req.latex) {
        Ok(expr) => {
            let format = req.format.as_deref().unwrap_or("latex");
            let target = match format {
                "unicode" => kleis::render::RenderTarget::Unicode,
                "html" => kleis::render::RenderTarget::HTML,
                _ => kleis::render::RenderTarget::LaTeX,
            };

            let ctx = kleis::render::build_default_context();
            let output = kleis::render::render_expression(&expr, &ctx, &target);

            let response = RenderResponse {
                output,
                format: format.to_string(),
                success: true,
                error: None,
            };
            (StatusCode::OK, Json(response))
        }
        Err(e) => {
            let response = RenderResponse {
                output: String::new(),
                format: req.format.unwrap_or_else(|| "latex".to_string()),
                success: false,
                error: Some(format!("Parse error: {:?}", e)),
            };
            (StatusCode::BAD_REQUEST, Json(response))
        }
    }
}

// Handler for parsing LaTeX into AST
async fn parse_handler(
    State(_state): State<Arc<AppState>>,
    Json(req): Json<ParseRequest>,
) -> impl IntoResponse {
    match kleis::parser::parse_latex(&req.latex) {
        Ok(expr) => {
            // Convert Expression to JSON-serializable format
            let ast_json = expression_to_json(&expr);
            let response = ParseResponse {
                ast: ast_json,
                success: true,
                error: None,
            };
            (StatusCode::OK, Json(response))
        }
        Err(e) => {
            let response = ParseResponse {
                ast: serde_json::Value::Null,
                success: false,
                error: Some(format!("Parse error: {:?}", e)),
            };
            (StatusCode::BAD_REQUEST, Json(response))
        }
    }
}

// Convert Pattern to JSON (Grammar v0.8)
fn pattern_to_json(pattern: &kleis::ast::Pattern) -> serde_json::Value {
    use kleis::ast::Pattern;
    use serde_json::json;
    match pattern {
        Pattern::Wildcard => json!("_"),
        Pattern::Variable(name) => json!({"Variable": name}),
        Pattern::Constant(c) => json!({"Constant": c}),
        Pattern::Constructor { name, args } => json!({
            "Constructor": {
                "name": name,
                "args": args.iter().map(pattern_to_json).collect::<Vec<_>>()
            }
        }),
        Pattern::As { pattern, binding } => json!({
            "As": {
                "pattern": pattern_to_json(pattern),
                "binding": binding
            }
        }),
    }
}

// Convert Expression to JSON (simplified serialization)
fn expression_to_json(expr: &kleis::ast::Expression) -> serde_json::Value {
    use kleis::ast::Expression;
    use serde_json::json;

    match expr {
        Expression::Const(s) => json!({"Const": s}),
        Expression::String(s) => json!({"String": s}),
        Expression::Object(s) => json!({"Object": s}),
        Expression::Placeholder { id, hint } => json!({"Placeholder": {"id": id, "hint": hint}}),
        Expression::Operation { name, args, .. } => {
            let args_json: Vec<serde_json::Value> = args.iter().map(expression_to_json).collect();
            json!({"Operation": {"name": name, "args": args_json}})
        }
        Expression::Match { .. } => {
            // TODO: Implement match expression JSON serialization
            json!({"Match": "not yet implemented"})
        }
        Expression::Quantifier {
            quantifier,
            variables,
            where_clause,
            body,
        } => {
            let _ = where_clause; // TODO: Include where clause in JSON
            json!({
                "Quantifier": {
                    "kind": match quantifier {
                        kleis::ast::QuantifierKind::ForAll => "forall",
                        kleis::ast::QuantifierKind::Exists => "exists",
                    },
                    "variables": variables.iter().map(|v| {
                        json!({"name": v.name, "type": v.type_annotation})
                    }).collect::<Vec<_>>(),
                    "body": expression_to_json(body)
                }
            })
        }
        Expression::List(elements) => {
            let elements_json: Vec<serde_json::Value> =
                elements.iter().map(expression_to_json).collect();
            json!({"List": elements_json})
        }
        Expression::Conditional {
            condition,
            then_branch,
            else_branch,
            ..
        } => {
            json!({
                "Conditional": {
                    "condition": expression_to_json(condition),
                    "then": expression_to_json(then_branch),
                    "else": expression_to_json(else_branch)
                }
            })
        }
        Expression::Let {
            pattern,
            type_annotation,
            value,
            body,
            ..
        } => {
            json!({
                "Let": {
                    "pattern": pattern_to_json(pattern),
                    "type_annotation": type_annotation,
                    "value": expression_to_json(value),
                    "body": expression_to_json(body)
                }
            })
        }
        Expression::Ascription {
            expr,
            type_annotation,
        } => json!({
            "Ascription": {
                "expr": expression_to_json(expr),
                "type_annotation": type_annotation
            }
        }),
        Expression::Lambda { params, body, .. } => {
            let param_objs: Vec<_> = params
                .iter()
                .map(|p| {
                    json!({
                        "name": p.name,
                        "type_annotation": p.type_annotation
                    })
                })
                .collect();
            json!({
                "Lambda": {
                    "params": param_objs,
                    "body": expression_to_json(body)
                }
            })
        }
    }
}

// Convert JSON back to Expression
fn json_to_expression(json: &serde_json::Value) -> Result<kleis::ast::Expression, String> {
    use kleis::ast::Expression;

    if let Some(obj) = json.as_object() {
        if let Some(const_val) = obj.get("Const") {
            if let Some(s) = const_val.as_str() {
                return Ok(Expression::Const(s.to_string()));
            }
        } else if let Some(obj_val) = obj.get("Object") {
            if let Some(s) = obj_val.as_str() {
                return Ok(Expression::Object(s.to_string()));
            }
        } else if let Some(placeholder_val) = obj.get("Placeholder") {
            if let Some(placeholder_obj) = placeholder_val.as_object() {
                let id = placeholder_obj
                    .get("id")
                    .and_then(|v| v.as_u64())
                    .ok_or("Missing or invalid placeholder id")? as usize;
                let hint = placeholder_obj
                    .get("hint")
                    .and_then(|v| v.as_str())
                    .ok_or("Missing or invalid placeholder hint")?
                    .to_string();
                return Ok(Expression::Placeholder { id, hint });
            }
        } else if let Some(op_val) = obj.get("Operation") {
            if let Some(op_obj) = op_val.as_object() {
                let name = op_obj
                    .get("name")
                    .and_then(|v| v.as_str())
                    .ok_or("Missing or invalid operation name")?
                    .to_string();
                let args_json = op_obj
                    .get("args")
                    .and_then(|v| v.as_array())
                    .ok_or("Missing or invalid operation args")?;
                let args: Result<Vec<Expression>, String> =
                    args_json.iter().map(json_to_expression).collect();
                return Ok(Expression::Operation {
                    name,
                    args: args?,
                    span: None,
                });
            }
        } else if let Some(list_val) = obj.get("List") {
            if let Some(list_array) = list_val.as_array() {
                let elements: Result<Vec<Expression>, String> =
                    list_array.iter().map(json_to_expression).collect();
                return Ok(Expression::List(elements?));
            }
        }
    }

    Err(format!("Invalid expression JSON: {:?}", json))
}

// =============================================================================
// EditorNode JSON conversion - for Visual Editor AST
// =============================================================================

/// Convert JSON to EditorNode (Visual Editor AST with kind/metadata support)
fn json_to_editor_node(json: &serde_json::Value) -> Result<kleis::editor_ast::EditorNode, String> {
    use kleis::editor_ast::{EditorNode, OperationData, PlaceholderData};

    if let Some(obj) = json.as_object() {
        // Handle Const
        if let Some(const_val) = obj.get("Const") {
            if let Some(s) = const_val.as_str() {
                return Ok(EditorNode::Const {
                    value: s.to_string(),
                });
            }
        }
        // Handle Object
        else if let Some(obj_val) = obj.get("Object") {
            if let Some(s) = obj_val.as_str() {
                return Ok(EditorNode::Object {
                    object: s.to_string(),
                });
            }
        }
        // Handle Placeholder
        else if let Some(placeholder_val) = obj.get("Placeholder") {
            if let Some(placeholder_obj) = placeholder_val.as_object() {
                let id = placeholder_obj
                    .get("id")
                    .and_then(|v| v.as_u64())
                    .ok_or("Missing or invalid placeholder id")? as usize;
                let hint = placeholder_obj
                    .get("hint")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                return Ok(EditorNode::Placeholder {
                    placeholder: PlaceholderData { id, hint },
                });
            }
        }
        // Handle Operation (with optional kind/metadata)
        else if let Some(op_val) = obj.get("Operation") {
            if let Some(op_obj) = op_val.as_object() {
                let name = op_obj
                    .get("name")
                    .and_then(|v| v.as_str())
                    .ok_or("Missing or invalid operation name")?
                    .to_string();

                let args_json = op_obj
                    .get("args")
                    .and_then(|v| v.as_array())
                    .ok_or("Missing or invalid operation args")?;

                let args: Result<Vec<EditorNode>, String> =
                    args_json.iter().map(json_to_editor_node).collect();

                // Extract optional kind
                let kind = op_obj
                    .get("kind")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                // Extract optional metadata
                let metadata = op_obj.get("metadata").and_then(|v| {
                    if let Some(meta_obj) = v.as_object() {
                        let map: std::collections::HashMap<String, serde_json::Value> = meta_obj
                            .iter()
                            .map(|(k, v)| (k.clone(), v.clone()))
                            .collect();
                        if map.is_empty() { None } else { Some(map) }
                    } else {
                        None
                    }
                });

                return Ok(EditorNode::Operation {
                    operation: OperationData {
                        name,
                        args: args?,
                        kind,
                        metadata,
                    },
                });
            }
        }
        // Handle List
        else if let Some(list_val) = obj.get("List") {
            if let Some(list_array) = list_val.as_array() {
                let elements: Result<Vec<EditorNode>, String> =
                    list_array.iter().map(json_to_editor_node).collect();
                return Ok(EditorNode::List { list: elements? });
            }
        }
    }

    Err(format!("Invalid EditorNode JSON: {:?}", json))
}

// Handler for rendering from AST (uses EditorNode renderer)
async fn render_ast_handler(
    State(_state): State<Arc<AppState>>,
    Json(req): Json<RenderASTRequest>,
) -> impl IntoResponse {
    let format = req.format.as_deref().unwrap_or("html");
    let target = match format {
        "unicode" => kleis::render_editor::RenderTarget::Unicode,
        "latex" => kleis::render_editor::RenderTarget::LaTeX,
        "typst" => kleis::render_editor::RenderTarget::Typst,
        "kleis" => kleis::render_editor::RenderTarget::Kleis,
        _ => kleis::render_editor::RenderTarget::HTML,
    };

    // Parse as EditorNode - works for both old format (kind: None) and new format
    // Using render_editor module which preserves metadata (fixes tensor index bug)
    match json_to_editor_node(&req.ast) {
        Ok(node) => {
            let output = kleis::render_editor::render_editor_node(&node, &target);
            let response = RenderASTResponse {
                output,
                format: format.to_string(),
                success: true,
                error: None,
            };
            (StatusCode::OK, Json(response))
        }
        Err(e) => {
            let response = RenderASTResponse {
                output: String::new(),
                format: format.to_string(),
                success: false,
                error: Some(format!("Invalid AST: {}", e)),
            };
            (StatusCode::BAD_REQUEST, Json(response))
        }
    }
}

// Handler for listing available operations
async fn operations_handler() -> impl IntoResponse {
    let operations = vec![
        OperationInfo {
            name: "Fraction".to_string(),
            description: "Creates a fraction a/b".to_string(),
            example_latex: "\\frac{a}{b}".to_string(),
        },
        OperationInfo {
            name: "Square Root".to_string(),
            description: "Square root of x".to_string(),
            example_latex: "\\sqrt{x}".to_string(),
        },
        OperationInfo {
            name: "Integral".to_string(),
            description: "Definite integral".to_string(),
            example_latex: "\\int_{a}^{b} f(x) \\, dx".to_string(),
        },
        OperationInfo {
            name: "Sum".to_string(),
            description: "Summation with bounds".to_string(),
            example_latex: "\\sum_{i=1}^{n} i".to_string(),
        },
        OperationInfo {
            name: "Matrix".to_string(),
            description: "2x2 matrix".to_string(),
            example_latex: "\\begin{bmatrix}a&b\\\\c&d\\end{bmatrix}".to_string(),
        },
    ];

    (StatusCode::OK, Json(operations))
}

async fn templates_handler() -> impl IntoResponse {
    let dir = std::path::Path::new("std_template_lib");
    let templates = if dir.exists() {
        match kleis::kleist_parser::load_kleist_directory(dir) {
            Ok(file) => file
                .templates
                .iter()
                .filter(|t| t.svg.is_some() || t.category.is_some())
                .map(|t| TemplateInfo {
                    name: t.name.clone(),
                    category: t.category.clone(),
                    glyph: t.glyph.clone(),
                    svg: t.svg.clone(),
                    metadata: t.metadata.clone(),
                })
                .collect(),
            Err(_) => Vec::new(),
        }
    } else {
        Vec::new()
    };
    (StatusCode::OK, Json(templates))
}

// Palette API types
#[derive(Debug, Serialize)]
struct PaletteResponse {
    tabs: Vec<PaletteTab>,
}

#[derive(Debug, Serialize)]
struct PaletteTab {
    name: String,
    items: Vec<PaletteTabItem>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
enum PaletteTabItem {
    #[serde(rename = "group")]
    Group {
        name: String,
        items: Vec<PaletteEntry>,
    },
    #[serde(rename = "separator")]
    Separator,
    #[serde(rename = "template")]
    Template(PaletteEntry),
    #[serde(rename = "tool")]
    Tool(PaletteTool),
}

#[derive(Debug, Serialize)]
struct PaletteEntry {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    glyph: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    svg: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    shortcut: Option<String>,
    ast: serde_json::Value,
    metadata: std::collections::HashMap<String, String>,
}

#[derive(Debug, Serialize)]
struct PaletteTool {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    glyph: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    svg: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    handler: Option<String>,
}

fn build_ast_from_pattern(pattern: &str) -> serde_json::Value {
    // Parse "name(arg1, arg2)" into an EditorNode AST
    if let Some(paren_pos) = pattern.find('(') {
        let name = &pattern[..paren_pos];
        let args_str = &pattern[paren_pos + 1..pattern.len() - 1];
        let args: Vec<serde_json::Value> = if args_str.is_empty() {
            vec![]
        } else {
            args_str
                .split(',')
                .enumerate()
                .map(|(i, arg)| {
                    let hint = arg.trim().to_string();
                    serde_json::json!({
                        "Placeholder": { "id": i, "hint": hint }
                    })
                })
                .collect()
        };
        serde_json::json!({
            "Operation": {
                "name": name,
                "args": args
            }
        })
    } else {
        // No parentheses = zero-arg operation
        serde_json::json!({
            "Operation": {
                "name": pattern,
                "args": []
            }
        })
    }
}

async fn palette_handler() -> impl IntoResponse {
    let dir = std::path::Path::new("std_template_lib");
    if !dir.exists() {
        return (StatusCode::OK, Json(PaletteResponse { tabs: vec![] }));
    }

    let kleist_file = match kleis::kleist_parser::load_kleist_directory(dir) {
        Ok(f) => f,
        Err(_) => {
            return (StatusCode::OK, Json(PaletteResponse { tabs: vec![] }));
        }
    };

    // Build lookup maps from templates and tools
    let template_map: std::collections::HashMap<String, &kleis::kleist_parser::TemplateDefinition> =
        kleist_file
            .templates
            .iter()
            .map(|t| (t.name.clone(), t))
            .collect();
    let tool_map: std::collections::HashMap<String, &kleis::kleist_parser::ToolDefinition> =
        kleist_file
            .tools
            .iter()
            .map(|t| (t.name.clone(), t))
            .collect();

    let palette = match kleist_file.palette {
        Some(p) => p,
        None => {
            return (StatusCode::OK, Json(PaletteResponse { tabs: vec![] }));
        }
    };

    let tabs: Vec<PaletteTab> = palette
        .tabs
        .iter()
        .map(|tab| {
            let items: Vec<PaletteTabItem> = tab
                .items
                .iter()
                .map(|item| match item {
                    kleis::kleist_parser::TabItem::Separator => PaletteTabItem::Separator,
                    kleis::kleist_parser::TabItem::Template(tref) => {
                        let entry =
                            make_palette_entry(&tref.name, tref.shortcut.as_deref(), &template_map);
                        PaletteTabItem::Template(entry)
                    }
                    kleis::kleist_parser::TabItem::Tool(tref) => {
                        let tool = tool_map.get(&tref.name);
                        PaletteTabItem::Tool(PaletteTool {
                            name: tref.name.clone(),
                            glyph: tool.and_then(|t| t.glyph.clone()),
                            svg: tool.and_then(|t| t.svg.clone()),
                            handler: tool.and_then(|t| t.handler.clone()),
                        })
                    }
                    kleis::kleist_parser::TabItem::Group(group) => {
                        let group_items: Vec<PaletteEntry> = group
                            .items
                            .iter()
                            .filter_map(|gi| match gi {
                                kleis::kleist_parser::GroupItem::Template(tref) => {
                                    Some(make_palette_entry(
                                        &tref.name,
                                        tref.shortcut.as_deref(),
                                        &template_map,
                                    ))
                                }
                                kleis::kleist_parser::GroupItem::Tool(_) => None,
                            })
                            .collect();
                        PaletteTabItem::Group {
                            name: group.name.clone(),
                            items: group_items,
                        }
                    }
                })
                .collect();

            PaletteTab {
                name: tab.name.clone(),
                items,
            }
        })
        .collect();

    (StatusCode::OK, Json(PaletteResponse { tabs }))
}

fn make_palette_entry(
    name: &str,
    shortcut: Option<&str>,
    template_map: &std::collections::HashMap<String, &kleis::kleist_parser::TemplateDefinition>,
) -> PaletteEntry {
    let template = template_map.get(name);
    let ast = template
        .and_then(|t| t.pattern.as_deref())
        .map(build_ast_from_pattern)
        .unwrap_or_else(|| build_ast_from_pattern(name));

    PaletteEntry {
        name: name.to_string(),
        glyph: template.and_then(|t| t.glyph.clone()),
        svg: template.and_then(|t| t.svg.clone()),
        shortcut: shortcut
            .map(|s| s.to_string())
            .or_else(|| template.and_then(|t| t.shortcut.clone())),
        ast,
        metadata: template.map(|t| t.metadata.clone()).unwrap_or_default(),
    }
}

// Handler for gallery examples
async fn gallery_handler() -> impl IntoResponse {
    let samples = kleis::render::collect_samples_for_gallery();

    let examples: Vec<GalleryExample> = samples
        .into_iter()
        .map(|(title, latex)| GalleryExample { title, latex })
        .collect();

    (StatusCode::OK, Json(GalleryResponse { examples }))
}

// Handler for rendering with Typst (returns SVG with placeholder positions)
async fn render_typst_handler(
    State(_state): State<Arc<AppState>>,
    Json(req): Json<RenderASTRequest>,
) -> impl IntoResponse {
    eprintln!("=== render_typst_handler called ===");
    eprintln!("Received AST JSON: {:?}", req.ast);

    match json_to_editor_node(&req.ast) {
        Ok(node) => {
            eprintln!("Parsed EditorNode: {:#?}", node);

            // Collect ALL argument slots with their info (empty or filled)
            let arg_slots = collect_argument_slots_from_editor_node(&node);
            eprintln!("Argument slots: {} total", arg_slots.len());

            for (i, slot) in arg_slots.iter().enumerate() {
                eprintln!(
                    "  Slot {}: id={}, is_placeholder={}, hint='{}'",
                    i, slot.id, slot.is_placeholder, slot.hint
                );
            }

            // Get unfilled placeholder IDs for Typst square rendering
            // Parse "ph{number}" format back to usize
            let unfilled_ids: Vec<usize> = arg_slots
                .iter()
                .filter(|s| s.is_placeholder)
                .filter_map(|s| {
                    if s.id.starts_with("ph") {
                        s.id[2..].parse::<usize>().ok()
                    } else {
                        None
                    }
                })
                .collect();

            // For all_slot_ids, we only care about placeholders (which have numeric IDs)
            // Filled slots have UUIDs which don't map to Typst placeholder positions
            let all_slot_ids = unfilled_ids.clone();

            // Build node_id -> UUID map from argument slots
            // Truncate UUIDs with collision detection and regeneration
            let mut node_id_to_uuid = std::collections::HashMap::new();
            let mut used_truncated = std::collections::HashSet::new();

            for slot in &arg_slots {
                let node_id = slot
                    .path
                    .iter()
                    .enumerate()
                    .map(|(depth, &idx)| {
                        if depth == 0 {
                            format!("{}", idx)
                        } else {
                            format!(".{}", idx)
                        }
                    })
                    .collect::<String>();
                let node_id = if node_id.is_empty() {
                    "0".to_string()
                } else {
                    format!("0.{}", node_id)
                };

                // Only process filled slots (not placeholders)
                if !slot.is_placeholder {
                    // Truncate UUID to first 8 chars
                    let mut truncated = slot.id.chars().take(8).collect::<String>();

                    // Check for collision - regenerate if needed
                    let mut attempts = 0;
                    while used_truncated.contains(&truncated) && attempts < 100 {
                        eprintln!(
                            "⚠️  UUID collision detected for {}, regenerating...",
                            truncated
                        );
                        let new_uuid = uuid::Uuid::new_v4().to_string().replace("-", "");
                        truncated = new_uuid.chars().take(8).collect::<String>();
                        attempts += 1;
                    }

                    if attempts >= 100 {
                        eprintln!(
                            "❌ Failed to generate unique 8-char UUID after 100 attempts, using full UUID"
                        );
                        truncated = uuid::Uuid::new_v4().to_string().replace("-", "");
                    }

                    used_truncated.insert(truncated.clone());
                    node_id_to_uuid.insert(node_id, truncated);
                }
            }

            eprintln!(
                "Built node_id->UUID map with {} entries",
                node_id_to_uuid.len()
            );
            for (node_id, uuid) in &node_id_to_uuid {
                eprintln!("  {} -> {}", node_id, uuid);
            }

            // Compile with Typst using semantic bounding box extraction
            // Pass both unfilled_ids (for placeholder squares) and all_slot_ids (for filled content)
            match kleis::math_layout::compile_editor_node_with_semantic_boxes(
                &node,
                &unfilled_ids,
                &all_slot_ids,
                &node_id_to_uuid,
            ) {
                Ok(output) => {
                    let response = serde_json::json!({
                        "svg": output.svg,
                        "placeholders": output.placeholder_positions.iter().map(|p| {
                            serde_json::json!({
                                "id": p.id,
                                "x": p.x,
                                "y": p.y,
                                "width": p.width,
                                "height": p.height,
                            })
                        }).collect::<Vec<_>>(),
                        "argument_bounding_boxes": output.argument_bounding_boxes.iter().map(|b| {
                            serde_json::json!({
                                "arg_index": b.arg_index,
                                "node_id": b.node_id,
                                "x": b.x,
                                "y": b.y,
                                "width": b.width,
                                "height": b.height,
                            })
                        }).collect::<Vec<_>>(),
                        "argument_slots": arg_slots,  // Return ALL slots (for frontend to make clickable)
                        "success": true,
                    });
                    (StatusCode::OK, Json(response))
                }
                Err(e) => {
                    let response = serde_json::json!({
                        "error": format!("Typst compilation failed: {}", e),
                        "success": false,
                    });
                    (StatusCode::INTERNAL_SERVER_ERROR, Json(response))
                }
            }
        }
        Err(e) => {
            let response = serde_json::json!({
                "error": format!("Invalid AST: {}", e),
                "success": false,
            });
            (StatusCode::BAD_REQUEST, Json(response))
        }
    }
}

// Argument slot info (for tracking editable regions)
#[derive(Debug, Clone, serde::Serialize)]
struct ArgumentSlot {
    id: String,           // UUID for filled values, or placeholder ID as string
    path: Vec<usize>,     // Path in AST (e.g., [0] = first arg of root operation)
    hint: String,         // Description of this slot
    is_placeholder: bool, // True if empty placeholder, false if filled
    role: Option<String>, // Semantic role (e.g., superscript, subscript, base)
}

// Collect ALL argument slots from expression (both empty and filled)
fn collect_argument_slots(expr: &kleis::ast::Expression) -> Vec<ArgumentSlot> {
    let mut slots = Vec::new();
    collect_slots_recursive(expr, &mut slots, vec![], None);
    slots
}

fn collect_slots_recursive(
    expr: &kleis::ast::Expression,
    slots: &mut Vec<ArgumentSlot>,
    path: Vec<usize>,
    role: Option<String>,
) {
    use kleis::ast::Expression;

    match expr {
        Expression::Placeholder { id, hint } => {
            // Empty placeholder - convert ID to string
            slots.push(ArgumentSlot {
                id: format!("ph{}", id), // Prefix with "ph" to distinguish from UUIDs
                path: path.clone(),
                hint: hint.clone(),
                is_placeholder: true,
                role: role.clone(),
            });
        }
        Expression::Const(value) | Expression::String(value) | Expression::Object(value) => {
            // Filled value - generate UUID without dashes
            let uuid = uuid::Uuid::new_v4().to_string().replace("-", "");
            slots.push(ArgumentSlot {
                id: uuid,
                path: path.clone(),
                hint: format!("value: {}", value),
                is_placeholder: false,
                role: role.clone(),
            });
        }
        Expression::Operation { name, args, .. } => {
            // Create a slot for the operation itself (for bounding box positioning)
            let uuid = uuid::Uuid::new_v4().to_string().replace("-", "");
            slots.push(ArgumentSlot {
                id: uuid,
                path: path.clone(),
                hint: format!("operation: {}", name),
                is_placeholder: false,
                role: role.clone(),
            });

            // For Matrix constructors with List format: skip first two args (dimensions)
            let is_matrix = matches!(name.as_str(), "Matrix" | "PMatrix" | "VMatrix" | "BMatrix");
            let has_list_format =
                is_matrix && args.len() == 3 && matches!(args.get(2), Some(Expression::List(_)));

            // For Piecewise with List format: skip first arg (n = number of cases)
            let is_piecewise = name == "Piecewise";
            let piecewise_list_format = is_piecewise
                && args.len() == 3
                && matches!(args.get(1), Some(Expression::List(_)))
                && matches!(args.get(2), Some(Expression::List(_)));

            // Recursively process each argument
            for (i, arg) in args.iter().enumerate() {
                // Skip dimension arguments for Matrix with List format
                if has_list_format && i < 2 {
                    continue;
                }

                // Skip size argument for Piecewise with List format
                if piecewise_list_format && i == 0 {
                    continue;
                }

                let mut child_path = path.clone();
                child_path.push(i);
                let child_role = determine_arg_role(name, i);
                collect_slots_recursive(arg, slots, child_path, child_role);
            }
        }
        Expression::Match { .. } => {
            // TODO: Implement match expression slot collection
            // For now, don't collect slots from match expressions
        }
        Expression::Quantifier { body, .. } => {
            // Collect slots from quantifier body
            let mut child_path = path.clone();
            child_path.push(0);
            collect_slots_recursive(body, slots, child_path, None);
        }
        Expression::List(elements) => {
            // Collect slots from list elements
            for (i, elem) in elements.iter().enumerate() {
                let mut child_path = path.clone();
                child_path.push(i);
                collect_slots_recursive(elem, slots, child_path, None);
            }
        }
        Expression::Conditional {
            condition,
            then_branch,
            else_branch,
            ..
        } => {
            // Collect slots from all three parts
            let mut cond_path = path.clone();
            cond_path.push(0);
            collect_slots_recursive(condition, slots, cond_path, Some("condition".to_string()));

            let mut then_path = path.clone();
            then_path.push(1);
            collect_slots_recursive(then_branch, slots, then_path, Some("then".to_string()));

            let mut else_path = path.clone();
            else_path.push(2);
            collect_slots_recursive(else_branch, slots, else_path, Some("else".to_string()));
        }
        Expression::Let { value, body, .. } => {
            // Collect slots from value and body
            let mut value_path = path.clone();
            value_path.push(0);
            collect_slots_recursive(value, slots, value_path, Some("value".to_string()));

            let mut body_path = path.clone();
            body_path.push(1);
            collect_slots_recursive(body, slots, body_path, Some("body".to_string()));
        }
        Expression::Ascription { expr, .. } => {
            // Collect slots from inner expression
            collect_slots_recursive(expr, slots, path, role);
        }
        Expression::Lambda { body, .. } => {
            // Collect slots from lambda body
            collect_slots_recursive(body, slots, path, role);
        }
    }
}

/// Tensor info extracted from EditorNode
struct TensorInfo {
    name: String,
    upper_count: usize, // contravariant indices
    lower_count: usize, // covariant indices
}

/// Check if the root operation is a formatting-only operation (not semantic)
/// These are valid for rendering but don't have mathematical types
fn is_formatting_operation(node: &kleis::editor_ast::EditorNode) -> Option<String> {
    use kleis::editor_ast::EditorNode;

    match node {
        EditorNode::Operation { operation } => {
            // List of formatting-only operations
            match operation.name.as_str() {
                "subsup" => Some("Display: base with sub/superscript".to_string()),
                "subscript" => Some("Display: base with subscript".to_string()),
                "superscript" | "power" => None, // power is semantic (exponentiation)
                "tilde" => Some("Display: variable with tilde".to_string()),
                "hat" => Some("Display: variable with hat".to_string()),
                "bar" => Some("Display: variable with bar".to_string()),
                "vec" => Some("Display: vector notation".to_string()),
                "dot" => Some("Display: time derivative (dot)".to_string()),
                "ddot" => Some("Display: second time derivative (double dot)".to_string()),
                _ => None,
            }
        }
        _ => None,
    }
}

// Recursively search for tensor operations in EditorNode
fn find_tensor_in_editor_node(node: &kleis::editor_ast::EditorNode) -> Option<TensorInfo> {
    use kleis::editor_ast::EditorNode;

    match node {
        EditorNode::Operation { operation } => {
            // Check if this operation is a tensor
            if operation.kind.as_deref() == Some("tensor") {
                // New structure: args[0] = symbol, args[1:] = indices
                // indexStructure describes args[1:], not args[0]
                let symbol = if !operation.args.is_empty() {
                    // Extract symbol from args[0]
                    match &operation.args[0] {
                        EditorNode::Object { object } => object.clone(),
                        EditorNode::Const { value } => value.to_string(),
                        EditorNode::Placeholder { placeholder } => {
                            placeholder.hint.clone().unwrap_or_else(|| "T".to_string())
                        }
                        _ => "T".to_string(),
                    }
                } else {
                    // Fallback to operation name for backward compatibility
                    operation.name.clone()
                };

                // Extract index structure from metadata
                let (upper, lower) = if let Some(meta) = &operation.metadata {
                    if let Some(idx_struct) = meta.get("indexStructure") {
                        if let Some(arr) = idx_struct.as_array() {
                            let up = arr.iter().filter(|v| v.as_str() == Some("up")).count();
                            let down = arr.iter().filter(|v| v.as_str() == Some("down")).count();
                            (up, down)
                        } else {
                            (0, 0)
                        }
                    } else {
                        (0, 0)
                    }
                } else {
                    // Infer from number of args (excluding symbol at args[0])
                    let num_indices = if operation.args.len() > 1 {
                        operation.args.len() - 1
                    } else {
                        0
                    };
                    (0, num_indices)
                };

                return Some(TensorInfo {
                    name: symbol,
                    upper_count: upper,
                    lower_count: lower,
                });
            }
            // Recursively check args
            for arg in &operation.args {
                if let Some(info) = find_tensor_in_editor_node(arg) {
                    return Some(info);
                }
            }
            None
        }
        EditorNode::List { list } => {
            for elem in list {
                if let Some(info) = find_tensor_in_editor_node(elem) {
                    return Some(info);
                }
            }
            None
        }
        _ => None,
    }
}

// Convert EditorNode back to Expression for type checking
fn editor_node_to_expression(
    node: &kleis::editor_ast::EditorNode,
) -> Result<kleis::ast::Expression, String> {
    use kleis::ast::Expression;
    use kleis::editor_ast::EditorNode;

    match node {
        EditorNode::Const { value } => Ok(Expression::Const(value.clone())),
        EditorNode::Object { object } => Ok(Expression::Object(object.clone())),
        EditorNode::Placeholder { placeholder } => Ok(Expression::Placeholder {
            id: placeholder.id,
            hint: placeholder.hint.clone().unwrap_or_else(|| "□".to_string()),
        }),
        EditorNode::Operation { operation } => {
            // Tensor operations are formatting-only: T^i_j is just T for verification
            // The indices are display metadata, not mathematical operations
            if operation.name == "tensor" || operation.kind.as_deref() == Some("tensor") {
                // args[0] is the base symbol, args[1:] are indices (ignored for verification)
                if let Some(base) = operation.args.first() {
                    return editor_node_to_expression(base);
                }
            }

            let args: Result<Vec<Expression>, String> = operation
                .args
                .iter()
                .map(editor_node_to_expression)
                .collect();
            Ok(Expression::Operation {
                name: operation.name.clone(),
                args: args?,
                span: None,
            })
        }
        EditorNode::List { list } => {
            let elements: Result<Vec<Expression>, String> =
                list.iter().map(editor_node_to_expression).collect();
            Ok(Expression::List(elements?))
        }
    }
}

fn determine_arg_role(op_name: &str, arg_index: usize) -> Option<String> {
    match op_name {
        "sup" | "power" => match arg_index {
            0 => Some("base".to_string()),
            1 => Some("superscript".to_string()),
            _ => None,
        },
        "sub" => match arg_index {
            0 => Some("base".to_string()),
            1 => Some("subscript".to_string()),
            _ => None,
        },
        "index" | "index_mixed" | "tensor_mixed" => match arg_index {
            0 => Some("base".to_string()),
            1 => Some("superscript".to_string()),
            2 => Some("subscript".to_string()),
            _ => None,
        },
        _ => None,
    }
}

// Collect ALL argument slots from EditorNode (both empty and filled)
fn collect_argument_slots_from_editor_node(
    node: &kleis::editor_ast::EditorNode,
) -> Vec<ArgumentSlot> {
    let mut slots = Vec::new();
    collect_editor_slots_recursive(node, &mut slots, vec![], None);
    slots
}

fn collect_editor_slots_recursive(
    node: &kleis::editor_ast::EditorNode,
    slots: &mut Vec<ArgumentSlot>,
    path: Vec<usize>,
    role: Option<String>,
) {
    use kleis::editor_ast::EditorNode;

    match node {
        EditorNode::Placeholder { placeholder } => {
            slots.push(ArgumentSlot {
                id: format!("ph{}", placeholder.id),
                path: path.clone(),
                hint: placeholder.hint.clone().unwrap_or_else(|| "□".to_string()),
                is_placeholder: true,
                role: role.clone(),
            });
        }
        EditorNode::Const { value } => {
            let uuid = uuid::Uuid::new_v4().to_string().replace("-", "");
            slots.push(ArgumentSlot {
                id: uuid,
                path: path.clone(),
                hint: format!("value: {}", value),
                is_placeholder: false,
                role: role.clone(),
            });
        }
        EditorNode::Object { object } => {
            let uuid = uuid::Uuid::new_v4().to_string().replace("-", "");
            slots.push(ArgumentSlot {
                id: uuid,
                path: path.clone(),
                hint: format!("value: {}", object),
                is_placeholder: false,
                role: role.clone(),
            });
        }
        EditorNode::Operation { operation } => {
            if operation.args.is_empty() {
                // Zero-arg operation is a leaf (e.g., glyph, symbol template).
                // Treat like Object/Const: give it a UUID for bounding box tracking.
                let uuid = uuid::Uuid::new_v4().to_string().replace("-", "");
                slots.push(ArgumentSlot {
                    id: uuid,
                    path: path.clone(),
                    hint: format!("value: {}", operation.name),
                    is_placeholder: false,
                    role: role.clone(),
                });
            } else {
                // Multi-arg operation: recurse into children
                for (i, arg) in operation.args.iter().enumerate() {
                    // Skip structural arguments that are not user-editable content:
                    // - Matrix: first two args are dimensions (rows, cols)
                    // - Piecewise: first arg is case count
                    let is_structural = match operation.name.as_str() {
                        "Matrix" => i < 2,
                        "Piecewise" => i == 0,
                        _ => false,
                    };
                    if is_structural {
                        continue;
                    }

                    let mut child_path = path.clone();
                    child_path.push(i);
                    let child_role = determine_arg_role(&operation.name, i);
                    collect_editor_slots_recursive(arg, slots, child_path, child_role);
                }
            }
        }
        EditorNode::List { list } => {
            for (i, elem) in list.iter().enumerate() {
                let mut child_path = path.clone();
                child_path.push(i);
                collect_editor_slots_recursive(elem, slots, child_path, None);
            }
        }
    }
}

// Health check endpoint
// Handler for type checking expressions
async fn type_check_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<TypeCheckRequest>,
) -> impl IntoResponse {
    // Try to get type checker from state
    let type_checker_guard = state.type_checker.lock().unwrap();

    if type_checker_guard.is_none() {
        return Json(TypeCheckResponse {
            success: false,
            type_name: None,
            error: Some(
                "Type checker not initialized (stdlib/matrices.kleis not loaded)".to_string(),
            ),
            suggestion: None,
        });
    }

    // Parse AST from JSON - try EditorNode first
    let editor_node = match json_to_editor_node(&req.ast) {
        Ok(n) => n,
        Err(e) => {
            return Json(TypeCheckResponse {
                success: false,
                type_name: None,
                error: Some(format!("Failed to parse AST: {}", e)),
                suggestion: None,
            });
        }
    };

    // Check if this is a formatting-only operation (display, not semantic)
    // These are valid for rendering but have no mathematical type
    if let Some(format_type) = is_formatting_operation(&editor_node) {
        return Json(TypeCheckResponse {
            success: true,
            type_name: Some(format_type),
            error: None,
            suggestion: Some(
                "💡 This is a formatting operation - type depends on content.".to_string(),
            ),
        });
    }

    // Check if this is a tensor operation - handle specially since type checker
    // doesn't yet support Unicode operation names like Γ, R, g
    if let Some(info) = find_tensor_in_editor_node(&editor_node) {
        // Known tensors get friendly descriptions
        let tensor_type = match info.name.as_str() {
            "Γ" => format!(
                "Tensor({}, {}, dim, ℝ) — Christoffel symbol Γ^λ_μν",
                info.upper_count, info.lower_count
            ),
            "R" => format!(
                "Tensor({}, {}, dim, ℝ) — Riemann tensor R^ρ_σμν",
                info.upper_count, info.lower_count
            ),
            "g" => format!(
                "Tensor({}, {}, dim, ℝ) — Metric tensor g_μν",
                info.upper_count, info.lower_count
            ),
            // Generic tensors: infer type from index structure
            _ => {
                let index_notation = format!(
                    "{}{}",
                    if info.upper_count > 0 {
                        format!("^{{{}}}", "μ".repeat(info.upper_count))
                    } else {
                        String::new()
                    },
                    if info.lower_count > 0 {
                        format!("_{{{}}}", "ν".repeat(info.lower_count))
                    } else {
                        String::new()
                    }
                );
                format!(
                    "Tensor({}, {}, dim, ℝ) — {}{}",
                    info.upper_count, info.lower_count, info.name, index_notation
                )
            }
        };
        return Json(TypeCheckResponse {
            success: true,
            type_name: Some(tensor_type),
            error: None,
            suggestion: None,
        });
    }

    // Convert EditorNode to Expression for type checking
    let expr = match editor_node_to_expression(&editor_node) {
        Ok(e) => e,
        Err(e) => {
            return Json(TypeCheckResponse {
                success: false,
                type_name: None,
                error: Some(format!("Failed to convert to Expression: {}", e)),
                suggestion: None,
            });
        }
    };

    // Need to clone the type checker since check() takes &mut self
    // For now, create a new one each time (TODO: make check() use &self)
    drop(type_checker_guard);

    // Re-create type checker with full stdlib (minimal_prelude + matrices)
    let result = match kleis::type_checker::TypeChecker::with_stdlib() {
        Ok(mut checker) => checker.check(&expr),
        Err(e) => {
            return Json(TypeCheckResponse {
                success: false,
                type_name: None,
                error: Some(format!("Failed to initialize TypeChecker: {}", e)),
                suggestion: None,
            });
        }
    };

    // Convert TypeCheckResult to response
    match result {
        kleis::type_checker::TypeCheckResult::Success(ty) => Json(TypeCheckResponse {
            success: true,
            type_name: Some(format!("{:?}", ty)),
            error: None,
            suggestion: None,
        }),
        kleis::type_checker::TypeCheckResult::Error {
            message,
            suggestion,
        } => Json(TypeCheckResponse {
            success: false,
            type_name: None,
            error: Some(message),
            suggestion,
        }),
        kleis::type_checker::TypeCheckResult::Polymorphic { type_var, .. } => {
            Json(TypeCheckResponse {
                success: true,
                type_name: Some(format!("Polymorphic({:?})", type_var)),
                error: None,
                suggestion: Some("Type is polymorphic - needs more context".to_string()),
            })
        }
    }
}

// Render AST to Kleis syntax
async fn render_kleis_handler(
    State(_state): State<Arc<AppState>>,
    Json(req): Json<RenderKleisRequest>,
) -> impl IntoResponse {
    // Parse AST from JSON
    let expr = match json_to_expression(&req.ast) {
        Ok(e) => e,
        Err(e) => {
            return Json(RenderKleisResponse {
                kleis: String::new(),
                success: false,
                error: Some(format!("Failed to parse AST: {}", e)),
            });
        }
    };

    // Render to Kleis syntax
    let ctx = kleis::render::build_default_context();
    let kleis_output =
        kleis::render::render_expression(&expr, &ctx, &kleis::render::RenderTarget::Kleis);

    Json(RenderKleisResponse {
        kleis: kleis_output,
        success: true,
        error: None,
    })
}

// Export AST to Typst code (for PhD candidates to copy/paste into thesis documents)
async fn export_typst_handler(
    State(_state): State<Arc<AppState>>,
    Json(req): Json<ExportTypstRequest>,
) -> impl IntoResponse {
    // Parse AST from JSON as EditorNode
    let node = match json_to_editor_node(&req.ast) {
        Ok(n) => n,
        Err(e) => {
            return Json(ExportTypstResponse {
                typst: String::new(),
                success: false,
                error: Some(format!("Failed to parse AST: {}", e)),
            });
        }
    };

    // Render to Typst syntax using render_editor module
    let typst_output =
        kleis::render_editor::render_editor_node(&node, &kleis::render::RenderTarget::Typst);

    Json(ExportTypstResponse {
        typst: typst_output,
        success: true,
        error: None,
    })
}

// Verify AST with Z3
async fn verify_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<VerifyRequest>,
) -> impl IntoResponse {
    use kleis::editor_type_translator::EditorTypeTranslator;
    use kleis::solvers::backend::SolverBackend;
    use kleis::type_checker::TypeCheckResult;

    // Parse as EditorNode first to preserve type metadata (kind, metadata fields)
    let editor_node = match json_to_editor_node(&req.ast) {
        Ok(n) => n,
        Err(e) => {
            return Json(VerifyResponse {
                success: false,
                result: "error".to_string(),
                kleis_syntax: String::new(),
                counterexample: None,
                error: Some(format!("Failed to parse AST: {}", e)),
            });
        }
    };

    // Extract type information from EditorNode metadata BEFORE converting to Expression
    // This preserves the rich type info (tensor indices, matrix dimensions, etc.)
    let mut inferred_types = std::collections::HashMap::new();
    extract_types_from_editor_node(&editor_node, &mut inferred_types, "root");

    // Convert EditorNode to Expression for verification
    let expr = match editor_node_to_expression(&editor_node) {
        Ok(e) => e,
        Err(e) => {
            return Json(VerifyResponse {
                success: false,
                result: "error".to_string(),
                kleis_syntax: String::new(),
                counterexample: None,
                error: Some(format!("Failed to convert to Expression: {}", e)),
            });
        }
    };

    // Check for unfilled placeholders before verification
    let placeholders = expr.find_placeholders();
    if !placeholders.is_empty() {
        let placeholder_hints: Vec<String> = placeholders
            .iter()
            .map(|(_, hint)| format!("\"{}\"", hint))
            .collect();
        return Json(VerifyResponse {
            success: false,
            result: "incomplete".to_string(),
            kleis_syntax: String::new(),
            counterexample: None,
            error: Some(format!(
                "Please fill in all placeholders before verifying. Unfilled: {}",
                placeholder_hints.join(", ")
            )),
        });
    }

    // Render to Kleis syntax first
    let ctx = kleis::render::build_default_context();
    let kleis_syntax =
        kleis::render::render_expression(&expr, &ctx, &kleis::render::RenderTarget::Kleis);

    // Also run Kleis type checker to supplement EditorNode types
    // This handles operations where EditorNode doesn't have explicit metadata
    {
        let mut type_checker_guard = state.type_checker.lock().unwrap();
        if let Some(ref mut type_checker) = *type_checker_guard {
            // Type check the expression
            if let TypeCheckResult::Success(expr_type) = type_checker.check(&expr) {
                // Only add if not already inferred from EditorNode
                inferred_types
                    .entry("root".to_string())
                    .or_insert(expr_type);
            }
        }
    }

    // Use AxiomVerifier (same code path as `kleis test`) for Z3 verification.
    // This ensures axioms + identity elements are loaded consistently.
    let registry = &*state.registry;
    let mut verifier = match kleis::axiom_verifier::AxiomVerifier::new(registry) {
        Ok(v) => v,
        Err(e) => {
            return Json(VerifyResponse {
                success: false,
                result: "error".to_string(),
                kleis_syntax,
                counterexample: None,
                error: Some(format!("Failed to create verifier: {}", e)),
            });
        }
    };

    match verifier.verify_axiom(&expr) {
        Ok(result) => {
            use kleis::axiom_verifier::VerificationResult;
            let (result_str, counterexample) = match result {
                VerificationResult::Valid => ("valid".to_string(), None),
                VerificationResult::ValidWithWitness { witness } => {
                    ("valid".to_string(), Some(format!("Witness: {}", witness)))
                }
                VerificationResult::Invalid { witness } => {
                    ("invalid".to_string(), Some(witness.to_string()))
                }
                VerificationResult::Unknown => ("unknown".to_string(), None),
                VerificationResult::InconsistentAxioms => (
                    "error".to_string(),
                    Some("Axioms are inconsistent".to_string()),
                ),
                VerificationResult::Disabled => (
                    "error".to_string(),
                    Some("Axiom verification disabled".to_string()),
                ),
            };
            Json(VerifyResponse {
                success: true,
                result: result_str,
                kleis_syntax,
                counterexample,
                error: None,
            })
        }
        Err(e) => Json(VerifyResponse {
            success: false,
            result: "error".to_string(),
            kleis_syntax,
            counterexample: None,
            error: Some(format!("Verification error: {}", e)),
        }),
    }
}

/// Recursively extract types from EditorNode tree
/// This captures the rich type information from editor metadata (kind, dimensions, etc.)
fn extract_types_from_editor_node(
    node: &kleis::editor_ast::EditorNode,
    types: &mut std::collections::HashMap<String, kleis::type_inference::Type>,
    path: &str,
) {
    use kleis::editor_ast::EditorNode;
    use kleis::editor_type_translator::EditorTypeTranslator;

    // Try to extract type from this node
    if let Some(ty) = EditorTypeTranslator::translate(node) {
        types.insert(path.to_string(), ty);
    }

    // Recurse into children
    match node {
        EditorNode::Operation { operation } => {
            for (i, arg) in operation.args.iter().enumerate() {
                let child_path = format!("{}.arg{}", path, i);
                extract_types_from_editor_node(arg, types, &child_path);
            }
        }
        EditorNode::List { list } => {
            for (i, elem) in list.iter().enumerate() {
                let child_path = format!("{}.elem{}", path, i);
                extract_types_from_editor_node(elem, types, &child_path);
            }
        }
        _ => {} // Const, Object, Placeholder have no children
    }
}

// Check satisfiability with Z3 (existence check: "Can this be true?")
async fn check_sat_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<VerifyRequest>,
) -> impl IntoResponse {
    use kleis::solvers::backend::SolverBackend;

    // Parse as EditorNode first to preserve type metadata (kind, metadata fields)
    let editor_node = match json_to_editor_node(&req.ast) {
        Ok(n) => n,
        Err(e) => {
            return Json(CheckSatResponse {
                success: false,
                result: "error".to_string(),
                kleis_syntax: String::new(),
                example: None,
                error: Some(format!("Failed to parse AST: {}", e)),
            });
        }
    };

    // Extract type information from EditorNode metadata BEFORE converting to Expression
    let mut inferred_types = std::collections::HashMap::new();
    extract_types_from_editor_node(&editor_node, &mut inferred_types, "root");

    // Convert EditorNode to Expression for satisfiability check
    let expr = match editor_node_to_expression(&editor_node) {
        Ok(e) => e,
        Err(e) => {
            return Json(CheckSatResponse {
                success: false,
                result: "error".to_string(),
                kleis_syntax: String::new(),
                example: None,
                error: Some(format!("Failed to convert to Expression: {}", e)),
            });
        }
    };

    // Check for unfilled placeholders before checking satisfiability
    let placeholders = expr.find_placeholders();
    if !placeholders.is_empty() {
        let placeholder_hints: Vec<String> = placeholders
            .iter()
            .map(|(_, hint)| format!("\"{}\"", hint))
            .collect();
        return Json(CheckSatResponse {
            success: false,
            result: "incomplete".to_string(),
            kleis_syntax: String::new(),
            example: None,
            error: Some(format!(
                "Please fill in all placeholders first. Unfilled: {}",
                placeholder_hints.join(", ")
            )),
        });
    }

    // Render to Kleis syntax first
    let ctx = kleis::render::build_default_context();
    let kleis_syntax =
        kleis::render::render_expression(&expr, &ctx, &kleis::render::RenderTarget::Kleis);

    // Reduce concrete sub-expressions (e.g. multiply(ones, ones) → Matrix)
    // but preserve the top-level equality for Z3 to solve.
    let expr = if let kleis::ast::Expression::Operation { name, args, span } = &expr {
        if (name == "equals" || name == "eq") && args.len() == 2 {
            let evaluator = kleis::evaluator::Evaluator::new();
            let lhs = evaluator
                .eval_concrete(&args[0])
                .unwrap_or_else(|_| args[0].clone());
            let rhs = evaluator
                .eval_concrete(&args[1])
                .unwrap_or_else(|_| args[1].clone());
            kleis::ast::Expression::Operation {
                name: name.clone(),
                args: vec![lhs, rhs],
                span: span.clone(),
            }
        } else {
            expr
        }
    } else {
        expr
    };

    let registry = &*state.registry;
    let mut backend = match kleis::solvers::Z3Backend::new(registry) {
        Ok(b) => b,
        Err(e) => {
            return Json(CheckSatResponse {
                success: false,
                result: "error".to_string(),
                kleis_syntax,
                example: None,
                error: Some(format!("Failed to create solver backend: {}", e)),
            });
        }
    };

    match backend.check_satisfiability(&expr) {
        Ok(result) => {
            use kleis::solvers::backend::SatisfiabilityResult;
            let (result_str, example) = match result {
                SatisfiabilityResult::Satisfiable { witness } => {
                    ("satisfiable".to_string(), Some(witness.to_string()))
                }
                SatisfiabilityResult::Unsatisfiable => ("unsatisfiable".to_string(), None),
                SatisfiabilityResult::Unknown => ("unknown".to_string(), None),
            };
            Json(CheckSatResponse {
                success: true,
                result: result_str,
                kleis_syntax,
                example,
                error: None,
            })
        }
        Err(e) => Json(CheckSatResponse {
            success: false,
            result: "error".to_string(),
            kleis_syntax,
            example: None,
            error: Some(format!("Satisfiability check error: {}", e)),
        }),
    }
}

async fn health_handler() -> &'static str {
    "OK"
}

fn verify_graph_core(req: VerifyGraphRequest) -> VerifyGraphResponse {
    use kleis::evaluator::Evaluator;
    use kleis::kleis_parser::parse_kleis_program_with_file;

    let theory_path =
        std::path::PathBuf::from("std_template_lib").join(format!("{}.kleis", req.domain));

    if !theory_path.exists() {
        return VerifyGraphResponse {
            success: false,
            results: vec![],
            error: Some(format!(
                "No companion theory file: {}",
                theory_path.display()
            )),
            preamble: None,
        };
    }

    let theory_source = match std::fs::read_to_string(&theory_path) {
        Ok(s) => s,
        Err(e) => {
            return VerifyGraphResponse {
                success: false,
                results: vec![],
                error: Some(format!("Failed to read theory: {}", e)),
                preamble: None,
            };
        }
    };

    let preamble = build_graph_preamble(&req, &theory_source);

    let full_source = format!("{}{}", preamble, theory_source);
    let preamble_copy = preamble.clone();

    let program = match parse_kleis_program_with_file(
        &full_source,
        theory_path.to_string_lossy().to_string(),
    ) {
        Ok(p) => p,
        Err(e) => {
            return VerifyGraphResponse {
                success: false,
                results: vec![],
                error: Some(format!("Parse error: {}", e)),
                preamble: Some(preamble_copy),
            };
        }
    };

    let mut evaluator = Evaluator::new();
    evaluator.load_program(&program);
    let example_results = evaluator.run_all_examples(&program);

    let results: Vec<VerifyGraphExampleResult> = example_results
        .iter()
        .map(|r| VerifyGraphExampleResult {
            name: r.name.clone(),
            passed: r.passed,
            error: r.error.clone(),
        })
        .collect();

    let all_passed = results.iter().all(|r| r.passed);
    VerifyGraphResponse {
        success: all_passed,
        results,
        error: None,
        preamble: Some(preamble_copy),
    }
}

// ---------------------------------------------------------------------------
// Domain-agnostic graph preamble generator
// ---------------------------------------------------------------------------
// The server knows NOTHING about Petri nets, bond graphs, or any domain.
// It emits generic graph data; the companion .kleis theory interprets it.
// ---------------------------------------------------------------------------

fn build_graph_preamble(req: &VerifyGraphRequest, theory_source: &str) -> String {
    let nc = req.components.len();
    let nn = req.incidence.v;

    // Assign stable integer codes to each unique component_type string
    let mut type_map: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    let mut next_code: usize = 1;
    for comp in &req.components {
        let ct = comp
            .component_type
            .as_deref()
            .unwrap_or("Unknown")
            .to_string();
        if !type_map.contains_key(&ct) {
            type_map.insert(ct, next_code);
            next_code += 1;
        }
    }

    // Parse theory to discover declared TYPE_X operations.
    // Theories explicitly declare their required type codes as:
    //   operation TYPE_Foo : ℤ
    // This replaces the old text-scanning approach with AST-based extraction.
    let mut theory_declared_types: std::collections::BTreeSet<String> =
        std::collections::BTreeSet::new();
    if let Ok(theory_program) = kleis::kleis_parser::parse_kleis_program(theory_source) {
        for op in theory_program.operations() {
            if let Some(suffix) = op.name.strip_prefix("TYPE_") {
                if !suffix.is_empty() {
                    theory_declared_types.insert(suffix.to_string());
                    if !type_map.contains_key(suffix) {
                        type_map.insert(suffix.to_string(), next_code);
                        next_code += 1;
                    }
                }
            }
        }
    }

    // Build component-level incidence matrix from port-level entries
    let port_to_comp = |port_idx: usize| -> Option<usize> {
        req.port_labels
            .get(port_idx)
            .and_then(|label| label.split(':').next())
            .and_then(|ci_str| ci_str.parse::<usize>().ok())
    };

    // inc[net][comp] = signed value (aggregated across ports)
    let mut inc: std::collections::BTreeMap<(usize, usize), i64> =
        std::collections::BTreeMap::new();
    for entry in &req.incidence.entries {
        if let Some(ci) = port_to_comp(entry.port) {
            *inc.entry((entry.net, ci)).or_insert(0) += entry.value;
        }
    }

    let mut preamble = String::new();

    // -- Operation declarations
    preamble.push_str("// Graph primitives — auto-generated, domain-agnostic\n");
    preamble.push_str("operation graph_nc : ℤ\n");
    preamble.push_str("operation graph_nn : ℤ\n");
    preamble.push_str("operation graph_ctype : ℤ → ℤ\n");
    preamble.push_str("operation graph_param : ℤ × ℤ → ℝ\n");
    preamble.push_str("operation graph_inc : ℤ × ℤ → ℝ\n");
    preamble.push_str("\n");

    // Emit operation declarations only for TYPE_X not declared by the theory.
    // The theory declares its own TYPE_X operations; the preamble fills in any
    // extras that appear in the request's component_type values.
    let undeclared: Vec<_> = type_map
        .keys()
        .filter(|name| !theory_declared_types.contains(name.as_str()))
        .cloned()
        .collect();
    if !undeclared.is_empty() {
        preamble.push_str("// Extra type codes from request (not declared in theory)\n");
        for name in &undeclared {
            let safe_name = name.replace(' ', "_");
            preamble.push_str(&format!("operation TYPE_{safe_name} : ℤ\n"));
        }
    }
    preamble.push_str("\n");

    // -- GraphData structure
    preamble.push_str("structure GraphData {\n");
    preamble.push_str(&format!("    axiom nc_val: graph_nc = {nc}\n"));
    preamble.push_str(&format!("    axiom nn_val: graph_nn = {nn}\n"));

    // Type code values
    for (name, &code) in &type_map {
        let safe_name = name.replace(' ', "_");
        preamble.push_str(&format!(
            "    axiom type_{safe_name}: TYPE_{safe_name} = {code}\n"
        ));
    }

    // Component types
    for (ci, comp) in req.components.iter().enumerate() {
        let ct = comp
            .component_type
            .as_deref()
            .unwrap_or("Unknown")
            .to_string();
        let code = type_map.get(&ct).copied().unwrap_or(0);
        preamble.push_str(&format!(
            "    axiom ctype_{ci}: graph_ctype({ci}) = {code}\n"
        ));
    }

    // Closed-world: components outside range have type 0
    preamble.push_str(&format!(
        "    axiom ctype_closed: ∀(c : ℤ). (c < 0 ∨ c ≥ {nc}) → graph_ctype(c) = 0\n"
    ));

    // Distinctness: all TYPE codes are different and > 0.
    // Prevents Z3 from equating an undefined TYPE_X with a defined one.
    let codes: Vec<(&String, &usize)> = type_map.iter().collect();
    for i in 0..codes.len() {
        for j in (i + 1)..codes.len() {
            let a = codes[i].0.replace(' ', "_");
            let b = codes[j].0.replace(' ', "_");
            preamble.push_str(&format!(
                "    axiom distinct_{a}_{b}: ¬(TYPE_{a} = TYPE_{b})\n"
            ));
        }
    }
    // All type codes are positive (0 = "no such type")
    for (name, _) in &type_map {
        let safe_name = name.replace(' ', "_");
        preamble.push_str(&format!(
            "    axiom pos_{safe_name}: TYPE_{safe_name} > 0\n"
        ));
    }

    // Component-level incidence matrix — enumerate ALL (net, comp) pairs
    // explicitly including zeros, so Z3 doesn't need quantifier instantiation.
    for net in 0..nn {
        for comp in 0..nc {
            let val = inc.get(&(net, comp)).copied().unwrap_or(0);
            preamble.push_str(&format!(
                "    axiom inc_{net}_{comp}: graph_inc({net}, {comp}) = {val}.0\n"
            ));
        }
    }

    // Component parameters — enumerate ALL (comp, param) pairs explicitly
    let max_params = req
        .components
        .iter()
        .map(|c| c.params.as_ref().map_or(0, |p| p.len()))
        .max()
        .unwrap_or(0);
    for ci in 0..nc {
        let param_count = req.components[ci].params.as_ref().map_or(0, |p| p.len());
        let max_p = std::cmp::max(param_count, max_params);
        if let Some(ref params) = req.components[ci].params {
            let mut sorted_keys: Vec<&String> = params.keys().collect();
            sorted_keys.sort();
            for pi in 0..max_p {
                let v = if pi < sorted_keys.len() {
                    params[sorted_keys[pi]].as_f64().unwrap_or(0.0)
                } else {
                    0.0
                };
                let v_str = if v.fract() == 0.0 && v.abs() < 1e15 {
                    format!("{}", v as i64)
                } else {
                    format!("{}", v)
                };
                preamble.push_str(&format!(
                    "    axiom param_{ci}_{pi}: graph_param({ci}, {pi}) = {v_str}\n"
                ));
            }
        } else {
            for pi in 0..max_p {
                preamble.push_str(&format!(
                    "    axiom param_{ci}_{pi}: graph_param({ci}, {pi}) = 0\n"
                ));
            }
        }
    }

    preamble.push_str("}\n\n");
    preamble
}

async fn verify_graph_handler(
    State(_state): State<Arc<AppState>>,
    Json(req): Json<VerifyGraphRequest>,
) -> impl IntoResponse {
    let result = tokio::task::spawn_blocking(move || verify_graph_core(req)).await;
    match result {
        Ok(resp) => Json(resp),
        Err(e) => Json(VerifyGraphResponse {
            success: false,
            results: vec![],
            error: Some(format!("Task panic: {}", e)),
            preamble: None,
        }),
    }
}

// =============================================================================
// Graph simulation setup — domain-agnostic two-phase protocol
// =============================================================================

fn simulate_setup_core(req: SimulateSetupRequest) -> SimulateSetupResponse {
    use kleis::ast::Expression;
    use kleis::evaluator::Evaluator;
    use kleis::kleis_parser::parse_kleis_program_with_file;

    let theory_path =
        std::path::PathBuf::from("std_template_lib").join(format!("{}.kleis", req.domain));

    if !theory_path.exists() {
        return SimulateSetupResponse {
            sim_mode: "unknown".into(),
            a_matrix: None,
            b_matrix: None,
            state_map: vec![],
            input_map: vec![],
            initial_state: vec![],
            input_values: vec![],
            dt: 0.0,
            tau_min: None,
            chunk_size: None,
            error: Some(format!("No theory file: {}", theory_path.display())),
        };
    }

    let theory_source = match std::fs::read_to_string(&theory_path) {
        Ok(s) => s,
        Err(e) => {
            return SimulateSetupResponse {
                sim_mode: "unknown".into(),
                a_matrix: None,
                b_matrix: None,
                state_map: vec![],
                input_map: vec![],
                initial_state: vec![],
                input_values: vec![],
                dt: 0.0,
                tau_min: None,
                chunk_size: None,
                error: Some(format!("Failed to read theory: {}", e)),
            };
        }
    };

    // Detect sim_mode from the .kleist domain config
    let kleist_path =
        std::path::PathBuf::from("std_template_lib").join(format!("{}.kleist", req.domain));
    let sim_mode = if let Ok(kleist_src) = std::fs::read_to_string(&kleist_path) {
        if kleist_src.contains("sim_mode: \"continuous\"") {
            "continuous".to_string()
        } else if kleist_src.contains("sim_mode: \"discrete\"") {
            "discrete".to_string()
        } else {
            "discrete".to_string()
        }
    } else {
        "discrete".to_string()
    };

    if sim_mode == "discrete" {
        return SimulateSetupResponse {
            sim_mode: "discrete".into(),
            a_matrix: None,
            b_matrix: None,
            state_map: vec![],
            input_map: vec![],
            initial_state: vec![],
            input_values: vec![],
            dt: 0.0,
            tau_min: None,
            chunk_size: None,
            error: None,
        };
    }

    // Detect dt from .kleist
    let dt: f64 = if let Ok(kleist_src) = std::fs::read_to_string(&kleist_path) {
        kleist_src
            .lines()
            .find(|l| l.contains("sim_dt:"))
            .and_then(|l| l.split('"').nth(1).and_then(|v| v.parse::<f64>().ok()))
            .unwrap_or(0.0001)
    } else {
        0.0001
    };

    // --- Pass 1: eval_concrete for dimensions and mappings ---
    // Build sim preamble (concrete defines) + theory for eval_concrete
    let verify_req = VerifyGraphRequest {
        domain: req.domain.clone(),
        components: req.components.clone(),
        incidence: req.incidence.clone(),
        port_labels: req.port_labels.clone(),
    };

    // We need a SimulateGraphRequest to build the sim preamble. Create a dummy
    // with empty state — we only need the topology for counting.
    let dummy_state: Vec<f64> = req.components.iter().map(|_| 0.0).collect();
    let sim_req = SimulateGraphRequest {
        domain: req.domain.clone(),
        components: req.components.clone(),
        incidence: req.incidence.clone(),
        port_labels: req.port_labels.clone(),
        state: dummy_state,
        action: SimulateAction::FindEnabled,
        last_fired: None,
        sim_mode: None,
        a_matrix: None,
        b_matrix: None,
        inputs: None,
        dt: None,
        chunk_size: None,
    };

    let sim_preamble = build_sim_preamble(&sim_req, &sim_req.state);
    let pass1_source = format!("{}{}", sim_preamble, theory_source);

    let pass1_program = match parse_kleis_program_with_file(
        &pass1_source,
        theory_path.to_string_lossy().to_string(),
    ) {
        Ok(p) => p,
        Err(e) => {
            return SimulateSetupResponse {
                sim_mode,
                a_matrix: None,
                b_matrix: None,
                state_map: vec![],
                input_map: vec![],
                initial_state: vec![],
                input_values: vec![],
                dt,
                tau_min: None,
                chunk_size: None,
                error: Some(format!("Pass 1 parse error: {}", e)),
            };
        }
    };

    let mut eval1 = Evaluator::new();
    eval1.load_program(&pass1_program);

    // Helper to evaluate a Kleis expression string to a number via eval_concrete
    let eval_expr = |evaluator: &Evaluator, expr_str: &str| -> Result<f64, String> {
        let expr = kleis::kleis_parser::parse_kleis(expr_str)
            .map_err(|e| format!("Parse '{}': {}", expr_str, e))?;
        let result = evaluator.eval_concrete(&expr)?;
        match &result {
            Expression::Const(s) => s.parse::<f64>().map_err(|_| format!("Not a number: {}", s)),
            _ => Err(format!("Unexpected result: {:?}", result)),
        }
    };

    // Helper to evaluate a Kleis expression string to a string via eval_concrete
    let eval_str = |evaluator: &Evaluator, expr_str: &str| -> Result<String, String> {
        let expr = kleis::kleis_parser::parse_kleis(expr_str)
            .map_err(|e| format!("Parse '{}': {}", expr_str, e))?;
        let result = evaluator.eval_concrete(&expr)?;
        match result {
            Expression::String(s) => Ok(s),
            Expression::Const(s) => Ok(s),
            other => Err(format!(
                "Expected string from '{}', got: {:?}",
                expr_str, other
            )),
        }
    };

    let ns = match eval_expr(&eval1, "sim_state_count") {
        Ok(v) => v as usize,
        Err(e) => {
            return SimulateSetupResponse {
                sim_mode,
                a_matrix: None,
                b_matrix: None,
                state_map: vec![],
                input_map: vec![],
                initial_state: vec![],
                input_values: vec![],
                dt,
                tau_min: None,
                chunk_size: None,
                error: Some(format!("sim_state_count: {}", e)),
            };
        }
    };

    let ni = match eval_expr(&eval1, "sim_input_count") {
        Ok(v) => v as usize,
        Err(e) => {
            return SimulateSetupResponse {
                sim_mode,
                a_matrix: None,
                b_matrix: None,
                state_map: vec![],
                input_map: vec![],
                initial_state: vec![],
                input_values: vec![],
                dt,
                tau_min: None,
                chunk_size: None,
                error: Some(format!("sim_input_count: {}", e)),
            };
        }
    };

    if ns == 0 {
        return SimulateSetupResponse {
            sim_mode,
            a_matrix: None,
            b_matrix: None,
            state_map: vec![],
            input_map: vec![],
            initial_state: vec![],
            input_values: vec![],
            dt,
            tau_min: None,
            chunk_size: None,
            error: Some("No state variables (no C or I elements)".into()),
        };
    }

    // Evaluate mappings, initial state, and input values
    let mut state_map = Vec::with_capacity(ns);
    let mut state_nets = Vec::with_capacity(ns);
    let mut initial_state = Vec::with_capacity(ns);
    let mut input_map = Vec::with_capacity(ni);
    let mut input_nets = Vec::with_capacity(ni);
    let mut input_values = Vec::with_capacity(ni);

    for i in 0..ns {
        let comp = eval_expr(&eval1, &format!("sim_state_map({})", i)).unwrap_or(-1.0) as usize;
        let net =
            eval_expr(&eval1, &format!("sim_connected_net({})", comp)).unwrap_or(-1.0) as usize;
        let init = eval_expr(&eval1, &format!("sim_initial_state({})", i)).unwrap_or(0.0);
        state_map.push(comp);
        state_nets.push(net);
        initial_state.push(init);
    }

    for k in 0..ni {
        let comp = eval_expr(&eval1, &format!("sim_input_map({})", k)).unwrap_or(-1.0) as usize;
        let net =
            eval_expr(&eval1, &format!("sim_connected_net({})", comp)).unwrap_or(-1.0) as usize;
        let val = eval_expr(&eval1, &format!("sim_input_value({})", k)).unwrap_or(0.0);
        input_map.push(comp);
        input_nets.push(net);
        input_values.push(val);
    }

    // --- Pass 2: Z3 extraction of A and B via theory-owned probes ---
    //
    // The theory defines the probe protocol (sim_topology_source,
    // sim_probe_count, sim_probe_kind, sim_probe_col, sim_probe_source).
    // The server is domain-agnostic: it evaluates these contract functions
    // to get Kleis source strings, then parses and runs Z3 on each probe.

    let verify_preamble = build_graph_preamble(&verify_req, &theory_source);

    // Get topology and probe metadata from the theory
    let topology_src = match eval_str(&eval1, "sim_topology_source") {
        Ok(s) => s,
        Err(e) => {
            return SimulateSetupResponse {
                sim_mode,
                a_matrix: None,
                b_matrix: None,
                state_map,
                input_map,
                initial_state,
                input_values,
                dt,
                tau_min: None,
                chunk_size: None,
                error: Some(format!("sim_topology_source: {}", e)),
            };
        }
    };

    let n_probes = match eval_expr(&eval1, "sim_probe_count") {
        Ok(v) => v as usize,
        Err(e) => {
            return SimulateSetupResponse {
                sim_mode,
                a_matrix: None,
                b_matrix: None,
                state_map,
                input_map,
                initial_state,
                input_values,
                dt,
                tau_min: None,
                chunk_size: None,
                error: Some(format!("sim_probe_count: {}", e)),
            };
        }
    };

    let mut a_matrix = vec![vec![0.0_f64; ns]; ns];
    let mut b_matrix = vec![vec![0.0_f64; ni]; ns];

    for p in 0..n_probes {
        let kind = match eval_expr(&eval1, &format!("sim_probe_kind({})", p)) {
            Ok(v) => v as usize,
            Err(e) => {
                return SimulateSetupResponse {
                    sim_mode,
                    a_matrix: None,
                    b_matrix: None,
                    state_map,
                    input_map,
                    initial_state,
                    input_values,
                    dt,
                    tau_min: None,
                    chunk_size: None,
                    error: Some(format!("sim_probe_kind({}): {}", p, e)),
                };
            }
        };
        let col = match eval_expr(&eval1, &format!("sim_probe_col({})", p)) {
            Ok(v) => v as usize,
            Err(e) => {
                return SimulateSetupResponse {
                    sim_mode,
                    a_matrix: None,
                    b_matrix: None,
                    state_map,
                    input_map,
                    initial_state,
                    input_values,
                    dt,
                    tau_min: None,
                    chunk_size: None,
                    error: Some(format!("sim_probe_col({}): {}", p, e)),
                };
            }
        };
        let probe_src = match eval_str(&eval1, &format!("sim_probe_source({})", p)) {
            Ok(s) => s,
            Err(e) => {
                return SimulateSetupResponse {
                    sim_mode,
                    a_matrix: None,
                    b_matrix: None,
                    state_map,
                    input_map,
                    initial_state,
                    input_values,
                    dt,
                    tau_min: None,
                    chunk_size: None,
                    error: Some(format!("sim_probe_source({}): {}", p, e)),
                };
            }
        };

        let full_source = format!(
            "{}{}{}{}",
            verify_preamble, theory_source, topology_src, probe_src
        );

        let probe_program = match parse_kleis_program_with_file(
            &full_source,
            theory_path.to_string_lossy().to_string(),
        ) {
            Ok(prog) => prog,
            Err(e) => {
                return SimulateSetupResponse {
                    sim_mode,
                    a_matrix: None,
                    b_matrix: None,
                    state_map,
                    input_map,
                    initial_state,
                    input_values,
                    dt,
                    tau_min: None,
                    chunk_size: None,
                    error: Some(format!("Probe {} parse error: {}", p, e)),
                };
            }
        };

        let mut eval_probe = Evaluator::new();
        eval_probe.load_program(&probe_program);
        let results = eval_probe.run_all_examples(&probe_program);

        let probe_result = results.iter().find(|r| r.name == "PROBE");
        match probe_result {
            Some(r) if r.passed => {
                if let Some(ref w) = r.witness {
                    for binding in &w.bindings {
                        if let Expression::Const(val_str) = &binding.value {
                            if let Ok(val) = val_str.parse::<f64>() {
                                if let Some(rest) = binding.name.strip_prefix("d_") {
                                    if let Ok(i) = rest.parse::<usize>() {
                                        if i < ns {
                                            if kind == 0 && col < ns {
                                                a_matrix[i][col] = val;
                                            } else if kind == 1 && col < ni {
                                                b_matrix[i][col] = val;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Some(r) => {
                return SimulateSetupResponse {
                    sim_mode,
                    a_matrix: None,
                    b_matrix: None,
                    state_map,
                    input_map,
                    initial_state,
                    input_values,
                    dt,
                    tau_min: None,
                    chunk_size: None,
                    error: Some(format!(
                        "Probe {} failed: {}",
                        p,
                        r.error.as_deref().unwrap_or("unknown")
                    )),
                };
            }
            None => {
                return SimulateSetupResponse {
                    sim_mode,
                    a_matrix: None,
                    b_matrix: None,
                    state_map,
                    input_map,
                    initial_state,
                    input_values,
                    dt,
                    tau_min: None,
                    chunk_size: None,
                    error: Some(format!("Probe {} example not found", p)),
                };
            }
        }
    }

    // --- Pass 3: eval_concrete for theory-owned adaptive parameters ---
    // Inject extracted A/B matrices so theory can compute eigenvalue-based timing.
    let mut pass3_preamble = sim_preamble.clone();
    let a_rows: Vec<String> = a_matrix
        .iter()
        .map(|row| {
            format!(
                "[{}]",
                row.iter()
                    .map(|v| format!("{v}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })
        .collect();
    pass3_preamble.push_str(&format!("define sim_A_val = [{}]\n", a_rows.join(", ")));
    let b_rows: Vec<String> = b_matrix
        .iter()
        .map(|row| {
            format!(
                "[{}]",
                row.iter()
                    .map(|v| format!("{v}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })
        .collect();
    pass3_preamble.push_str(&format!("define sim_B_val = [{}]\n", b_rows.join(", ")));
    pass3_preamble.push_str(&format!("define sim_ns_val = {ns}\n"));
    pass3_preamble.push_str(&format!("define sim_ni_val = {ni}\n"));
    let inputs_str: Vec<String> = input_values.iter().map(|v| format!("{v}")).collect();
    pass3_preamble.push_str(&format!(
        "define sim_inputs = [{}]\n",
        inputs_str.join(", ")
    ));

    let pass3_source = format!("{}{}", pass3_preamble, theory_source);
    let (tau_min, dt_adaptive, chunk_size) = match parse_kleis_program_with_file(
        &pass3_source,
        theory_path.to_string_lossy().to_string(),
    ) {
        Ok(pass3_program) => {
            let mut eval3 = Evaluator::new();
            eval3.load_program(&pass3_program);
            let tau = eval_expr(&eval3, "sim_tau_min").ok();
            let dt_a = eval_expr(&eval3, "sim_dt").unwrap_or(dt);
            let cs = eval_expr(&eval3, "sim_chunk_size").map(|v| v as usize).ok();
            (tau, dt_a, cs)
        }
        Err(e) => {
            eprintln!("Pass 3 parse error (non-fatal, using defaults): {}", e);
            (None, dt, None)
        }
    };

    SimulateSetupResponse {
        sim_mode,
        a_matrix: Some(a_matrix),
        b_matrix: Some(b_matrix),
        state_map,
        input_map,
        initial_state,
        input_values,
        dt: dt_adaptive,
        tau_min,
        chunk_size,
        error: None,
    }
}

async fn simulate_setup_handler(
    State(_state): State<Arc<AppState>>,
    Json(req): Json<SimulateSetupRequest>,
) -> impl IntoResponse {
    let result = tokio::task::spawn_blocking(move || simulate_setup_core(req)).await;
    match result {
        Ok(resp) => Json(resp),
        Err(e) => Json(SimulateSetupResponse {
            sim_mode: "unknown".into(),
            a_matrix: None,
            b_matrix: None,
            state_map: vec![],
            input_map: vec![],
            initial_state: vec![],
            input_values: vec![],
            dt: 0.0,
            tau_min: None,
            chunk_size: None,
            error: Some(format!("Task panic: {}", e)),
        }),
    }
}

#[derive(Debug, Deserialize)]
struct SimulateSetupRequest {
    domain: String,
    components: Vec<VerifyGraphComponent>,
    incidence: VerifyGraphIncidence,
    port_labels: Vec<String>,
}

#[derive(Debug, Serialize)]
struct SimulateSetupResponse {
    sim_mode: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    a_matrix: Option<Vec<Vec<f64>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    b_matrix: Option<Vec<Vec<f64>>>,
    state_map: Vec<usize>,
    input_map: Vec<usize>,
    initial_state: Vec<f64>,
    input_values: Vec<f64>,
    dt: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    tau_min: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    chunk_size: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

// =============================================================================
// Graph simulation — domain-agnostic discrete/continuous stepper
// =============================================================================

#[derive(Debug, Deserialize)]
struct SimulateGraphRequest {
    domain: String,
    components: Vec<VerifyGraphComponent>,
    incidence: VerifyGraphIncidence,
    port_labels: Vec<String>,
    state: Vec<f64>,
    action: SimulateAction,
    #[serde(default)]
    last_fired: Option<usize>,
    #[serde(default)]
    sim_mode: Option<String>,
    #[serde(default)]
    a_matrix: Option<Vec<Vec<f64>>>,
    #[serde(default)]
    b_matrix: Option<Vec<Vec<f64>>>,
    #[serde(default)]
    inputs: Option<Vec<f64>>,
    #[serde(default)]
    dt: Option<f64>,
    #[serde(default)]
    chunk_size: Option<usize>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum SimulateAction {
    Step,
    Run { max_steps: Option<usize> },
    FindEnabled,
    Reset,
}

#[derive(Debug, Serialize)]
struct SimulateGraphResponse {
    state: Vec<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    time_series: Option<Vec<SimulateTimeSample>>,
    enabled: Vec<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    fired: Option<usize>,
    halted: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    halt_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
struct SimulateTimeSample {
    step: usize,
    state: Vec<f64>,
    fired: Option<usize>,
}

/// Build a simulation preamble: concrete defines for eval_concrete.
///
/// Unlike the verification preamble (Z3 axioms), this produces `define`
/// statements with nth-based lookup so the theory's sim_* functions can
/// be evaluated concretely without Z3.
fn build_sim_preamble(req: &SimulateGraphRequest, state: &[f64]) -> String {
    let nc = req.components.len();
    let nn = req.incidence.v;

    // Type code mapping (same logic as build_graph_preamble)
    let mut type_map: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    let mut next_code: usize = 1;
    for comp in &req.components {
        let ct = comp
            .component_type
            .as_deref()
            .unwrap_or("Unknown")
            .to_string();
        if !type_map.contains_key(&ct) {
            type_map.insert(ct, next_code);
            next_code += 1;
        }
    }

    // Build component-level incidence matrix from port-level entries
    let port_to_comp = |port_idx: usize| -> Option<usize> {
        req.port_labels
            .get(port_idx)
            .and_then(|label| label.split(':').next())
            .and_then(|ci_str| ci_str.parse::<usize>().ok())
    };

    let mut inc: std::collections::BTreeMap<(usize, usize), i64> =
        std::collections::BTreeMap::new();
    for entry in &req.incidence.entries {
        if let Some(ci) = port_to_comp(entry.port) {
            *inc.entry((entry.net, ci)).or_insert(0) += entry.value;
        }
    }

    let mut preamble = String::new();
    preamble.push_str("// Simulation preamble — auto-generated concrete defines\n");

    // State vector (f64 — supports both discrete integer tokens and continuous real values)
    let state_str: Vec<String> = state
        .iter()
        .map(|v| {
            if v.fract() == 0.0 && v.abs() < 1e15 {
                format!("{}", *v as i64)
            } else {
                format!("{}", v)
            }
        })
        .collect();
    preamble.push_str(&format!("define sim_state = [{}]\n", state_str.join(", ")));

    // Graph dimensions
    preamble.push_str(&format!("define graph_nc_val = {nc}\n"));
    preamble.push_str(&format!("define graph_nn_val = {nn}\n"));

    // Component type codes as list
    let ctype_list: Vec<String> = req
        .components
        .iter()
        .map(|c| {
            let ct = c.component_type.as_deref().unwrap_or("Unknown").to_string();
            format!("{}", type_map.get(&ct).copied().unwrap_or(0))
        })
        .collect();
    preamble.push_str(&format!(
        "define graph_ctype_val(c) = nth([{}], c)\n",
        ctype_list.join(", ")
    ));

    // Incidence matrix as nested lists
    let mut inc_rows: Vec<String> = Vec::new();
    for n in 0..nn {
        let row: Vec<String> = (0..nc)
            .map(|c| format!("{}", inc.get(&(n, c)).copied().unwrap_or(0)))
            .collect();
        inc_rows.push(format!("[{}]", row.join(", ")));
    }
    if nn == 0 {
        preamble.push_str("define graph_inc_val(n, c) = 0\n");
    } else {
        preamble.push_str(&format!(
            "define graph_inc_val(n, c) = nth(nth([{}], n), c)\n",
            inc_rows.join(", ")
        ));
    }

    // Component parameters as nested lists (real-valued)
    let max_params = req
        .components
        .iter()
        .map(|c| c.params.as_ref().map_or(0, |p| p.len()))
        .max()
        .unwrap_or(0);
    if nc == 0 || max_params == 0 {
        preamble.push_str("define graph_param_val(c, p) = 0\n");
    } else {
        let mut param_rows: Vec<String> = Vec::new();
        for comp in &req.components {
            let mut vals: Vec<f64> = vec![0.0; max_params];
            if let Some(ref params) = comp.params {
                let mut sorted_keys: Vec<&String> = params.keys().collect();
                sorted_keys.sort();
                for (pi, key) in sorted_keys.iter().enumerate() {
                    if pi < max_params {
                        vals[pi] = params[*key].as_f64().unwrap_or(0.0);
                    }
                }
            }
            let row: Vec<String> = vals
                .iter()
                .map(|v| {
                    if v.fract() == 0.0 && v.abs() < 1e15 {
                        format!("{}", *v as i64)
                    } else {
                        format!("{}", v)
                    }
                })
                .collect();
            param_rows.push(format!("[{}]", row.join(", ")));
        }
        preamble.push_str(&format!(
            "define graph_param_val(c, p) = nth(nth([{}], c), p)\n",
            param_rows.join(", ")
        ));
    }

    // TYPE_X constants as concrete defines
    for (name, &code) in &type_map {
        let safe_name = name.replace(' ', "_");
        preamble.push_str(&format!("define TYPE_{safe_name} = {code}\n"));
    }

    // Also provide the verification-style names for the shared classification
    // functions (is_place, is_transition etc.) that reference graph_ctype.
    // The sim theory uses graph_ctype_val variants, so this isn't strictly needed,
    // but we alias graph_ctype to graph_ctype_val for any shared code.
    preamble.push_str("define graph_ctype(c) = graph_ctype_val(c)\n");
    preamble.push_str("define graph_param(c, p) = graph_param_val(c, p)\n");

    preamble.push('\n');
    preamble
}

/// Build just the state-update define (for multi-step without full re-parse).
fn build_sim_state_define(state: &[f64]) -> String {
    let state_str: Vec<String> = state
        .iter()
        .map(|v| {
            if v.fract() == 0.0 && v.abs() < 1e15 {
                format!("{}", *v as i64)
            } else {
                format!("{}", v)
            }
        })
        .collect();
    format!("define sim_state = [{}]\n", state_str.join(", "))
}

/// Compute ẋ = Ax + Bu (pure matrix-vector math, domain-agnostic).
fn linear_deriv(a: &[Vec<f64>], b: &[Vec<f64>], x: &[f64], u: &[f64]) -> Vec<f64> {
    let ns = x.len();
    let ni = u.len();
    (0..ns)
        .map(|i| {
            let ax: f64 = (0..ns).map(|j| a[i][j] * x[j]).sum();
            let bu: f64 = (0..ni).map(|k| b[i][k] * u[k]).sum();
            ax + bu
        })
        .collect()
}

/// One RK4 step: x_{n+1} from x_n for ẋ = Ax + Bu.
fn rk4_step(a: &[Vec<f64>], b: &[Vec<f64>], x: &[f64], u: &[f64], dt: f64) -> Vec<f64> {
    let k1 = linear_deriv(a, b, x, u);

    let x1: Vec<f64> = x.iter().zip(&k1).map(|(xi, k)| xi + 0.5 * dt * k).collect();
    let k2 = linear_deriv(a, b, &x1, u);

    let x2: Vec<f64> = x.iter().zip(&k2).map(|(xi, k)| xi + 0.5 * dt * k).collect();
    let k3 = linear_deriv(a, b, &x2, u);

    let x3: Vec<f64> = x.iter().zip(&k3).map(|(xi, k)| xi + dt * k).collect();
    let k4 = linear_deriv(a, b, &x3, u);

    x.iter()
        .enumerate()
        .map(|(i, xi)| xi + dt / 6.0 * (k1[i] + 2.0 * k2[i] + 2.0 * k3[i] + k4[i]))
        .collect()
}

fn simulate_continuous_core(
    req: &SimulateGraphRequest,
    _theory_source: &str,
) -> SimulateGraphResponse {
    let a_matrix = match &req.a_matrix {
        Some(a) => a,
        None => {
            return SimulateGraphResponse {
                state: req.state.clone(),
                time_series: None,
                enabled: vec![],
                fired: None,
                halted: true,
                halt_reason: None,
                error: Some("Missing a_matrix for continuous simulation".into()),
            };
        }
    };
    let b_matrix = match &req.b_matrix {
        Some(b) => b,
        None => {
            return SimulateGraphResponse {
                state: req.state.clone(),
                time_series: None,
                enabled: vec![],
                fired: None,
                halted: true,
                halt_reason: None,
                error: Some("Missing b_matrix for continuous simulation".into()),
            };
        }
    };
    let inputs = match &req.inputs {
        Some(i) => i,
        None => {
            return SimulateGraphResponse {
                state: req.state.clone(),
                time_series: None,
                enabled: vec![],
                fired: None,
                halted: true,
                halt_reason: None,
                error: Some("Missing inputs for continuous simulation".into()),
            };
        }
    };
    let dt = req.dt.unwrap_or(0.0001);
    let chunk_size = req.chunk_size.unwrap_or(100);

    let mut state = req.state.clone();
    let mut history: Vec<SimulateTimeSample> = Vec::with_capacity(chunk_size);

    for step in 0..chunk_size {
        state = rk4_step(a_matrix, b_matrix, &state, inputs, dt);
        history.push(SimulateTimeSample {
            step,
            state: state.clone(),
            fired: None,
        });
    }

    SimulateGraphResponse {
        state,
        time_series: Some(history),
        enabled: vec![],
        fired: None,
        halted: false,
        halt_reason: None,
        error: None,
    }
}

fn simulate_graph_core(req: SimulateGraphRequest) -> SimulateGraphResponse {
    use kleis::ast::Expression;
    use kleis::evaluator::Evaluator;
    use kleis::kleis_parser::parse_kleis_program;

    let nc = req.components.len();

    let theory_path =
        std::path::PathBuf::from("std_template_lib").join(format!("{}.kleis", req.domain));

    if !theory_path.exists() {
        return SimulateGraphResponse {
            state: req.state,
            time_series: None,
            enabled: vec![],
            fired: None,
            halted: true,
            halt_reason: None,
            error: Some(format!(
                "No companion theory file: {}",
                theory_path.display()
            )),
        };
    }

    let theory_source = match std::fs::read_to_string(&theory_path) {
        Ok(s) => s,
        Err(e) => {
            return SimulateGraphResponse {
                state: req.state,
                time_series: None,
                enabled: vec![],
                fired: None,
                halted: true,
                halt_reason: None,
                error: Some(format!("Failed to read theory: {}", e)),
            };
        }
    };

    // Build structural preamble once (everything except sim_state)
    let structural_preamble = build_sim_preamble(&req, &req.state);

    // Build evaluator from state + pre-computed structural parts
    let make_evaluator = |state: &[f64]| -> Result<Evaluator, String> {
        let state_line = build_sim_state_define(state);
        let full_source = format!("{}{}{}", state_line, structural_preamble, theory_source);
        let program =
            parse_kleis_program(&full_source).map_err(|e| format!("Parse error: {}", e))?;
        let mut eval = Evaluator::new();
        eval.load_program(&program)?;
        Ok(eval)
    };

    // Eval helpers
    let eval_enabled = |eval: &Evaluator, t: usize| -> bool {
        let expr = Expression::Operation {
            name: "sim_enabled".to_string(),
            args: vec![Expression::Const(t.to_string())],
            span: None,
        };
        match eval.eval_concrete(&expr) {
            Ok(Expression::Const(ref s)) | Ok(Expression::Object(ref s)) => s == "true",
            _ => false,
        }
    };

    let eval_fire = |eval: &Evaluator, t: usize, c: usize| -> f64 {
        let expr = Expression::Operation {
            name: "sim_fire".to_string(),
            args: vec![
                Expression::Const(t.to_string()),
                Expression::Const(c.to_string()),
            ],
            span: None,
        };
        match eval.eval_concrete(&expr) {
            Ok(Expression::Const(ref s)) => s.parse::<f64>().unwrap_or(0.0),
            _ => 0.0,
        }
    };

    let eval_halt_reason = |eval: &Evaluator| -> String {
        let expr = Expression::Operation {
            name: "sim_halt_reason".to_string(),
            args: vec![],
            span: None,
        };
        match eval.eval_concrete(&expr) {
            Ok(Expression::String(s)) => s,
            Ok(Expression::Const(s)) => s,
            _ => "unknown".to_string(),
        }
    };

    let find_enabled =
        |eval: &Evaluator| -> Vec<usize> { (0..nc).filter(|&t| eval_enabled(eval, t)).collect() };

    let fire = |eval: &Evaluator, t: usize| -> Vec<f64> {
        (0..nc).map(|c| eval_fire(eval, t, c)).collect()
    };

    let pick_next = |enabled: &[usize], last: Option<usize>| -> usize {
        if enabled.len() <= 1 {
            return enabled[0];
        }
        if let Some(prev) = last {
            if let Some(&t) = enabled.iter().find(|&&e| e > prev) {
                return t;
            }
        }
        enabled[0]
    };

    let update_state = |eval: &mut Evaluator, state: &[f64]| -> Result<(), String> {
        let state_src = build_sim_state_define(state);
        let state_prog =
            parse_kleis_program(&state_src).map_err(|e| format!("State parse error: {}", e))?;
        eval.load_program(&state_prog)?;
        Ok(())
    };

    // --- Continuous simulation branch ---
    if req.sim_mode.as_deref() == Some("continuous") {
        return simulate_continuous_core(&req, &theory_source);
    }

    match req.action {
        SimulateAction::Reset => {
            // The client provides the initial state (computed from stateParam
            // metadata in .kleist). The server just validates it via the theory.
            let eval = match make_evaluator(&req.state) {
                Ok(e) => e,
                Err(e) => {
                    return SimulateGraphResponse {
                        state: req.state,
                        time_series: None,
                        enabled: vec![],
                        fired: None,
                        halted: false,
                        halt_reason: None,
                        error: Some(e),
                    };
                }
            };
            let enabled = find_enabled(&eval);
            SimulateGraphResponse {
                state: req.state,
                time_series: None,
                enabled,
                fired: None,
                halted: false,
                halt_reason: None,
                error: None,
            }
        }

        SimulateAction::FindEnabled => {
            let eval = match make_evaluator(&req.state) {
                Ok(e) => e,
                Err(e) => {
                    return SimulateGraphResponse {
                        state: req.state,
                        time_series: None,
                        enabled: vec![],
                        fired: None,
                        halted: false,
                        halt_reason: None,
                        error: Some(e),
                    };
                }
            };
            let enabled = find_enabled(&eval);
            SimulateGraphResponse {
                state: req.state,
                time_series: None,
                enabled,
                fired: None,
                halted: false,
                halt_reason: None,
                error: None,
            }
        }

        SimulateAction::Step => {
            let eval = match make_evaluator(&req.state) {
                Ok(e) => e,
                Err(e) => {
                    return SimulateGraphResponse {
                        state: req.state,
                        time_series: None,
                        enabled: vec![],
                        fired: None,
                        halted: true,
                        halt_reason: None,
                        error: Some(e),
                    };
                }
            };
            let enabled = find_enabled(&eval);
            if enabled.is_empty() {
                let reason = eval_halt_reason(&eval);
                return SimulateGraphResponse {
                    state: req.state,
                    time_series: None,
                    enabled: vec![],
                    fired: None,
                    halted: true,
                    halt_reason: Some(reason),
                    error: None,
                };
            }
            let t = pick_next(&enabled, req.last_fired);
            let new_state = fire(&eval, t);
            let mut eval2 = match make_evaluator(&new_state) {
                Ok(e) => e,
                Err(e) => {
                    return SimulateGraphResponse {
                        state: new_state,
                        time_series: None,
                        enabled: vec![],
                        fired: Some(t),
                        halted: true,
                        halt_reason: None,
                        error: Some(e),
                    };
                }
            };
            let new_enabled = find_enabled(&eval2);
            let halted = new_enabled.is_empty();
            let halt_reason = if halted {
                Some(eval_halt_reason(&eval2))
            } else {
                None
            };
            SimulateGraphResponse {
                state: new_state,
                time_series: None,
                enabled: new_enabled,
                fired: Some(t),
                halted,
                halt_reason,
                error: None,
            }
        }

        SimulateAction::Run { max_steps } => {
            let max = max_steps.unwrap_or(1000);
            let mut state = req.state;
            let mut eval = match make_evaluator(&state) {
                Ok(e) => e,
                Err(e) => {
                    return SimulateGraphResponse {
                        state,
                        time_series: None,
                        enabled: vec![],
                        fired: None,
                        halted: true,
                        halt_reason: None,
                        error: Some(e),
                    };
                }
            };
            let mut history: Vec<SimulateTimeSample> = Vec::new();
            let mut halted = false;
            let mut halt_reason = None;
            let mut last = req.last_fired;

            for step in 0..max {
                let enabled = find_enabled(&eval);
                if enabled.is_empty() {
                    halted = true;
                    halt_reason = Some(eval_halt_reason(&eval));
                    break;
                }
                let t = pick_next(&enabled, last);
                state = fire(&eval, t);
                last = Some(t);
                history.push(SimulateTimeSample {
                    step,
                    state: state.clone(),
                    fired: Some(t),
                });
                if let Err(e) = update_state(&mut eval, &state) {
                    return SimulateGraphResponse {
                        state,
                        time_series: Some(history),
                        enabled: vec![],
                        fired: None,
                        halted: true,
                        halt_reason: None,
                        error: Some(e),
                    };
                }
            }

            if !halted && history.len() >= max {
                halted = true;
                halt_reason = Some("max_steps".to_string());
            }

            let enabled = find_enabled(&eval);
            SimulateGraphResponse {
                state,
                time_series: Some(history),
                enabled,
                fired: None,
                halted,
                halt_reason,
                error: None,
            }
        }
    }
}

async fn simulate_graph_handler(
    State(_state): State<Arc<AppState>>,
    Json(req): Json<SimulateGraphRequest>,
) -> impl IntoResponse {
    let result = tokio::task::spawn_blocking(move || simulate_graph_core(req)).await;
    match result {
        Ok(resp) => Json(resp),
        Err(e) => Json(SimulateGraphResponse {
            state: vec![],
            time_series: None,
            enabled: vec![],
            fired: None,
            halted: true,
            halt_reason: None,
            error: Some(format!("Task panic: {}", e)),
        }),
    }
}

// =============================================================================
// Tests for verify_graph
// =============================================================================

#[cfg(test)]
mod verify_graph_tests {
    use super::*;
    use std::collections::HashMap;

    fn comp(
        comp_type: &str,
        component_type: &str,
        params: Vec<(&str, serde_json::Value)>,
    ) -> VerifyGraphComponent {
        let p: HashMap<String, serde_json::Value> = params
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect();
        VerifyGraphComponent {
            comp_type: comp_type.to_string(),
            component_type: Some(component_type.to_string()),
            params: if p.is_empty() { None } else { Some(p) },
        }
    }

    fn entry(net: usize, port: usize, value: i64) -> VerifyGraphEntry {
        VerifyGraphEntry { net, port, value }
    }

    // =========================================================================
    // Linear workflow:  Source(1) → T → Place(0) → T → Sink(0)
    //
    //  c0=SourcePlace  c1=Transition  c2=Place  c3=Transition  c4=SinkPlace
    //  Ports (4 per component): c0:0..3  c1:4..7  c2:8..11  c3:12..15  c4:16..19
    //  Nets:
    //    n0: c0(+1) → c1(-1)   n1: c1(+1) → c2(-1)
    //    n2: c2(+1) → c3(-1)   n3: c3(+1) → c4(-1)
    // =========================================================================
    fn petri_linear_workflow() -> VerifyGraphRequest {
        VerifyGraphRequest {
            domain: "petri_net".to_string(),
            components: vec![
                comp(
                    "pn_source_place",
                    "SourcePlace",
                    vec![("tokens", serde_json::json!(1))],
                ),
                comp("pn_transition", "Transition", vec![]),
                comp("pn_place", "Place", vec![("tokens", serde_json::json!(0))]),
                comp("pn_transition", "Transition", vec![]),
                comp(
                    "pn_sink_place",
                    "SinkPlace",
                    vec![("tokens", serde_json::json!(0))],
                ),
            ],
            incidence: VerifyGraphIncidence {
                entries: vec![
                    entry(0, 1, 1),
                    entry(0, 6, -1),
                    entry(1, 5, 1),
                    entry(1, 10, -1),
                    entry(2, 9, 1),
                    entry(2, 14, -1),
                    entry(3, 13, 1),
                    entry(3, 18, -1),
                ],
                v: 4,
                p: 20,
            },
            port_labels: (0..20)
                .map(|i| format!("{}:{}", i / 4, ["top", "right", "bottom", "left"][i % 4]))
                .collect(),
        }
    }

    // -------------------------------------------------------------------------
    // Preamble structure tests (domain-agnostic)
    // -------------------------------------------------------------------------

    #[test]
    fn preamble_emits_generic_counts() {
        let preamble = build_graph_preamble(&petri_linear_workflow(), "");
        assert!(
            preamble.contains("graph_nc = 5"),
            "expected 5 components:\n{preamble}"
        );
        assert!(
            preamble.contains("graph_nn = 4"),
            "expected 4 nets:\n{preamble}"
        );
    }

    #[test]
    fn preamble_emits_type_codes() {
        let preamble = build_graph_preamble(&petri_linear_workflow(), "");
        assert!(
            preamble.contains("TYPE_SourcePlace"),
            "missing TYPE_SourcePlace"
        );
        assert!(
            preamble.contains("TYPE_Transition"),
            "missing TYPE_Transition"
        );
        assert!(preamble.contains("TYPE_Place"), "missing TYPE_Place");
        assert!(
            preamble.contains("TYPE_SinkPlace"),
            "missing TYPE_SinkPlace"
        );
    }

    #[test]
    fn preamble_emits_per_component_type() {
        let preamble = build_graph_preamble(&petri_linear_workflow(), "");
        assert!(preamble.contains("graph_ctype(0)"), "missing ctype for c0");
        assert!(preamble.contains("graph_ctype(4)"), "missing ctype for c4");
    }

    #[test]
    fn preamble_emits_params() {
        let preamble = build_graph_preamble(&petri_linear_workflow(), "");
        assert!(
            preamble.contains("graph_param(0, 0) = 1"),
            "source place should have param 0 = 1:\n{preamble}"
        );
        assert!(
            preamble.contains("graph_param(2, 0) = 0"),
            "middle place should have param 0 = 0:\n{preamble}"
        );
    }

    #[test]
    fn preamble_emits_component_level_incidence() {
        let preamble = build_graph_preamble(&petri_linear_workflow(), "");
        assert!(
            preamble.contains("graph_inc(0, 0) = 1.0"),
            "c0 source of net 0"
        );
        assert!(
            preamble.contains("graph_inc(0, 1) = -1.0"),
            "c1 target of net 0"
        );
        assert!(
            preamble.contains("graph_inc(1, 1) = 1.0"),
            "c1 source of net 1"
        );
        assert!(
            preamble.contains("graph_inc(1, 2) = -1.0"),
            "c2 target of net 1"
        );
    }

    #[test]
    fn preamble_has_no_domain_specific_operations() {
        let preamble = build_graph_preamble(&petri_linear_workflow(), "");
        assert!(
            !preamble.contains("graph_iw"),
            "no domain-specific graph_iw"
        );
        assert!(
            !preamble.contains("graph_ow"),
            "no domain-specific graph_ow"
        );
        assert!(
            !preamble.contains("graph_m0"),
            "no domain-specific graph_m0"
        );
        assert!(
            !preamble.contains("valid_state"),
            "no domain-specific valid_state"
        );
        assert!(
            !preamble.contains("can_fire"),
            "no domain-specific can_fire"
        );
    }

    #[test]
    fn preamble_closed_world_axiom() {
        let preamble = build_graph_preamble(&petri_linear_workflow(), "");
        assert!(
            preamble.contains("ctype_closed"),
            "missing closed-world axiom"
        );
    }

    #[test]
    fn preamble_no_params_when_none() {
        let req = VerifyGraphRequest {
            domain: "petri_net".to_string(),
            components: vec![VerifyGraphComponent {
                comp_type: "pn_place".to_string(),
                component_type: Some("Place".to_string()),
                params: None,
            }],
            incidence: VerifyGraphIncidence {
                entries: vec![],
                v: 0,
                p: 4,
            },
            port_labels: (0..4)
                .map(|i| format!("0:{}", ["top", "right", "bottom", "left"][i]))
                .collect(),
        };
        let preamble = build_graph_preamble(&req, "");
        assert!(preamble.contains("graph_nc = 1"));
        assert!(
            !preamble.contains("graph_param(0"),
            "no params emitted when None"
        );
    }

    #[test]
    fn preamble_empty_graph() {
        let req = VerifyGraphRequest {
            domain: "petri_net".to_string(),
            components: vec![],
            incidence: VerifyGraphIncidence {
                entries: vec![],
                v: 0,
                p: 0,
            },
            port_labels: vec![],
        };
        let preamble = build_graph_preamble(&req, "");
        assert!(preamble.contains("graph_nc = 0"));
        assert!(preamble.contains("graph_nn = 0"));
    }

    #[test]
    fn preamble_electronics_params() {
        let req = VerifyGraphRequest {
            domain: "electronics".to_string(),
            components: vec![
                comp("resistor", "Passive", vec![("R", serde_json::json!(1000))]),
                comp("dc_voltage", "Source", vec![("V", serde_json::json!(5))]),
            ],
            incidence: VerifyGraphIncidence {
                entries: vec![entry(0, 1, 1), entry(0, 3, -1)],
                v: 1,
                p: 4,
            },
            port_labels: vec![
                "0:left".into(),
                "0:right".into(),
                "1:pos".into(),
                "1:neg".into(),
            ],
        };
        let preamble = build_graph_preamble(&req, "");
        assert!(
            preamble.contains("graph_param(0, 0) = 1000"),
            "resistor R=1000"
        );
        assert!(preamble.contains("graph_param(1, 0) = 5"), "voltage V=5");
        assert!(preamble.contains("TYPE_Passive"));
        assert!(preamble.contains("TYPE_Source"));
    }

    // -------------------------------------------------------------------------
    // Z3 verification tests (theory + preamble → examples checked)
    // -------------------------------------------------------------------------

    #[test]
    fn petri_linear_workflow_passes_z3() {
        let resp = verify_graph_core(petri_linear_workflow());
        assert!(resp.error.is_none(), "unexpected error: {:?}", resp.error);
        assert!(
            !resp.results.is_empty(),
            "expected at least one example result"
        );
        for r in &resp.results {
            assert!(r.passed, "example '{}' failed: {:?}", r.name, r.error);
        }
        assert!(resp.success);
    }

    #[test]
    fn petri_no_tokens_fails_initial_marking() {
        let mut req = petri_linear_workflow();
        req.components[0] = comp(
            "pn_source_place",
            "SourcePlace",
            vec![("tokens", serde_json::json!(0))],
        );
        let resp = verify_graph_core(req);
        assert!(resp.error.is_none(), "unexpected error: {:?}", resp.error);
        let initial = resp
            .results
            .iter()
            .find(|r| r.name.contains("INITIAL MARKING"));
        assert!(initial.is_some(), "expected INITIAL MARKING example");
        assert!(!initial.unwrap().passed, "should fail with all 0 tokens");
        assert!(!resp.success);
    }

    #[test]
    fn petri_multiple_tokens_passes() {
        let mut req = petri_linear_workflow();
        req.components[0] = comp(
            "pn_source_place",
            "SourcePlace",
            vec![("tokens", serde_json::json!(5))],
        );
        let resp = verify_graph_core(req);
        assert!(resp.error.is_none(), "unexpected error: {:?}", resp.error);
        assert!(resp.success, "should pass with 5 tokens");
        let preamble = resp.preamble.unwrap();
        assert!(
            preamble.contains("graph_param(0, 0) = 5"),
            "preamble should reflect 5 tokens"
        );
    }

    #[test]
    fn petri_no_sink_fails_sink_exists() {
        let req = VerifyGraphRequest {
            domain: "petri_net".to_string(),
            components: vec![
                comp(
                    "pn_source_place",
                    "SourcePlace",
                    vec![("tokens", serde_json::json!(1))],
                ),
                comp("pn_transition", "Transition", vec![]),
                comp("pn_place", "Place", vec![("tokens", serde_json::json!(0))]),
            ],
            incidence: VerifyGraphIncidence {
                entries: vec![
                    entry(0, 1, 1),
                    entry(0, 6, -1),
                    entry(1, 5, 1),
                    entry(1, 10, -1),
                ],
                v: 2,
                p: 12,
            },
            port_labels: (0..12)
                .map(|i| format!("{}:{}", i / 4, ["top", "right", "bottom", "left"][i % 4]))
                .collect(),
        };
        let resp = verify_graph_core(req);
        assert!(resp.error.is_none(), "unexpected error: {:?}", resp.error);
        let sink_check = resp.results.iter().find(|r| r.name.contains("SINK EXISTS"));
        assert!(sink_check.is_some(), "expected SINK EXISTS example");
        assert!(
            !sink_check.unwrap().passed,
            "SINK EXISTS should fail when no SinkPlace"
        );
    }

    #[test]
    fn petri_non_bipartite_fails() {
        let req = VerifyGraphRequest {
            domain: "petri_net".to_string(),
            components: vec![
                comp(
                    "pn_source_place",
                    "SourcePlace",
                    vec![("tokens", serde_json::json!(1))],
                ),
                comp(
                    "pn_sink_place",
                    "SinkPlace",
                    vec![("tokens", serde_json::json!(0))],
                ),
            ],
            incidence: VerifyGraphIncidence {
                entries: vec![entry(0, 1, 1), entry(0, 5, -1)],
                v: 1,
                p: 8,
            },
            port_labels: (0..8)
                .map(|i| format!("{}:{}", i / 4, ["top", "right", "bottom", "left"][i % 4]))
                .collect(),
        };
        let resp = verify_graph_core(req);
        assert!(resp.error.is_none(), "unexpected error: {:?}", resp.error);
        let bipartite = resp.results.iter().find(|r| r.name.contains("BIPARTITE"));
        assert!(bipartite.is_some(), "expected BIPARTITE example");
        assert!(
            !bipartite.unwrap().passed,
            "BIPARTITE should fail with place-to-place arc"
        );
    }

    #[test]
    fn petri_empty_graph_fails() {
        let req = VerifyGraphRequest {
            domain: "petri_net".to_string(),
            components: vec![],
            incidence: VerifyGraphIncidence {
                entries: vec![],
                v: 0,
                p: 0,
            },
            port_labels: vec![],
        };
        let resp = verify_graph_core(req);
        assert!(
            resp.error.is_none(),
            "empty graph should still parse: {:?}",
            resp.error
        );
        assert!(!resp.success, "empty graph should fail verification");
    }

    #[test]
    fn petri_self_loop_passes() {
        let req = VerifyGraphRequest {
            domain: "petri_net".to_string(),
            components: vec![
                comp("pn_place", "Place", vec![("tokens", serde_json::json!(1))]),
                comp("pn_transition", "Transition", vec![]),
            ],
            incidence: VerifyGraphIncidence {
                entries: vec![
                    entry(0, 1, 1),
                    entry(0, 6, -1),
                    entry(1, 5, 1),
                    entry(1, 3, -1),
                ],
                v: 2,
                p: 8,
            },
            port_labels: (0..8)
                .map(|i| format!("{}:{}", i / 4, ["top", "right", "bottom", "left"][i % 4]))
                .collect(),
        };
        let preamble = build_graph_preamble(&req, "");
        assert!(
            preamble.contains("graph_inc(0, 0) = 1.0"),
            "c0 source of net 0"
        );
        assert!(
            preamble.contains("graph_inc(0, 1) = -1.0"),
            "c1 target of net 0"
        );
        assert!(
            preamble.contains("graph_inc(1, 1) = 1.0"),
            "c1 source of net 1"
        );
        assert!(
            preamble.contains("graph_inc(1, 0) = -1.0"),
            "c0 target of net 1"
        );
    }

    #[test]
    fn petri_fork_topology() {
        let req = VerifyGraphRequest {
            domain: "petri_net".to_string(),
            components: vec![
                comp(
                    "pn_source_place",
                    "SourcePlace",
                    vec![("tokens", serde_json::json!(1))],
                ),
                comp("pn_transition", "Transition", vec![]),
                comp("pn_place", "Place", vec![("tokens", serde_json::json!(0))]),
                comp("pn_place", "Place", vec![("tokens", serde_json::json!(0))]),
            ],
            incidence: VerifyGraphIncidence {
                entries: vec![
                    entry(0, 1, 1),
                    entry(0, 6, -1),
                    entry(1, 5, 1),
                    entry(1, 10, -1),
                    entry(2, 7, 1),
                    entry(2, 14, -1),
                ],
                v: 3,
                p: 16,
            },
            port_labels: (0..16)
                .map(|i| format!("{}:{}", i / 4, ["top", "right", "bottom", "left"][i % 4]))
                .collect(),
        };
        let preamble = build_graph_preamble(&req, "");
        assert!(preamble.contains("graph_nc = 4"), "4 components");
        assert!(preamble.contains("graph_nn = 3"), "3 nets");
        assert!(
            preamble.contains("graph_inc(1, 1) = 1.0"),
            "c1 source of net 1"
        );
        assert!(
            preamble.contains("graph_inc(1, 2) = -1.0"),
            "c2 target of net 1"
        );
        assert!(
            preamble.contains("graph_inc(2, 1) = 1.0"),
            "c1 source of net 2"
        );
        assert!(
            preamble.contains("graph_inc(2, 3) = -1.0"),
            "c3 target of net 2"
        );
    }

    #[test]
    fn petri_join_topology() {
        let req = VerifyGraphRequest {
            domain: "petri_net".to_string(),
            components: vec![
                comp(
                    "pn_source_place",
                    "SourcePlace",
                    vec![("tokens", serde_json::json!(1))],
                ),
                comp(
                    "pn_source_place",
                    "SourcePlace",
                    vec![("tokens", serde_json::json!(1))],
                ),
                comp("pn_transition", "Transition", vec![]),
                comp(
                    "pn_sink_place",
                    "SinkPlace",
                    vec![("tokens", serde_json::json!(0))],
                ),
            ],
            incidence: VerifyGraphIncidence {
                entries: vec![
                    entry(0, 1, 1),
                    entry(0, 10, -1),
                    entry(1, 5, 1),
                    entry(1, 8, -1),
                    entry(2, 9, 1),
                    entry(2, 14, -1),
                ],
                v: 3,
                p: 16,
            },
            port_labels: (0..16)
                .map(|i| format!("{}:{}", i / 4, ["top", "right", "bottom", "left"][i % 4]))
                .collect(),
        };
        let resp = verify_graph_core(req);
        assert!(resp.error.is_none(), "unexpected error: {:?}", resp.error);
        for r in &resp.results {
            assert!(r.passed, "example '{}' failed: {:?}", r.name, r.error);
        }
        assert!(resp.success, "join net with tokens should pass");
    }

    // -------------------------------------------------------------------------
    // Missing / no-theory domains
    // -------------------------------------------------------------------------

    #[test]
    fn missing_theory_returns_error() {
        let req = VerifyGraphRequest {
            domain: "nonexistent_domain".to_string(),
            components: vec![],
            incidence: VerifyGraphIncidence {
                entries: vec![],
                v: 0,
                p: 0,
            },
            port_labels: vec![],
        };
        let resp = verify_graph_core(req);
        assert!(!resp.success);
        assert!(resp.error.is_some());
        assert!(resp.error.unwrap().contains("No companion theory file"));
    }

    #[test]
    fn electronics_has_theory() {
        // electronics.kleis now exists — verify it loads and runs checks
        let req = VerifyGraphRequest {
            domain: "electronics".to_string(),
            components: vec![
                comp("resistor", "Passive", vec![("R", serde_json::json!(1000))]),
                comp(
                    "dc_voltage",
                    "VoltageSource",
                    vec![("V", serde_json::json!(5))],
                ),
                comp("ground", "Ground", vec![]),
            ],
            incidence: VerifyGraphIncidence {
                entries: vec![
                    entry(0, 0, 1),
                    entry(0, 2, -1),
                    entry(1, 1, -1),
                    entry(1, 3, -1),
                    entry(1, 4, -1),
                ],
                v: 2,
                p: 5,
            },
            port_labels: vec![
                "0:pos".into(),
                "0:neg".into(),
                "1:left".into(),
                "1:right".into(),
                "2:pin".into(),
            ],
        };
        let resp = verify_graph_core(req);
        assert!(resp.error.is_none(), "unexpected error: {:?}", resp.error);
        assert!(!resp.results.is_empty(), "expected Z3 results from theory");
    }

    #[test]
    fn bond_graph_has_theory() {
        // bond_graph.kleis now exists — verify it loads and runs checks
        let req = VerifyGraphRequest {
            domain: "bond_graph".to_string(),
            components: vec![
                comp("bg_effort_source", "EffortSource", vec![]),
                comp("bg_junction_1", "Junction1", vec![]),
                comp("bg_resistor", "Resistor", vec![]),
            ],
            incidence: VerifyGraphIncidence {
                entries: vec![
                    entry(0, 0, 1),
                    entry(0, 1, -1),
                    entry(1, 2, 1),
                    entry(1, 3, -1),
                ],
                v: 2,
                p: 4,
            },
            port_labels: vec![
                "0:port".into(),
                "1:left".into(),
                "1:right".into(),
                "2:port".into(),
            ],
        };
        let resp = verify_graph_core(req);
        assert!(resp.error.is_none(), "unexpected error: {:?}", resp.error);
        assert!(!resp.results.is_empty(), "expected Z3 results from theory");
    }

    // -------------------------------------------------------------------------
    // Mutex net: structural checks via Z3
    // -------------------------------------------------------------------------
    #[test]
    fn petri_mutex_passes_structural() {
        let req = VerifyGraphRequest {
            domain: "petri_net".to_string(),
            components: vec![
                comp(
                    "pn_source_place",
                    "SourcePlace",
                    vec![("tokens", serde_json::json!(1))],
                ),
                comp("pn_place", "Place", vec![("tokens", serde_json::json!(0))]),
                comp(
                    "pn_source_place",
                    "SourcePlace",
                    vec![("tokens", serde_json::json!(1))],
                ),
                comp("pn_place", "Place", vec![("tokens", serde_json::json!(0))]),
                comp("pn_place", "Place", vec![("tokens", serde_json::json!(1))]),
                comp("pn_transition", "Transition", vec![]),
                comp("pn_transition", "Transition", vec![]),
                comp("pn_transition", "Transition", vec![]),
                comp("pn_transition", "Transition", vec![]),
                comp(
                    "pn_sink_place",
                    "SinkPlace",
                    vec![("tokens", serde_json::json!(0))],
                ),
            ],
            incidence: VerifyGraphIncidence {
                entries: vec![
                    entry(0, 1, 1),
                    entry(0, 22, -1),
                    entry(1, 17, 1),
                    entry(1, 23, -1),
                    entry(2, 21, 1),
                    entry(2, 6, -1),
                    entry(3, 5, 1),
                    entry(3, 26, -1),
                    entry(4, 25, 1),
                    entry(4, 3, -1),
                    entry(5, 27, 1),
                    entry(5, 19, -1),
                    entry(6, 9, 1),
                    entry(6, 30, -1),
                    entry(7, 18, 1),
                    entry(7, 31, -1),
                    entry(8, 29, 1),
                    entry(8, 14, -1),
                    entry(9, 13, 1),
                    entry(9, 34, -1),
                    entry(10, 33, 1),
                    entry(10, 11, -1),
                    entry(11, 35, 1),
                    entry(11, 16, -1),
                ],
                v: 12,
                p: 40,
            },
            port_labels: (0..40)
                .map(|i| format!("{}:{}", i / 4, ["top", "right", "bottom", "left"][i % 4]))
                .collect(),
        };

        let preamble = build_graph_preamble(&req, "");
        assert!(preamble.contains("graph_nc = 10"), "10 components");
        assert!(preamble.contains("graph_nn = 12"), "12 nets");

        let resp = verify_graph_core(req);
        assert!(resp.error.is_none(), "unexpected error: {:?}", resp.error);
        for r in &resp.results {
            assert!(r.passed, "example '{}' failed: {:?}", r.name, r.error);
        }
        assert!(resp.success);
    }

    // =========================================================================
    // Bond graph verification tests
    // =========================================================================

    fn bond_graph_simple_se_r() -> VerifyGraphRequest {
        // Se --[n0]--> 1-junction --[n1]--> R
        // 3 components, 2 nets, each component has 1 port (sources/1-ports)
        // and 4 ports (junction)
        VerifyGraphRequest {
            domain: "bond_graph".to_string(),
            components: vec![
                comp("bg_effort_source", "EffortSource", vec![]),
                comp("bg_junction_1", "Junction1", vec![]),
                comp("bg_resistor", "Resistor", vec![]),
            ],
            incidence: VerifyGraphIncidence {
                entries: vec![
                    entry(0, 0, 1),
                    entry(0, 2, -1),
                    entry(1, 3, 1),
                    entry(1, 5, -1),
                ],
                v: 2,
                p: 6,
            },
            port_labels: vec![
                "0:port".into(),
                "1:top".into(),
                "1:right".into(),
                "1:bottom".into(),
                "1:left".into(),
                "2:port".into(),
            ],
        }
    }

    #[test]
    fn bond_graph_simple_circuit_passes() {
        let resp = verify_graph_core(bond_graph_simple_se_r());
        assert!(resp.error.is_none(), "unexpected error: {:?}", resp.error);
        for r in &resp.results {
            assert!(r.passed, "example '{}' failed: {:?}", r.name, r.error);
        }
        assert!(resp.success);
    }

    #[test]
    fn bond_graph_preamble_has_refined_types() {
        let preamble = build_graph_preamble(&bond_graph_simple_se_r(), "");
        assert!(
            preamble.contains("TYPE_EffortSource"),
            "missing TYPE_EffortSource"
        );
        assert!(
            preamble.contains("TYPE_Junction1"),
            "missing TYPE_Junction1"
        );
        assert!(preamble.contains("TYPE_Resistor"), "missing TYPE_Resistor");
    }

    #[test]
    fn preamble_skips_theory_declared_type_operations() {
        let theory = "operation TYPE_Resistor : ℤ\noperation TYPE_Junction1 : ℤ\n";
        let preamble = build_graph_preamble(&bond_graph_simple_se_r(), theory);
        assert!(
            preamble.contains("axiom type_Resistor"),
            "should still emit axiom for Resistor"
        );
        assert!(
            !preamble.contains("operation TYPE_Resistor"),
            "should NOT re-declare theory-declared TYPE_Resistor"
        );
        assert!(
            !preamble.contains("operation TYPE_Junction1"),
            "should NOT re-declare theory-declared TYPE_Junction1"
        );
        assert!(
            preamble.contains("operation TYPE_EffortSource"),
            "should declare EffortSource (not in theory)"
        );
    }

    #[test]
    fn bond_graph_effort_conflict_fails() {
        // Two Se on the same 0-junction — effort conflict
        let req = VerifyGraphRequest {
            domain: "bond_graph".to_string(),
            components: vec![
                comp("bg_effort_source", "EffortSource", vec![]),
                comp("bg_effort_source", "EffortSource", vec![]),
                comp("bg_junction_0", "Junction0", vec![]),
                comp("bg_resistor", "Resistor", vec![]),
            ],
            incidence: VerifyGraphIncidence {
                entries: vec![
                    entry(0, 0, 1),
                    entry(0, 2, -1),
                    entry(1, 1, 1),
                    entry(1, 3, -1),
                    entry(2, 4, 1),
                    entry(2, 7, -1),
                ],
                v: 3,
                p: 8,
            },
            port_labels: vec![
                "0:port".into(),
                "1:port".into(),
                "2:top".into(),
                "2:right".into(),
                "2:bottom".into(),
                "2:left".into(),
                "3:port".into(),
                "3:port".into(),
            ],
        };
        let resp = verify_graph_core(req);
        assert!(resp.error.is_none(), "unexpected error: {:?}", resp.error);
        let conflict = resp
            .results
            .iter()
            .find(|r| r.name.contains("EFFORT CONFLICT"));
        assert!(
            conflict.is_some(),
            "expected EFFORT CONFLICT check in results"
        );
        assert!(
            !conflict.unwrap().passed,
            "EFFORT CONFLICT should fail with two Se on same 0-junction"
        );
    }

    #[test]
    fn bond_graph_rlc_with_junctions_passes() {
        // Se --[n0]--> 1-junc --[n1]--> 0-junc --[n2]--> R
        //                                  |---[n3]--> C
        //                                  |---[n4]--> I
        let req = VerifyGraphRequest {
            domain: "bond_graph".to_string(),
            components: vec![
                comp("bg_effort_source", "EffortSource", vec![]),
                comp("bg_junction_1", "Junction1", vec![]),
                comp("bg_junction_0", "Junction0", vec![]),
                comp("bg_resistor", "Resistor", vec![]),
                comp("bg_capacitor", "Capacitor", vec![]),
                comp("bg_inertia", "Inertia", vec![]),
            ],
            incidence: VerifyGraphIncidence {
                entries: vec![
                    entry(0, 0, 1),
                    entry(0, 1, -1),
                    entry(1, 2, 1),
                    entry(1, 5, -1),
                    entry(2, 6, 1),
                    entry(2, 9, -1),
                    entry(3, 7, 1),
                    entry(3, 10, -1),
                    entry(4, 8, 1),
                    entry(4, 11, -1),
                ],
                v: 5,
                p: 12,
            },
            port_labels: vec![
                "0:port".into(),
                "1:top".into(),
                "1:right".into(),
                "1:bottom".into(),
                "1:left".into(),
                "2:top".into(),
                "2:right".into(),
                "2:bottom".into(),
                "2:left".into(),
                "3:port".into(),
                "4:port".into(),
                "5:port".into(),
            ],
        };
        let resp = verify_graph_core(req);
        assert!(resp.error.is_none(), "unexpected error: {:?}", resp.error);
        for r in &resp.results {
            assert!(r.passed, "example '{}' failed: {:?}", r.name, r.error);
        }
        assert!(resp.success);
    }

    #[test]
    fn bond_graph_no_load_fails() {
        // Two Se + junctions but no R/C/I — should fail "1-PORT EXISTS"
        let req = VerifyGraphRequest {
            domain: "bond_graph".to_string(),
            components: vec![
                comp("bg_effort_source", "EffortSource", vec![]),
                comp("bg_junction_1", "Junction1", vec![]),
                comp("bg_effort_source", "EffortSource", vec![]),
                comp("bg_junction_0", "Junction0", vec![]),
            ],
            incidence: VerifyGraphIncidence {
                entries: vec![
                    entry(0, 0, 1),
                    entry(0, 4, -1),
                    entry(1, 5, 1),
                    entry(1, 9, -1),
                    entry(2, 3, 1),
                    entry(2, 6, -1),
                ],
                v: 3,
                p: 10,
            },
            port_labels: vec![
                "0:port".into(),
                "1:top".into(),
                "1:right".into(),
                "1:bottom".into(),
                "1:left".into(),
                "2:port".into(),
                "3:top".into(),
                "3:right".into(),
                "3:bottom".into(),
                "3:left".into(),
            ],
        };
        let resp = verify_graph_core(req);
        assert!(resp.error.is_none(), "unexpected error: {:?}", resp.error);
        let load_check = resp
            .results
            .iter()
            .find(|r| r.name.contains("1-PORT EXISTS"));
        assert!(load_check.is_some(), "expected 1-PORT EXISTS check");
        assert!(
            !load_check.unwrap().passed,
            "1-PORT EXISTS should fail when no R/C/I present"
        );
    }

    // =========================================================================
    // Electronics verification tests
    // =========================================================================

    fn electronics_simple_circuit() -> VerifyGraphRequest {
        // dc_voltage + resistor + ground: minimal valid circuit
        VerifyGraphRequest {
            domain: "electronics".to_string(),
            components: vec![
                comp(
                    "dc_voltage",
                    "VoltageSource",
                    vec![("V", serde_json::json!(5))],
                ),
                comp("resistor", "Passive", vec![("R", serde_json::json!(1000))]),
                comp("ground", "Ground", vec![]),
            ],
            incidence: VerifyGraphIncidence {
                entries: vec![
                    entry(0, 0, 1),
                    entry(0, 2, -1),
                    entry(1, 3, 1),
                    entry(1, 1, -1),
                    entry(1, 4, -1),
                ],
                v: 2,
                p: 5,
            },
            port_labels: vec![
                "0:pos".into(),
                "0:neg".into(),
                "1:left".into(),
                "1:right".into(),
                "2:pin".into(),
            ],
        }
    }

    #[test]
    fn electronics_simple_circuit_passes() {
        let resp = verify_graph_core(electronics_simple_circuit());
        assert!(resp.error.is_none(), "unexpected error: {:?}", resp.error);
        for r in &resp.results {
            assert!(r.passed, "example '{}' failed: {:?}", r.name, r.error);
        }
        assert!(resp.success);
    }

    #[test]
    fn electronics_preamble_has_refined_types() {
        let preamble = build_graph_preamble(&electronics_simple_circuit(), "");
        assert!(
            preamble.contains("TYPE_VoltageSource"),
            "missing TYPE_VoltageSource"
        );
        assert!(preamble.contains("TYPE_Passive"), "missing TYPE_Passive");
        assert!(preamble.contains("TYPE_Ground"), "missing TYPE_Ground");
    }

    #[test]
    fn electronics_no_ground_fails() {
        let req = VerifyGraphRequest {
            domain: "electronics".to_string(),
            components: vec![
                comp(
                    "dc_voltage",
                    "VoltageSource",
                    vec![("V", serde_json::json!(5))],
                ),
                comp("resistor", "Passive", vec![("R", serde_json::json!(1000))]),
            ],
            incidence: VerifyGraphIncidence {
                entries: vec![
                    entry(0, 0, 1),
                    entry(0, 2, -1),
                    entry(1, 3, 1),
                    entry(1, 1, -1),
                ],
                v: 2,
                p: 4,
            },
            port_labels: vec![
                "0:pos".into(),
                "0:neg".into(),
                "1:left".into(),
                "1:right".into(),
            ],
        };
        let resp = verify_graph_core(req);
        assert!(resp.error.is_none(), "unexpected error: {:?}", resp.error);
        let ground_check = resp
            .results
            .iter()
            .find(|r| r.name.contains("GROUND EXISTS"));
        assert!(
            ground_check.is_some(),
            "expected GROUND EXISTS check in results"
        );
        assert!(
            !ground_check.unwrap().passed,
            "GROUND EXISTS should fail when no ground component"
        );
    }

    #[test]
    fn electronics_parallel_voltage_sources_fails() {
        // Two dc_voltage sharing both nets — effort conflict
        let req = VerifyGraphRequest {
            domain: "electronics".to_string(),
            components: vec![
                comp(
                    "dc_voltage",
                    "VoltageSource",
                    vec![("V", serde_json::json!(5))],
                ),
                comp(
                    "dc_voltage",
                    "VoltageSource",
                    vec![("V", serde_json::json!(12))],
                ),
                comp("resistor", "Passive", vec![("R", serde_json::json!(1000))]),
                comp("ground", "Ground", vec![]),
            ],
            incidence: VerifyGraphIncidence {
                entries: vec![
                    entry(0, 0, 1),
                    entry(0, 2, 1),
                    entry(0, 4, -1),
                    entry(1, 1, -1),
                    entry(1, 3, -1),
                    entry(1, 5, -1),
                    entry(1, 6, -1),
                ],
                v: 2,
                p: 7,
            },
            port_labels: vec![
                "0:pos".into(),
                "0:neg".into(),
                "1:pos".into(),
                "1:neg".into(),
                "2:left".into(),
                "2:right".into(),
                "3:pin".into(),
            ],
        };
        let resp = verify_graph_core(req);
        assert!(resp.error.is_none(), "unexpected error: {:?}", resp.error);
        let parallel = resp
            .results
            .iter()
            .find(|r| r.name.contains("PARALLEL VOLTAGE"));
        assert!(
            parallel.is_some(),
            "expected PARALLEL VOLTAGE SOURCES check"
        );
        assert!(
            !parallel.unwrap().passed,
            "PARALLEL VOLTAGE SOURCES should fail when two vsources share both nets"
        );
    }
}

// =============================================================================
// Tests for simulate_graph
// =============================================================================

#[cfg(test)]
mod simulate_graph_tests {
    use super::*;

    fn sim_comp(comp_type: &str, tokens: Option<i64>) -> VerifyGraphComponent {
        let params = tokens.map(|t| {
            let mut m = std::collections::HashMap::new();
            m.insert("tokens".to_string(), serde_json::json!(t));
            m
        });
        VerifyGraphComponent {
            comp_type: comp_type.to_string(),
            component_type: Some(comp_type.to_string()),
            params,
        }
    }

    /// Linear workflow: [Source(1)] -> |T0| -> [Place(0)] -> |T1| -> [Sink(0)]
    ///
    /// Net 0: Source(+1) -- T0(-1)
    /// Net 1: T0(+1) -- Place(-1)
    /// Net 2: Place(+1) -- T1(-1)
    /// Net 3: T1(+1) -- Sink(-1)
    fn linear_workflow() -> SimulateGraphRequest {
        SimulateGraphRequest {
            domain: "petri_net".to_string(),
            components: vec![
                sim_comp("SourcePlace", Some(1)), // c0
                sim_comp("Transition", None),     // c1 = T0
                sim_comp("Place", Some(0)),       // c2
                sim_comp("Transition", None),     // c3 = T1
                sim_comp("SinkPlace", Some(0)),   // c4
            ],
            incidence: VerifyGraphIncidence {
                v: 4,
                p: 5,
                entries: vec![
                    // Net 0: Source -> T0
                    VerifyGraphEntry {
                        net: 0,
                        port: 0,
                        value: 1,
                    }, // Source port
                    VerifyGraphEntry {
                        net: 0,
                        port: 1,
                        value: -1,
                    }, // T0 port
                    // Net 1: T0 -> Place
                    VerifyGraphEntry {
                        net: 1,
                        port: 1,
                        value: 1,
                    }, // T0 port
                    VerifyGraphEntry {
                        net: 1,
                        port: 2,
                        value: -1,
                    }, // Place port
                    // Net 2: Place -> T1
                    VerifyGraphEntry {
                        net: 2,
                        port: 2,
                        value: 1,
                    }, // Place port
                    VerifyGraphEntry {
                        net: 2,
                        port: 3,
                        value: -1,
                    }, // T1 port
                    // Net 3: T1 -> Sink
                    VerifyGraphEntry {
                        net: 3,
                        port: 3,
                        value: 1,
                    }, // T1 port
                    VerifyGraphEntry {
                        net: 3,
                        port: 4,
                        value: -1,
                    }, // Sink port
                ],
            },
            port_labels: vec![
                "0:out".to_string(),
                "1:in".to_string(),
                "2:in".to_string(),
                "3:in".to_string(),
                "4:in".to_string(),
            ],
            state: vec![1.0, 0.0, 0.0, 0.0, 0.0],
            action: SimulateAction::FindEnabled,
            last_fired: None,
            sim_mode: None,
            a_matrix: None,
            b_matrix: None,
            inputs: None,
            dt: None,
            chunk_size: None,
        }
    }

    #[test]
    fn find_enabled_linear_workflow() {
        let req = linear_workflow();
        let resp = simulate_graph_core(req);
        assert!(resp.error.is_none(), "error: {:?}", resp.error);
        assert_eq!(resp.enabled, vec![1], "only T0 should be enabled");
    }

    #[test]
    fn step_fires_first_enabled() {
        let mut req = linear_workflow();
        req.action = SimulateAction::Step;
        let resp = simulate_graph_core(req);
        assert!(resp.error.is_none(), "error: {:?}", resp.error);
        assert_eq!(resp.fired, Some(1), "T0 should fire");
        assert_eq!(resp.state[0], 0.0, "source should lose token");
        assert_eq!(resp.state[2], 1.0, "middle place should gain token");
    }

    #[test]
    fn run_completes_linear_workflow() {
        let mut req = linear_workflow();
        req.action = SimulateAction::Run {
            max_steps: Some(100),
        };
        let resp = simulate_graph_core(req);
        assert!(resp.error.is_none(), "error: {:?}", resp.error);
        assert!(resp.halted);
        assert_eq!(
            resp.halt_reason.as_deref(),
            Some("completed"),
            "should halt when token reaches sink"
        );
        assert_eq!(resp.state[4], 1.0, "sink should have the token");
        assert_eq!(resp.state[0], 0.0, "source should be empty");
        assert!(resp.time_series.is_some());
        let history = resp.time_series.unwrap();
        assert_eq!(history.len(), 2, "two firings: T0 then T1");
    }

    #[test]
    fn reset_returns_client_provided_state() {
        let mut req = linear_workflow();
        // Client sends initial state (computed from stateParam metadata)
        req.state = vec![1.0, 0.0, 0.0, 0.0, 0.0];
        req.action = SimulateAction::Reset;
        let resp = simulate_graph_core(req);
        assert!(resp.error.is_none(), "error: {:?}", resp.error);
        assert_eq!(resp.state[0], 1.0, "source should have initial token");
        assert_eq!(resp.state[2], 0.0, "place should be empty");
        assert!(
            !resp.enabled.is_empty(),
            "should detect enabled transitions"
        );
    }

    #[test]
    fn deadlock_detection() {
        let mut req = linear_workflow();
        req.state = vec![0.0, 0.0, 0.0, 0.0, 0.0]; // no tokens anywhere
        req.action = SimulateAction::Step;
        let resp = simulate_graph_core(req);
        assert!(resp.halted);
        assert_eq!(resp.halt_reason.as_deref(), Some("deadlock"));
        assert!(resp.enabled.is_empty());
    }

    /// Mutex: two transitions compete for same token
    /// [Place(1)] -> |T0| -> [Out0(0)]
    /// [Place(1)] -> |T1| -> [Out1(0)]
    /// (same source place feeds both transitions)
    #[test]
    fn mutex_only_one_fires() {
        let req = SimulateGraphRequest {
            domain: "petri_net".to_string(),
            components: vec![
                sim_comp("Place", Some(1)),   // c0 — shared resource
                sim_comp("Transition", None), // c1 = T0
                sim_comp("Transition", None), // c2 = T1
                sim_comp("Place", Some(0)),   // c3 = Out0
                sim_comp("Place", Some(0)),   // c4 = Out1
            ],
            incidence: VerifyGraphIncidence {
                v: 4,
                p: 5,
                entries: vec![
                    // Net 0: Place -> T0
                    VerifyGraphEntry {
                        net: 0,
                        port: 0,
                        value: 1,
                    },
                    VerifyGraphEntry {
                        net: 0,
                        port: 1,
                        value: -1,
                    },
                    // Net 1: T0 -> Out0
                    VerifyGraphEntry {
                        net: 1,
                        port: 1,
                        value: 1,
                    },
                    VerifyGraphEntry {
                        net: 1,
                        port: 3,
                        value: -1,
                    },
                    // Net 2: Place -> T1
                    VerifyGraphEntry {
                        net: 2,
                        port: 0,
                        value: 1,
                    },
                    VerifyGraphEntry {
                        net: 2,
                        port: 2,
                        value: -1,
                    },
                    // Net 3: T1 -> Out1
                    VerifyGraphEntry {
                        net: 3,
                        port: 2,
                        value: 1,
                    },
                    VerifyGraphEntry {
                        net: 3,
                        port: 4,
                        value: -1,
                    },
                ],
            },
            port_labels: vec![
                "0:out".to_string(),
                "1:in".to_string(),
                "2:in".to_string(),
                "3:in".to_string(),
                "4:in".to_string(),
            ],
            state: vec![1.0, 0.0, 0.0, 0.0, 0.0],
            action: SimulateAction::Run {
                max_steps: Some(10),
            },
            last_fired: None,
            sim_mode: None,
            a_matrix: None,
            b_matrix: None,
            inputs: None,
            dt: None,
            chunk_size: None,
        };
        let resp = simulate_graph_core(req);
        assert!(resp.error.is_none(), "error: {:?}", resp.error);
        // Only one transition should have fired (token consumed from shared place)
        let total_out = resp.state[3] + resp.state[4];
        assert_eq!(
            total_out, 1.0,
            "exactly one output place should have the token"
        );
        assert_eq!(resp.state[0], 0.0, "shared place should be empty");
    }

    #[test]
    fn max_steps_halts_infinite_loop() {
        // Self-loop: Place -> T -> Place (fires forever)
        let req = SimulateGraphRequest {
            domain: "petri_net".to_string(),
            components: vec![
                sim_comp("Place", Some(1)),   // c0
                sim_comp("Transition", None), // c1
            ],
            incidence: VerifyGraphIncidence {
                v: 2,
                p: 2,
                entries: vec![
                    // Net 0: Place -> T
                    VerifyGraphEntry {
                        net: 0,
                        port: 0,
                        value: 1,
                    },
                    VerifyGraphEntry {
                        net: 0,
                        port: 1,
                        value: -1,
                    },
                    // Net 1: T -> Place
                    VerifyGraphEntry {
                        net: 1,
                        port: 1,
                        value: 1,
                    },
                    VerifyGraphEntry {
                        net: 1,
                        port: 0,
                        value: -1,
                    },
                ],
            },
            port_labels: vec!["0:out".to_string(), "1:in".to_string()],
            state: vec![1.0, 0.0],
            action: SimulateAction::Run { max_steps: Some(5) },
            last_fired: None,
            sim_mode: None,
            a_matrix: None,
            b_matrix: None,
            inputs: None,
            dt: None,
            chunk_size: None,
        };
        let resp = simulate_graph_core(req);
        assert!(resp.error.is_none(), "error: {:?}", resp.error);
        assert!(resp.halted);
        assert_eq!(resp.halt_reason.as_deref(), Some("max_steps"));
        assert!(resp.time_series.unwrap().len() == 5);
    }

    /// User's actual graph layout: transitions before places.
    /// [Source(5)] -> |T0| -> [Place(0)] -> |T1| -> [Sink(0)]
    ///
    /// Component order (as placed in Graph Editor):
    ///   c0 = T0, c1 = T1, c2 = Place, c3 = Source(5), c4 = Sink(0)
    ///
    /// Nets (port-level, 4 ports per component):
    ///   n0: c0:right(+1) → c2:left(-1)    T0 outputs to Place
    ///   n1: c2:right(+1) → c1:left(-1)    Place outputs to T1
    ///   n2: c3:right(+1) → c0:left(-1)    Source outputs to T0
    ///   n3: c1:right(+1) → c4:left(-1)    T1 outputs to Sink
    fn user_layout_5() -> SimulateGraphRequest {
        SimulateGraphRequest {
            domain: "petri_net".to_string(),
            components: vec![
                sim_comp("Transition", None),     // c0 = T0
                sim_comp("Transition", None),     // c1 = T1
                sim_comp("Place", Some(0)),       // c2
                sim_comp("SourcePlace", Some(5)), // c3
                sim_comp("SinkPlace", Some(0)),   // c4
            ],
            incidence: VerifyGraphIncidence {
                v: 4,
                p: 20,
                entries: vec![
                    // n0: T0:right(port 1) +1, Place:left(port 11) -1
                    VerifyGraphEntry {
                        net: 0,
                        port: 1,
                        value: 1,
                    },
                    VerifyGraphEntry {
                        net: 0,
                        port: 11,
                        value: -1,
                    },
                    // n1: Place:right(port 9) +1, T1:left(port 7) -1
                    VerifyGraphEntry {
                        net: 1,
                        port: 9,
                        value: 1,
                    },
                    VerifyGraphEntry {
                        net: 1,
                        port: 7,
                        value: -1,
                    },
                    // n2: Source:right(port 13) +1, T0:left(port 3) -1
                    VerifyGraphEntry {
                        net: 2,
                        port: 13,
                        value: 1,
                    },
                    VerifyGraphEntry {
                        net: 2,
                        port: 3,
                        value: -1,
                    },
                    // n3: T1:right(port 5) +1, Sink:left(port 19) -1
                    VerifyGraphEntry {
                        net: 3,
                        port: 5,
                        value: 1,
                    },
                    VerifyGraphEntry {
                        net: 3,
                        port: 19,
                        value: -1,
                    },
                ],
            },
            port_labels: (0..5)
                .flat_map(|ci| {
                    ["top", "right", "bottom", "left"]
                        .iter()
                        .map(move |p| format!("{}:{}", ci, p))
                })
                .collect(),
            state: vec![0.0, 0.0, 0.0, 5.0, 0.0],
            action: SimulateAction::FindEnabled,
            last_fired: None,
            sim_mode: None,
            a_matrix: None,
            b_matrix: None,
            inputs: None,
            dt: None,
            chunk_size: None,
        }
    }

    #[test]
    fn user_layout_step_by_step_5_tokens() {
        // Step through the entire simulation one step at a time,
        // recording the full trace.
        let base = user_layout_5();
        let mut state = base.state.clone();
        let mut trace: Vec<(usize, Vec<f64>)> = Vec::new(); // (fired, state_after)

        for _ in 0..100 {
            let req = SimulateGraphRequest {
                state: state.clone(),
                action: SimulateAction::Step,
                ..user_layout_5()
            };
            let resp = simulate_graph_core(req);
            assert!(resp.error.is_none(), "no error");

            if resp.halted {
                state = resp.state;
                break;
            }
            let fired = resp.fired.expect("should fire something");
            state = resp.state.clone();
            trace.push((fired, state.clone()));
        }

        // Print trace for debugging
        println!("=== Step-by-step trace (5 tokens) ===");
        println!("Components: c0=T0, c1=T1, c2=Place, c3=Source, c4=Sink");
        println!("Initial:    T0=0  T1=0  Place=0  Source=5  Sink=0");
        for (i, (fired, st)) in trace.iter().enumerate() {
            let name = match *fired {
                0 => "T0",
                1 => "T1",
                _ => "?",
            };
            println!(
                "Step {:2}: {} fired → T0={} T1={} Place={} Source={} Sink={}",
                i + 1,
                name,
                st[0],
                st[1],
                st[2],
                st[3],
                st[4]
            );
        }
        println!(
            "Final: T0={} T1={} Place={} Source={} Sink={}",
            state[0], state[1], state[2], state[3], state[4]
        );

        // Verify all tokens are accounted for (conservation)
        let total: f64 = state.iter().sum();
        assert_eq!(total, 5.0, "token conservation: sum must be 5");
        assert!(state[4] > 0.0, "at least one token should reach sink");
    }

    #[test]
    fn user_layout_run_5_tokens() {
        // Run all at once — should produce identical final state
        let mut req = user_layout_5();
        req.action = SimulateAction::Run {
            max_steps: Some(100),
        };
        let resp = simulate_graph_core(req);
        assert!(resp.error.is_none());
        assert!(resp.halted);

        let state = &resp.state;
        println!("=== Run trace (5 tokens) ===");
        println!("Components: c0=T0, c1=T1, c2=Place, c3=Source, c4=Sink");
        if let Some(ref ts) = resp.time_series {
            for (i, sample) in ts.iter().enumerate() {
                let fired_name = match sample.fired {
                    Some(0) => "T0",
                    Some(1) => "T1",
                    _ => "?",
                };
                println!(
                    "Step {:2}: {} → T0={} T1={} Place={} Source={} Sink={}",
                    i + 1,
                    fired_name,
                    sample.state[0],
                    sample.state[1],
                    sample.state[2],
                    sample.state[3],
                    sample.state[4]
                );
            }
        }
        println!(
            "Final: T0={} T1={} Place={} Source={} Sink={}",
            state[0], state[1], state[2], state[3], state[4]
        );
        println!("Halt reason: {:?}", resp.halt_reason);
        println!("Steps: {:?}", resp.time_series.as_ref().map(|v| v.len()));

        let total: f64 = state.iter().sum();
        assert_eq!(total, 5.0, "token conservation: sum must be 5");
        assert!(state[4] > 0.0, "at least one token should reach sink");
        assert_eq!(resp.halt_reason.as_deref(), Some("completed"));
    }
}

// =============================================================================
// Tests for simulate_setup + continuous simulation
// =============================================================================

#[cfg(test)]
mod continuous_sim_tests {
    use super::*;

    fn bg_comp(comp_type: &str, params: &[(&str, f64)]) -> VerifyGraphComponent {
        let mut m = std::collections::HashMap::new();
        for (k, v) in params {
            m.insert(k.to_string(), serde_json::json!(v));
        }
        VerifyGraphComponent {
            comp_type: comp_type.to_string(),
            component_type: Some(comp_type.to_string()),
            params: if m.is_empty() { None } else { Some(m) },
        }
    }

    /// Se(10V) — 1-junction — R(100Ω) — C(1μF)
    ///
    /// Components:
    ///   c0: EffortSource (effort=10)
    ///   c1: Junction1
    ///   c2: Resistor (R=100)
    ///   c3: Capacitor (C=1e-6, initial=0)
    ///
    /// Nets (component-level):
    ///   n0: Se(c0) ↔ Junction1(c1)
    ///   n1: Junction1(c1) ↔ R(c2)
    ///   n2: Junction1(c1) ↔ C(c3)
    fn rc_circuit_setup_request() -> SimulateSetupRequest {
        SimulateSetupRequest {
            domain: "bond_graph".to_string(),
            components: vec![
                bg_comp("EffortSource", &[("effort", 10.0)]),
                bg_comp("Junction1", &[]),
                bg_comp("Resistor", &[("R", 100.0)]),
                bg_comp("Capacitor", &[("C", 1e-6), ("initial", 0.0)]),
            ],
            incidence: VerifyGraphIncidence {
                v: 3,
                p: 4,
                entries: vec![
                    // n0: Se(c0) +1, Junction1(c1) -1
                    VerifyGraphEntry {
                        net: 0,
                        port: 0,
                        value: 1,
                    },
                    VerifyGraphEntry {
                        net: 0,
                        port: 1,
                        value: -1,
                    },
                    // n1: Junction1(c1) +1, R(c2) -1
                    VerifyGraphEntry {
                        net: 1,
                        port: 1,
                        value: 1,
                    },
                    VerifyGraphEntry {
                        net: 1,
                        port: 2,
                        value: -1,
                    },
                    // n2: Junction1(c1) +1, C(c3) -1
                    VerifyGraphEntry {
                        net: 2,
                        port: 1,
                        value: 1,
                    },
                    VerifyGraphEntry {
                        net: 2,
                        port: 3,
                        value: -1,
                    },
                ],
            },
            port_labels: vec![
                "0:port".to_string(),
                "1:port".to_string(),
                "2:port".to_string(),
                "3:port".to_string(),
            ],
        }
    }

    #[test]
    fn setup_detects_continuous_mode() {
        let resp = simulate_setup_core(rc_circuit_setup_request());
        assert_eq!(
            resp.sim_mode, "continuous",
            "bond_graph should be continuous"
        );
        if let Some(e) = &resp.error {
            println!("Setup error: {}", e);
        }
    }

    #[test]
    fn setup_extracts_dimensions() {
        let resp = simulate_setup_core(rc_circuit_setup_request());
        assert!(resp.error.is_none(), "error: {:?}", resp.error);
        assert_eq!(resp.state_map.len(), 1, "1 state variable (C)");
        assert_eq!(resp.input_map.len(), 1, "1 input (Se)");
        assert_eq!(resp.state_map[0], 3, "state[0] → component 3 (Capacitor)");
        assert_eq!(resp.input_map[0], 0, "input[0] → component 0 (Se)");
        assert_eq!(resp.initial_state, vec![0.0], "C starts at 0V");
    }

    #[test]
    fn setup_extracts_ab_matrices() {
        let resp = simulate_setup_core(rc_circuit_setup_request());
        assert!(resp.error.is_none(), "error: {:?}", resp.error);

        let a = resp.a_matrix.expect("A matrix should be present");
        let b = resp.b_matrix.expect("B matrix should be present");

        // RC circuit: A = [-1/(RC)], B = [1/(RC)]
        // R=100, C=1e-6 → RC = 1e-4 → 1/RC = 10000
        let rc_inv = 1.0 / (100.0 * 1e-6); // 10000
        let tol = 1.0; // Z3 may have small precision differences

        println!("A = {:?}", a);
        println!("B = {:?}", b);
        println!("Expected: A=[[-{}]], B=[[{}]]", rc_inv, rc_inv);

        assert_eq!(a.len(), 1, "A is 1×1");
        assert_eq!(a[0].len(), 1, "A[0] is 1-element");
        assert!(
            (a[0][0] - (-rc_inv)).abs() < tol,
            "A[0][0] = {} ≠ {}",
            a[0][0],
            -rc_inv
        );

        assert_eq!(b.len(), 1, "B is 1×1");
        assert_eq!(b[0].len(), 1, "B[0] is 1-element");
        assert!(
            (b[0][0] - rc_inv).abs() < tol,
            "B[0][0] = {} ≠ {}",
            b[0][0],
            rc_inv
        );
    }

    #[test]
    fn continuous_trajectory_approaches_steady_state() {
        let setup = simulate_setup_core(rc_circuit_setup_request());
        assert!(setup.error.is_none(), "setup error: {:?}", setup.error);

        let a = setup.a_matrix.unwrap();
        let b = setup.b_matrix.unwrap();
        let dt = setup.dt;

        // Run 1000 integration steps
        let req = SimulateGraphRequest {
            domain: "bond_graph".to_string(),
            components: vec![
                bg_comp("EffortSource", &[("effort", 10.0)]),
                bg_comp("Junction1", &[]),
                bg_comp("Resistor", &[("R", 100.0)]),
                bg_comp("Capacitor", &[("C", 1e-6), ("initial", 0.0)]),
            ],
            incidence: VerifyGraphIncidence {
                v: 3,
                p: 4,
                entries: vec![
                    VerifyGraphEntry {
                        net: 0,
                        port: 0,
                        value: 1,
                    },
                    VerifyGraphEntry {
                        net: 0,
                        port: 1,
                        value: -1,
                    },
                    VerifyGraphEntry {
                        net: 1,
                        port: 1,
                        value: 1,
                    },
                    VerifyGraphEntry {
                        net: 1,
                        port: 2,
                        value: -1,
                    },
                    VerifyGraphEntry {
                        net: 2,
                        port: 1,
                        value: 1,
                    },
                    VerifyGraphEntry {
                        net: 2,
                        port: 3,
                        value: -1,
                    },
                ],
            },
            port_labels: vec![
                "0:port".to_string(),
                "1:port".to_string(),
                "2:port".to_string(),
                "3:port".to_string(),
            ],
            state: setup.initial_state.clone(),
            action: SimulateAction::Step,
            last_fired: None,
            sim_mode: Some("continuous".to_string()),
            a_matrix: Some(a),
            b_matrix: Some(b),
            inputs: Some(setup.input_values.clone()),
            dt: Some(dt),
            chunk_size: Some(1000),
        };

        let resp = simulate_graph_core(req);
        assert!(resp.error.is_none(), "error: {:?}", resp.error);

        let ts = resp.time_series.expect("should have trajectory");
        assert_eq!(ts.len(), 1000, "1000 steps");

        // Capacitor voltage should be moving toward 10V
        let final_v = resp.state[0];
        println!(
            "After 1000 steps (dt={}): V_C = {:.6}V (target: 10V)",
            dt, final_v
        );
        assert!(final_v > 0.0, "voltage should be positive");
        assert!(
            final_v <= 10.0 + 1e-6,
            "voltage should not significantly exceed source: {}",
            final_v
        );
    }

    #[test]
    fn rk4_accuracy_with_large_dt() {
        // RC circuit: V(t) = V_s(1 - e^{-t/RC}), RC = 100 * 1e-6 = 1e-4
        // At t=5RC = 5e-4s, V = 10*(1 - e^{-5}) ≈ 9.9326V
        let a = vec![vec![-10000.0]]; // -1/RC
        let b = vec![vec![10000.0]]; // 1/RC
        let u = vec![10.0];
        let dt = 0.00005; // 50us — 5x larger than default 0.0001

        let mut x = vec![0.0];
        let n_steps = 10; // 10 steps * 50us = 500us = 5RC
        for _ in 0..n_steps {
            x = rk4_step(&a, &b, &x, &u, dt);
        }

        let expected = 10.0 * (1.0 - (-5.0_f64).exp()); // ~9.9326
        let err = (x[0] - expected).abs();
        println!(
            "RK4 large dt: V={:.8}, expected={:.8}, error={:.2e}",
            x[0], expected, err
        );
        assert!(
            err < 1e-3,
            "RK4 with dt=50us should be accurate to <0.1%: err={}",
            err
        );
    }

    #[test]
    fn rk4_stability_at_nyquist_limit() {
        // RK4 stability region for real negative eigenvalues: |λ*dt| < 2.785
        // λ = -10000, dt = 0.0002 → |λ*dt| = 2.0 (within stability)
        let a = vec![vec![-10000.0]];
        let b = vec![vec![10000.0]];
        let u = vec![10.0];
        let dt = 0.0002;

        let mut x = vec![0.0];
        for _ in 0..50 {
            x = rk4_step(&a, &b, &x, &u, dt);
        }

        // Should converge, not blow up
        assert!(x[0].is_finite(), "RK4 should remain stable at |λ*dt|=2.0");
        assert!(x[0] > 9.0, "should approach 10V: got {}", x[0]);
        assert!(
            x[0] < 10.1,
            "should not overshoot significantly: got {}",
            x[0]
        );
    }

    /// Se(1V) — 1-junction — R(1Ω) — (0-junction — C(1F) — I(1H))
    ///
    /// Components (matching user's graph editor layout):
    ///   c0: Junction0       (0-junction)
    ///   c1: Capacitor       (C=1, initial=0)
    ///   c2: Inertia         (I=1)
    ///   c3: EffortSource    (V=1)
    ///   c4: Resistor        (R=1)
    ///   c5: Junction1       (1-junction)
    ///
    /// Ports (12):
    ///   p0..p3:  0:top, 0:right, 0:bottom, 0:left   (Junction0)
    ///   p4:      1:port                               (C)
    ///   p5:      2:port                               (I)
    ///   p6:      3:port                               (Se)
    ///   p7:      4:port                               (R)
    ///   p8..p11: 5:top, 5:right, 5:bottom, 5:left   (Junction1)
    ///
    /// Nets (from user's incidence matrix):
    ///   n0: 0:right(+1), I:port(-1)        — 0-junction ↔ I
    ///   n1: 0:left(+1),  C:port(-1)        — 0-junction ↔ C
    ///   n2: Se:port(+1), 1:left(-1)        — Se ↔ 1-junction
    ///   n3: R:port(-1),  1:right(+1)       — 1-junction ↔ R
    ///   n4: 0:top(-1),   1:bottom(+1)      — 1-junction ↔ 0-junction
    fn rlc_circuit_setup_request() -> SimulateSetupRequest {
        SimulateSetupRequest {
            domain: "bond_graph".to_string(),
            components: vec![
                bg_comp("Junction0", &[]),
                bg_comp("Capacitor", &[("C", 1.0), ("initial", 0.0)]),
                bg_comp("Inertia", &[("I", 1.0)]),
                bg_comp("EffortSource", &[("effort", 1.0)]),
                bg_comp("Resistor", &[("R", 1.0)]),
                bg_comp("Junction1", &[]),
            ],
            incidence: VerifyGraphIncidence {
                v: 5,
                p: 12,
                entries: vec![
                    // n0: 0:right(+1), I:port(-1)
                    VerifyGraphEntry {
                        net: 0,
                        port: 1,
                        value: 1,
                    },
                    VerifyGraphEntry {
                        net: 0,
                        port: 5,
                        value: -1,
                    },
                    // n1: 0:left(+1), C:port(-1)
                    VerifyGraphEntry {
                        net: 1,
                        port: 3,
                        value: 1,
                    },
                    VerifyGraphEntry {
                        net: 1,
                        port: 4,
                        value: -1,
                    },
                    // n2: Se:port(+1), 1:left(-1)
                    VerifyGraphEntry {
                        net: 2,
                        port: 6,
                        value: 1,
                    },
                    VerifyGraphEntry {
                        net: 2,
                        port: 11,
                        value: -1,
                    },
                    // n3: R:port(-1), 1:right(+1)
                    VerifyGraphEntry {
                        net: 3,
                        port: 7,
                        value: -1,
                    },
                    VerifyGraphEntry {
                        net: 3,
                        port: 9,
                        value: 1,
                    },
                    // n4: 0:top(-1), 1:bottom(+1)
                    VerifyGraphEntry {
                        net: 4,
                        port: 0,
                        value: -1,
                    },
                    VerifyGraphEntry {
                        net: 4,
                        port: 10,
                        value: 1,
                    },
                ],
            },
            port_labels: vec![
                "0:top".into(),
                "0:right".into(),
                "0:bottom".into(),
                "0:left".into(),
                "1:port".into(),
                "2:port".into(),
                "3:port".into(),
                "4:port".into(),
                "5:top".into(),
                "5:right".into(),
                "5:bottom".into(),
                "5:left".into(),
            ],
        }
    }

    #[test]
    fn setup_extracts_rlc_dimensions() {
        let resp = simulate_setup_core(rlc_circuit_setup_request());
        if let Some(e) = &resp.error {
            println!("Setup error: {}", e);
        }
        assert!(resp.error.is_none(), "error: {:?}", resp.error);
        assert_eq!(resp.state_map.len(), 2, "2 state variables (C and I)");
        assert_eq!(resp.input_map.len(), 1, "1 input (Se)");
    }

    #[test]
    fn setup_extracts_rlc_ab_matrices() {
        let resp = simulate_setup_core(rlc_circuit_setup_request());
        assert!(resp.error.is_none(), "error: {:?}", resp.error);

        let a = resp.a_matrix.expect("A matrix should be present");
        let b = resp.b_matrix.expect("B matrix should be present");

        println!("A = {:?}", a);
        println!("B = {:?}", b);
        println!("state_map = {:?}", resp.state_map);
        println!("input_map = {:?}", resp.input_map);

        assert_eq!(a.len(), 2, "A is 2×2");
        assert_eq!(a[0].len(), 2, "A row 0 has 2 cols");
        assert_eq!(a[1].len(), 2, "A row 1 has 2 cols");
        assert_eq!(b.len(), 2, "B is 2×1");
        assert_eq!(b[0].len(), 1, "B row 0 has 1 col");
        assert_eq!(b[1].len(), 1, "B row 1 has 1 col");

        // Expected with R=C=I=1:
        //   A = [[-1, -1], [1, 0]]
        //   B = [[1], [0]]
        let tol = 0.1;
        assert!((a[0][0] - (-1.0)).abs() < tol, "A[0][0] = {} ≠ -1", a[0][0]);
        assert!((a[0][1] - (-1.0)).abs() < tol, "A[0][1] = {} ≠ -1", a[0][1]);
        assert!((a[1][0] - 1.0).abs() < tol, "A[1][0] = {} ≠ 1", a[1][0]);
        assert!((a[1][1] - 0.0).abs() < tol, "A[1][1] = {} ≠ 0", a[1][1]);

        assert!((b[0][0] - 1.0).abs() < tol, "B[0][0] = {} ≠ 1", b[0][0]);
        assert!((b[1][0] - 0.0).abs() < tol, "B[1][0] = {} ≠ 0", b[1][0]);
    }

    #[test]
    fn rlc_trajectory_oscillates() {
        // Hardcoded correct A/B for R=C=I=1 (underdamped RLC)
        // Eigenvalues: -0.5 ± j√(3)/2 → damped oscillation
        // Steady state: V_C = 0, I_L = V_s/R = 1 (inductor is DC short)
        let a = vec![vec![-1.0, -1.0], vec![1.0, 0.0]];
        let b = vec![vec![1.0], vec![0.0]];
        let u = vec![1.0];
        let dt = 0.01;

        let mut x = vec![0.0, 0.0]; // [V_C, I_L]
        let mut max_vc = 0.0_f64;
        let mut min_vc = 0.0_f64;
        for _ in 0..2000 {
            x = rk4_step(&a, &b, &x, &u, dt);
            max_vc = max_vc.max(x[0]);
            min_vc = min_vc.min(x[0]);
        }

        println!(
            "After 2000 steps: V_C={:.6}, I_L={:.6}, max_VC={:.6}, min_VC={:.6}",
            x[0], x[1], max_vc, min_vc
        );

        // V_C rises from 0, peaks, then oscillates back through 0 (goes negative)
        assert!(max_vc > 0.3, "V_C should peak above 0: max={}", max_vc);
        assert!(
            min_vc < -0.01,
            "V_C should go negative (oscillation): min={}",
            min_vc
        );
        // Steady state: V_C → 0, I_L → 1
        assert!(x[0].abs() < 0.01, "V_C should settle near 0: got {}", x[0]);
        assert!(
            (x[1] - 1.0).abs() < 0.01,
            "I_L should settle near 1A: got {}",
            x[1]
        );
    }
}
