use crate::ast::Expression;

use super::{Evaluator, eval_numeric};

impl Evaluator {
    pub(crate) fn apply_builtin(
        &self,
        name: &str,
        args: &[Expression],
    ) -> Result<Option<Expression>, String> {
        match name {
            "out" | "show" | "print" => {
                // out(expr) - pretty-prints the expression and returns it
                // Useful for exploring computed values in example blocks and Jupyter
                if args.len() != 1 {
                    return Err("out() takes exactly 1 argument".to_string());
                }
                let value = self.eval_concrete(&args[0])?;
                let formatted = self.pretty_print_value(&value);
                println!("{}", formatted);
                Ok(Some(value))
            }

            // diagram(options, plot(...), bar(...), scatter(...))
            //   → Combines elements and renders to SVG
            //
            // plot(xs, ys, options) → PlotElement
            // bar(xs, heights, options) → PlotElement
            // scatter(xs, ys, options) → PlotElement
            // etc.
            //
            "diagram" => self.builtin_diagram(args),
            "plot" => self.builtin_plot_element(args, crate::plotting::PlotType::Line),
            "scatter" => self.builtin_plot_element(args, crate::plotting::PlotType::Scatter),
            "bar" => self.builtin_plot_element(args, crate::plotting::PlotType::Bar),
            "hbar" => self.builtin_plot_element(args, crate::plotting::PlotType::HBar),
            "stem" => self.builtin_plot_element(args, crate::plotting::PlotType::Stem),
            "hstem" => self.builtin_plot_element(args, crate::plotting::PlotType::HStem),
            "fill_between" => self.builtin_fill_between_element(args),
            "stacked_area" => self.builtin_stacked_area(args),
            "boxplot" => self.builtin_boxplot_element(args, false),
            "hboxplot" => self.builtin_boxplot_element(args, true),
            "heatmap" | "colormesh" => {
                self.builtin_matrix_element(args, crate::plotting::PlotType::Colormesh)
            }
            "contour" => self.builtin_matrix_element(args, crate::plotting::PlotType::Contour),
            "quiver" => self.builtin_quiver_element(args),
            "place" => self.builtin_place_element(args),
            "yaxis" | "secondary_yaxis" => self.builtin_yaxis_element(args),
            "xaxis" | "secondary_xaxis" => self.builtin_xaxis_element(args),
            "path" => self.builtin_path_element(args),

            // Export Typst code (for embedding in documents)
            "export_typst" => self.builtin_export_typst(args),
            "export_typst_fragment" => self.builtin_export_typst_fragment(args),

            // Generate Typst table from Kleis data
            "table_typst" => self.builtin_table_typst(args),
            "table_typst_raw" => self.builtin_table_typst_raw(args),
            "typst_raw" => self.builtin_typst_raw(args),
            "concat" => self.builtin_concat(args),
            "str_eq" => self.builtin_str_eq(args),

            // Review context built-ins (thread-local, set by ReviewEngine)
            "review_intent" => {
                let intent = crate::review_mcp::engine::get_review_intent();
                Ok(Some(Expression::String(intent)))
            }
            "review_path" => {
                let path = crate::review_mcp::engine::get_review_path();
                Ok(Some(Expression::String(path)))
            }

            // Render EditorNode AST to Typst (for equations in documents)
            "render_to_typst" => self.builtin_render_to_typst(args),

            "lighten" => {
                // lighten(color, amount) → "color.lighten(amount)"
                // For Typst color manipulation
                if args.len() != 2 {
                    return Ok(None);
                }
                let color = self.extract_string(&args[0])?;
                let amount = self.extract_string(&args[1])?;
                Ok(Some(Expression::String(format!(
                    "{}.lighten({})",
                    color, amount
                ))))
            }

            "plus" | "+" => self.builtin_arithmetic(args, |a, b| a + b),
            "minus" | "-" => self.builtin_arithmetic(args, |a, b| a - b),
            "negate" => {
                if args.len() != 1 {
                    return Ok(None);
                }
                if let Some(n) = self.as_number(&args[0]) {
                    Ok(Some(Self::const_from_f64(-n)))
                } else {
                    Ok(None)
                }
            }
            "times" | "*" | "mul" => self.builtin_arithmetic(args, |a, b| a * b),
            "divide" | "/" | "div" => {
                if args.len() != 2 {
                    return Ok(None);
                }
                if let (Some(a), Some(b)) = (self.as_number(&args[0]), self.as_number(&args[1])) {
                    if b == 0.0 {
                        return Err("Division by zero".to_string());
                    }
                    Ok(Some(Expression::Const(format!("{}", a / b))))
                } else {
                    Ok(None)
                }
            }
            "mod" | "%" => {
                if args.len() != 2 {
                    return Ok(None);
                }
                if let (Some(a), Some(b)) = (self.as_integer(&args[0]), self.as_integer(&args[1])) {
                    if b == 0 {
                        return Err("Modulo by zero".to_string());
                    }
                    Ok(Some(Expression::Const(format!("{}", a % b))))
                } else {
                    Ok(None)
                }
            }

            "eq" | "=" | "==" | "equals" => {
                if args.len() != 2 {
                    return Ok(None);
                }
                let result = self.values_equal(&args[0], &args[1]);
                Ok(Some(Expression::Object(
                    if result { "true" } else { "false" }.to_string(),
                )))
            }
            "neq" | "!=" | "≠" => {
                if args.len() != 2 {
                    return Ok(None);
                }
                let result = !self.values_equal(&args[0], &args[1]);
                Ok(Some(Expression::Object(
                    if result { "true" } else { "false" }.to_string(),
                )))
            }
            "lt" | "<" | "less_than" => self.builtin_comparison(args, |a, b| a < b),
            "le" | "<=" | "≤" | "leq" | "less_or_equal" => {
                self.builtin_comparison(args, |a, b| a <= b)
            }
            "gt" | ">" | "greater_than" => self.builtin_comparison(args, |a, b| a > b),
            "ge" | ">=" | "≥" | "geq" | "greater_or_equal" => {
                self.builtin_comparison(args, |a, b| a >= b)
            }

            "and" | "∧" | "logical_and" => {
                if args.len() != 2 {
                    return Ok(None);
                }
                let a = self.as_bool(&args[0]);
                let b = self.as_bool(&args[1]);
                match (a, b) {
                    (Some(a), Some(b)) => Ok(Some(Expression::Object(
                        if a && b { "true" } else { "false" }.to_string(),
                    ))),
                    _ => Ok(None),
                }
            }
            "or" | "∨" | "logical_or" => {
                if args.len() != 2 {
                    return Ok(None);
                }
                let a = self.as_bool(&args[0]);
                let b = self.as_bool(&args[1]);
                match (a, b) {
                    (Some(a), Some(b)) => Ok(Some(Expression::Object(
                        if a || b { "true" } else { "false" }.to_string(),
                    ))),
                    _ => Ok(None),
                }
            }
            "not" | "¬" | "logical_not" => {
                if args.len() != 1 {
                    return Ok(None);
                }
                if let Some(a) = self.as_bool(&args[0]) {
                    Ok(Some(Expression::Object(
                        if !a { "true" } else { "false" }.to_string(),
                    )))
                } else {
                    Ok(None)
                }
            }

            "strlen" => {
                if args.len() != 1 {
                    return Ok(None);
                }
                if let Some(s) = self.as_string(&args[0]) {
                    Ok(Some(Expression::Const(format!("{}", s.len()))))
                } else {
                    Ok(None)
                }
            }
            "hasPrefix" => {
                if args.len() != 2 {
                    return Ok(None);
                }
                if let (Some(s), Some(prefix)) =
                    (self.as_string(&args[0]), self.as_string(&args[1]))
                {
                    Ok(Some(Expression::Object(
                        if s.starts_with(&prefix) {
                            "true"
                        } else {
                            "false"
                        }
                        .to_string(),
                    )))
                } else {
                    Ok(None)
                }
            }
            "hasSuffix" => {
                if args.len() != 2 {
                    return Ok(None);
                }
                if let (Some(s), Some(suffix)) =
                    (self.as_string(&args[0]), self.as_string(&args[1]))
                {
                    Ok(Some(Expression::Object(
                        if s.ends_with(&suffix) {
                            "true"
                        } else {
                            "false"
                        }
                        .to_string(),
                    )))
                } else {
                    Ok(None)
                }
            }
            "contains" => {
                if args.len() != 2 {
                    return Ok(None);
                }
                if let (Some(s), Some(sub)) = (self.as_string(&args[0]), self.as_string(&args[1])) {
                    Ok(Some(Expression::Object(
                        if s.contains(&sub) { "true" } else { "false" }.to_string(),
                    )))
                } else {
                    Ok(None)
                }
            }
            "indexOf" => {
                if args.len() != 2 {
                    return Ok(None);
                }
                if let (Some(s), Some(sub)) = (self.as_string(&args[0]), self.as_string(&args[1])) {
                    let idx = s.find(&sub).map(|i| i as i64).unwrap_or(-1);
                    Ok(Some(Expression::Const(format!("{}", idx))))
                } else {
                    Ok(None)
                }
            }
            "substr" | "substring" => {
                if args.len() != 3 {
                    return Ok(None);
                }
                if let (Some(s), Some(start), Some(len)) = (
                    self.as_string(&args[0]),
                    self.as_integer(&args[1]),
                    self.as_integer(&args[2]),
                ) {
                    let start = start.max(0) as usize;
                    let len = len.max(0) as usize;
                    let result: String = s.chars().skip(start).take(len).collect();
                    Ok(Some(Expression::String(result)))
                } else {
                    Ok(None)
                }
            }
            "charAt" => {
                if args.len() != 2 {
                    return Ok(None);
                }
                if let (Some(s), Some(idx)) = (self.as_string(&args[0]), self.as_integer(&args[1]))
                {
                    if idx >= 0 {
                        if let Some(ch) = s.chars().nth(idx as usize) {
                            Ok(Some(Expression::String(ch.to_string())))
                        } else {
                            Ok(Some(Expression::String(String::new())))
                        }
                    } else {
                        Ok(Some(Expression::String(String::new())))
                    }
                } else {
                    Ok(None)
                }
            }
            "replace" => {
                if args.len() != 3 {
                    return Ok(None);
                }
                if let (Some(s), Some(from), Some(to)) = (
                    self.as_string(&args[0]),
                    self.as_string(&args[1]),
                    self.as_string(&args[2]),
                ) {
                    Ok(Some(Expression::String(s.replacen(&from, &to, 1))))
                } else {
                    Ok(None)
                }
            }
            "replaceAll" => {
                if args.len() != 3 {
                    return Ok(None);
                }
                if let (Some(s), Some(from), Some(to)) = (
                    self.as_string(&args[0]),
                    self.as_string(&args[1]),
                    self.as_string(&args[2]),
                ) {
                    Ok(Some(Expression::String(s.replace(&from, &to))))
                } else {
                    Ok(None)
                }
            }
            "intToStr" | "int_to_str" | "fromInt" | "intToString" | "builtin_intToStr" => {
                // intToStr(42) → "42"
                if args.len() != 1 {
                    return Ok(None);
                }
                if let Some(n) = self.as_integer(&args[0]) {
                    Ok(Some(Expression::String(format!("{}", n))))
                } else if let Some(f) = self.as_number(&args[0]) {
                    // Handle floats by converting to integer first
                    Ok(Some(Expression::String(format!("{}", f as i64))))
                } else {
                    Ok(None)
                }
            }
            "strToInt" | "str_to_int" | "toInt" | "builtin_strToInt" => {
                // strToInt("42") → 42
                // strToInt("abc") → -1 (invalid)
                if args.len() != 1 {
                    return Ok(None);
                }
                if let Some(s) = self.as_string(&args[0]) {
                    match s.trim().parse::<i64>() {
                        Ok(n) => Ok(Some(Expression::Const(format!("{}", n)))),
                        Err(_) => Ok(Some(Expression::Const("-1".to_string()))),
                    }
                } else {
                    Ok(None)
                }
            }

            "splitLines" => {
                // splitLines("line1\nline2\nline3") → Cons("line1", Cons("line2", Cons("line3", Nil)))
                // Auto-detects newline format: real \n (0x0A) or escaped two-char \n
                if args.len() != 1 {
                    return Ok(None);
                }
                if let Some(s) = self.as_string(&args[0]) {
                    let lines: Vec<&str> = if s.contains('\n') {
                        s.split('\n').collect()
                    } else if s.contains("\\n") {
                        s.split("\\n").collect()
                    } else {
                        vec![&s]
                    };
                    let mut result = Expression::Object("Nil".to_string());
                    for line in lines.into_iter().rev() {
                        result = Expression::Operation {
                            name: "Cons".to_string(),
                            args: vec![Expression::String(line.to_string()), result],
                            span: None,
                        };
                    }
                    Ok(Some(result))
                } else {
                    Ok(None)
                }
            }
            "countLines" => {
                // countLines("a\nb\nc") → 3
                // O(n) scan, no allocation
                if args.len() != 1 {
                    return Ok(None);
                }
                if let Some(s) = self.as_string(&args[0]) {
                    if s.is_empty() {
                        return Ok(Some(Expression::Const("0".to_string())));
                    }
                    let count = if s.contains('\n') {
                        s.split('\n').count()
                    } else if s.contains("\\n") {
                        s.split("\\n").count()
                    } else {
                        1
                    };
                    Ok(Some(Expression::Const(format!("{}", count))))
                } else {
                    Ok(None)
                }
            }
            "nthLine" => {
                // nthLine("a\nb\nc", 1) → "b" (0-indexed)
                if args.len() != 2 {
                    return Ok(None);
                }
                if let (Some(s), Some(n)) = (self.as_string(&args[0]), self.as_integer(&args[1])) {
                    let lines: Vec<&str> = if s.contains('\n') {
                        s.split('\n').collect()
                    } else if s.contains("\\n") {
                        s.split("\\n").collect()
                    } else {
                        vec![&s]
                    };
                    let idx = n as usize;
                    if idx < lines.len() {
                        Ok(Some(Expression::String(lines[idx].to_string())))
                    } else {
                        Ok(Some(Expression::String(String::new())))
                    }
                } else {
                    Ok(None)
                }
            }
            "readFile" => {
                // readFile("path/to/file.rs") → file contents as string
                if args.len() != 1 {
                    return Ok(None);
                }
                if let Some(path) = self.as_string(&args[0]) {
                    match std::fs::read_to_string(&path) {
                        Ok(contents) => Ok(Some(Expression::String(contents))),
                        Err(e) => Err(format!("readFile: {}: {}", path, e)),
                    }
                } else {
                    Ok(None)
                }
            }
            "scan_python" => {
                if args.len() != 1 {
                    return Ok(None);
                }
                if let Some(source) = self.as_string(&args[0]) {
                    crate::python::scan_python(&source).map(Some)
                } else {
                    Ok(None)
                }
            }
            "scan_rust" => {
                if args.len() != 1 {
                    return Ok(None);
                }
                if let Some(source) = self.as_string(&args[0]) {
                    crate::rust_scanner::scan_rust(&source).map(Some)
                } else {
                    Ok(None)
                }
            }
            "trimRight" => {
                // trimRight("hello  ") → "hello"
                if args.len() != 1 {
                    return Ok(None);
                }
                if let Some(s) = self.as_string(&args[0]) {
                    Ok(Some(Expression::String(s.trim_end().to_string())))
                } else {
                    Ok(None)
                }
            }
            "trimLeft" => {
                // trimLeft("  hello") → "hello"
                if args.len() != 1 {
                    return Ok(None);
                }
                if let Some(s) = self.as_string(&args[0]) {
                    Ok(Some(Expression::String(s.trim_start().to_string())))
                } else {
                    Ok(None)
                }
            }
            "trim" => {
                // trim("  hello  ") → "hello"
                if args.len() != 1 {
                    return Ok(None);
                }
                if let Some(s) = self.as_string(&args[0]) {
                    Ok(Some(Expression::String(s.trim().to_string())))
                } else {
                    Ok(None)
                }
            }
            "foldLines" => {
                // foldLines(f, init, source) → iteratively apply f(acc, line) over lines
                // Replaces recursive scan_lines: no stack overflow, no Cons list.
                // f can be a lambda or a named 2-arg function.
                if args.len() != 3 {
                    return Ok(None);
                }
                let func = &args[0];
                let init = &args[1];
                if let Some(source) = self.as_string(&args[2]) {
                    let lines: Vec<&str> = if source.contains('\n') {
                        source.split('\n').collect()
                    } else if source.contains("\\n") {
                        source.split("\\n").collect()
                    } else {
                        vec![&*source]
                    };
                    let mut acc = init.clone();
                    for line in &lines {
                        let line_expr = Expression::String(line.to_string());
                        // Try applying as lambda via beta reduction
                        match func {
                            Expression::Object(fname) => {
                                // Named function: call fname(acc, line)
                                let call = Expression::Operation {
                                    name: fname.clone(),
                                    args: vec![line_expr, acc.clone()],
                                    span: None,
                                };
                                acc = self.eval_concrete(&call)?;
                            }
                            _ => {
                                // Lambda or other expression: beta reduce
                                let reduced =
                                    self.beta_reduce_multi(func, &[line_expr, acc.clone()])?;
                                acc = self.eval_concrete(&reduced)?;
                            }
                        }
                    }
                    Ok(Some(acc))
                } else {
                    Ok(None)
                }
            }

            "isAscii" => {
                if args.len() != 1 {
                    return Ok(None);
                }
                if let Some(s) = self.as_string(&args[0]) {
                    // ASCII printable: every char in 0x20..=0x7E (space through tilde)
                    let ok = s.chars().all(|c| (' '..='~').contains(&c));
                    Ok(Some(Expression::Object(
                        if ok { "true" } else { "false" }.to_string(),
                    )))
                } else {
                    Ok(None)
                }
            }
            "isDigits" => {
                if args.len() != 1 {
                    return Ok(None);
                }
                if let Some(s) = self.as_string(&args[0]) {
                    let ok = !s.is_empty() && s.chars().all(|c| c.is_ascii_digit());
                    Ok(Some(Expression::Object(
                        if ok { "true" } else { "false" }.to_string(),
                    )))
                } else {
                    Ok(None)
                }
            }
            "isAlpha" => {
                if args.len() != 1 {
                    return Ok(None);
                }
                if let Some(s) = self.as_string(&args[0]) {
                    let ok = !s.is_empty() && s.chars().all(|c| c.is_ascii_alphabetic());
                    Ok(Some(Expression::Object(
                        if ok { "true" } else { "false" }.to_string(),
                    )))
                } else {
                    Ok(None)
                }
            }
            "isAlphaNum" => {
                if args.len() != 1 {
                    return Ok(None);
                }
                if let Some(s) = self.as_string(&args[0]) {
                    let ok = !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric());
                    Ok(Some(Expression::Object(
                        if ok { "true" } else { "false" }.to_string(),
                    )))
                } else {
                    Ok(None)
                }
            }

            "Cons" | "cons" => {
                // Cons(head, tail) - construct a list
                if args.len() != 2 {
                    return Ok(None);
                }
                Ok(Some(Expression::Operation {
                    name: "Cons".to_string(),
                    args: args.to_vec(),
                    span: None,
                }))
            }
            "Nil" | "nil" => {
                // Nil - empty list
                Ok(Some(Expression::Object("Nil".to_string())))
            }
            "head" | "car" => {
                // head(Cons(h, t)) → h
                if args.len() != 1 {
                    return Ok(None);
                }
                match &args[0] {
                    Expression::Operation {
                        name, args: inner, ..
                    } if name == "Cons" && inner.len() == 2 => Ok(Some(inner[0].clone())),
                    Expression::List(elements) if !elements.is_empty() => {
                        Ok(Some(elements[0].clone()))
                    }
                    _ => Err("head: expected non-empty list".to_string()),
                }
            }
            "tail" | "cdr" => {
                // tail(Cons(h, t)) → t
                if args.len() != 1 {
                    return Ok(None);
                }
                match &args[0] {
                    Expression::Operation {
                        name, args: inner, ..
                    } if name == "Cons" && inner.len() == 2 => Ok(Some(inner[1].clone())),
                    Expression::List(elements) if !elements.is_empty() => {
                        Ok(Some(Expression::List(elements[1..].to_vec())))
                    }
                    _ => Err("tail: expected non-empty list".to_string()),
                }
            }
            "null?" | "isEmpty" | "isNil" | "builtin_isEmpty" => {
                // null?(list) → true if empty
                if args.len() != 1 {
                    return Ok(None);
                }
                let is_empty = match &args[0] {
                    Expression::Object(s) if s == "Nil" => true,
                    Expression::Operation { name, .. } if name == "Nil" => true,
                    Expression::List(elements) => elements.is_empty(),
                    Expression::Operation { name, .. } if name == "Cons" => false,
                    _ => return Ok(None),
                };
                Ok(Some(Expression::Object(
                    if is_empty { "true" } else { "false" }.to_string(),
                )))
            }
            "length" => {
                // length(list) → number of elements
                if args.len() != 1 {
                    return Ok(None);
                }
                match &args[0] {
                    Expression::List(elements) => {
                        Ok(Some(Expression::Const(format!("{}", elements.len()))))
                    }
                    Expression::Object(s) if s == "Nil" => {
                        Ok(Some(Expression::Const("0".to_string())))
                    }
                    Expression::Operation {
                        name, args: inner, ..
                    } if name == "Cons" => {
                        // Count recursively: 1 + length(tail)
                        let tail_len = self.apply_builtin("length", &[inner[1].clone()])?;
                        if let Some(Expression::Const(n)) = tail_len {
                            let len: i64 = n.parse().unwrap_or(0);
                            Ok(Some(Expression::Const(format!("{}", len + 1))))
                        } else {
                            Ok(None)
                        }
                    }
                    _ => Ok(None),
                }
            }
            "nth" | "index" => {
                // nth(list, index) → element at index (alias: index)
                if args.len() != 2 {
                    return Ok(None);
                }
                let idx = self.as_integer(&args[1]);
                match (&args[0], idx) {
                    (Expression::List(elements), Some(i))
                        if i >= 0 && (i as usize) < elements.len() =>
                    {
                        Ok(Some(elements[i as usize].clone()))
                    }
                    (
                        Expression::Operation {
                            name, args: inner, ..
                        },
                        Some(0),
                    ) if name == "Cons" => Ok(Some(inner[0].clone())),
                    (
                        Expression::Operation {
                            name, args: inner, ..
                        },
                        Some(i),
                    ) if name == "Cons" && i > 0 => self.apply_builtin(
                        "nth",
                        &[inner[1].clone(), Expression::Const(format!("{}", i - 1))],
                    ),
                    _ => Ok(None),
                }
            }
            "apply_lambda" => {
                // apply_lambda(lambda, arg1, arg2, ...) → result of applying lambda to args
                // This is used internally when a let-bound lambda is called
                if args.is_empty() {
                    return Ok(None);
                }
                let lambda = &args[0];
                let call_args = &args[1..];

                // Apply the lambda using beta reduction
                if let Expression::Lambda { .. } = lambda {
                    let reduced = self.beta_reduce_multi(lambda, call_args)?;
                    // Evaluate the result
                    let result = self.eval_concrete(&reduced)?;
                    return Ok(Some(result));
                }
                Ok(None)
            }
            "list_map" | "map" | "builtin_map" => {
                // list_map(f, [a, b, c]) → [f(a), f(b), f(c)]
                // Works with Expression::List (bracket lists)
                if args.len() != 2 {
                    return Ok(None);
                }
                let func = &args[0];

                // Evaluate the list argument first (e.g., linspace(0, 10, 5) → [0, 2.5, ...])
                let evaluated_list = self.eval_concrete(&args[1])?;

                // Handle Expression::List
                if let Expression::List(elements) = &evaluated_list {
                    let mut results = Vec::with_capacity(elements.len());
                    for elem in elements {
                        // Apply function using beta reduction
                        let reduced = self.beta_reduce(func, elem)?;
                        let result = self.eval_concrete(&reduced)?;
                        results.push(result);
                    }
                    return Ok(Some(Expression::List(results)));
                }

                // Also handle Cons/Nil lists for compatibility
                if let Expression::Object(s) = &evaluated_list
                    && s == "Nil"
                {
                    return Ok(Some(Expression::List(vec![])));
                }
                if let Expression::Operation {
                    name, args: inner, ..
                } = &evaluated_list
                {
                    if name == "Nil" {
                        return Ok(Some(Expression::List(vec![])));
                    }
                    if name == "Cons" && inner.len() == 2 {
                        // Recursively map over Cons list
                        let head = &inner[0];
                        let tail = &inner[1];

                        // Apply function to head using beta reduction
                        let reduced = self.beta_reduce(func, head)?;
                        let new_head = self.eval_concrete(&reduced)?;

                        // Recursively map over tail
                        let mapped_tail =
                            self.apply_builtin("list_map", &[func.clone(), tail.clone()])?;
                        if let Some(Expression::List(mut tail_elems)) = mapped_tail {
                            let mut result = vec![new_head];
                            result.append(&mut tail_elems);
                            return Ok(Some(Expression::List(result)));
                        }
                    }
                }

                Ok(None)
            }
            "list_filter" | "filter" | "builtin_filter" => {
                // list_filter(predicate, [a, b, c]) → elements where predicate(x) is true
                if args.len() != 2 {
                    return Ok(None);
                }
                let pred = &args[0];

                // Evaluate the list argument first
                let evaluated_list = self.eval_concrete(&args[1])?;

                if let Expression::List(elements) = &evaluated_list {
                    let mut results = Vec::new();
                    for elem in elements {
                        // Apply predicate using beta reduction
                        let reduced = self.beta_reduce(pred, elem)?;
                        let result = self.eval_concrete(&reduced)?;
                        // Check if result is truthy
                        if let Expression::Object(s) = &result {
                            if s == "true" || s == "True" {
                                results.push(elem.clone());
                            }
                        } else if let Expression::Const(s) = &result
                            && (s == "true" || s == "True")
                        {
                            results.push(elem.clone());
                        }
                    }
                    return Ok(Some(Expression::List(results)));
                }
                Ok(None)
            }
            "list_fold" | "foldl" | "builtin_foldl" => {
                // list_fold(f, init, [a, b, c]) → f(f(f(init, a), b), c)
                if args.len() != 3 {
                    return Ok(None);
                }
                let func = &args[0];
                let init = &args[1];

                // Evaluate the list argument first
                let evaluated_list = self.eval_concrete(&args[2])?;

                if let Expression::List(elements) = &evaluated_list {
                    let mut acc = init.clone();
                    for elem in elements {
                        // Apply function: acc = f(acc, elem) using beta reduction
                        let reduced = self.beta_reduce_multi(func, &[acc, elem.clone()])?;
                        acc = self.eval_concrete(&reduced)?;
                    }
                    return Ok(Some(acc));
                }
                Ok(None)
            }
            "list_flatmap" | "flatmap" | "concat_map" | "builtin_flatmap" => {
                // list_flatmap(f, [a, b, c]) → flatten(map(f, [a, b, c]))
                // f should return a list, results are concatenated
                if args.len() != 2 {
                    return Ok(None);
                }
                let func = &args[0];

                // Evaluate the list argument first
                let evaluated_list = self.eval_concrete(&args[1])?;

                if let Expression::List(elements) = &evaluated_list {
                    let mut result = Vec::new();
                    for elem in elements {
                        // Apply function to element
                        let reduced = self.beta_reduce_multi(func, std::slice::from_ref(elem))?;
                        let mapped = self.eval_concrete(&reduced)?;
                        // Flatten: if result is a list, extend; otherwise push
                        if let Expression::List(inner) = mapped {
                            result.extend(inner);
                        } else {
                            result.push(mapped);
                        }
                    }
                    return Ok(Some(Expression::List(result)));
                }
                Ok(None)
            }
            "list_zip" | "zip" | "builtin_zip" => {
                // list_zip([a, b, c], [1, 2, 3]) → [(a, 1), (b, 2), (c, 3)]
                // Returns pairs (tuples) of corresponding elements
                if args.len() != 2 {
                    return Ok(None);
                }

                // Evaluate both list arguments first
                let evaluated_xs = self.eval_concrete(&args[0])?;
                let evaluated_ys = self.eval_concrete(&args[1])?;

                if let (Expression::List(xs), Expression::List(ys)) = (&evaluated_xs, &evaluated_ys)
                {
                    let pairs: Vec<Expression> = xs
                        .iter()
                        .zip(ys.iter())
                        .map(|(x, y)| Expression::operation("Pair", vec![x.clone(), y.clone()]))
                        .collect();
                    return Ok(Some(Expression::List(pairs)));
                }
                Ok(None)
            }
            "fst" | "first" => {
                // fst(Pair(a, b)) → a
                if args.len() != 1 {
                    return Ok(None);
                }
                // Evaluate the argument first
                let evaluated = self.eval_concrete(&args[0])?;

                if let Expression::Operation {
                    name,
                    args: pair_args,
                    ..
                } = &evaluated
                    && name == "Pair"
                    && pair_args.len() == 2
                {
                    return Ok(Some(pair_args[0].clone()));
                }
                Ok(None)
            }
            "snd" | "second" => {
                // snd(Pair(a, b)) → b
                if args.len() != 1 {
                    return Ok(None);
                }
                // Evaluate the argument first
                let evaluated = self.eval_concrete(&args[0])?;

                if let Expression::Operation {
                    name,
                    args: pair_args,
                    ..
                } = &evaluated
                    && name == "Pair"
                    && pair_args.len() == 2
                {
                    return Ok(Some(pair_args[1].clone()));
                }
                Ok(None)
            }
            "list_nth" => {
                // list_nth([a, b, c], 1) → b
                // Index into a list (0-based)
                if args.len() != 2 {
                    return Ok(None);
                }

                // Evaluate the list argument first
                let evaluated_list = self.eval_concrete(&args[0])?;

                if let Expression::List(elements) = &evaluated_list
                    && let Some(idx) = self.as_number(&args[1])
                {
                    let idx = idx as usize;
                    if idx < elements.len() {
                        return Ok(Some(elements[idx].clone()));
                    }
                }
                Ok(None)
            }
            "list_length" => {
                // list_length([a, b, c]) → 3
                if args.len() != 1 {
                    return Ok(None);
                }

                // Evaluate the list argument first
                let evaluated_list = self.eval_concrete(&args[0])?;

                if let Expression::List(elements) = &evaluated_list {
                    return Ok(Some(Expression::Const(elements.len().to_string())));
                }
                Ok(None)
            }
            "list_concat" | "list_append" | "append" | "builtin_append" => {
                // list_concat([a, b], [c, d]) → [a, b, c, d]
                if args.len() != 2 {
                    return Ok(None);
                }
                // Evaluate both list arguments first
                let evaluated_xs = self.eval_concrete(&args[0])?;
                let evaluated_ys = self.eval_concrete(&args[1])?;

                if let (Expression::List(xs), Expression::List(ys)) = (&evaluated_xs, &evaluated_ys)
                {
                    let mut result = xs.clone();
                    result.extend(ys.clone());
                    return Ok(Some(Expression::List(result)));
                }
                Ok(None)
            }
            "list_flatten" | "list_join" => {
                // list_flatten([[a, b], [c, d]]) → [a, b, c, d]
                if args.len() != 1 {
                    return Ok(None);
                }
                // Evaluate the list argument first
                let evaluated_list = self.eval_concrete(&args[0])?;

                if let Expression::List(outer) = &evaluated_list {
                    let mut result = Vec::new();
                    for item in outer {
                        // Also evaluate each inner item in case it's unevaluated
                        let evaluated_item = self.eval_concrete(item)?;
                        if let Expression::List(inner) = evaluated_item {
                            result.extend(inner);
                        } else {
                            result.push(evaluated_item);
                        }
                    }
                    return Ok(Some(Expression::List(result)));
                }
                Ok(None)
            }
            "list_slice" => {
                // list_slice([a, b, c, d], 1, 3) → [b, c] (from index 1 up to but not including 3)
                if args.len() < 2 {
                    return Ok(None);
                }
                // Evaluate the list argument first
                let evaluated_list = self.eval_concrete(&args[0])?;

                if let Expression::List(elements) = &evaluated_list {
                    let start = if args.len() >= 2 {
                        self.extract_f64(&args[1]).unwrap_or(0.0) as usize
                    } else {
                        0
                    };
                    let end = if args.len() >= 3 {
                        self.extract_f64(&args[2]).unwrap_or(elements.len() as f64) as usize
                    } else {
                        elements.len()
                    };
                    let end = end.min(elements.len());
                    let start = start.min(end);
                    return Ok(Some(Expression::List(elements[start..end].to_vec())));
                }
                Ok(None)
            }
            "list_rotate" => {
                // list_rotate([a, b, c], 1) → [b, c, a] (rotate left by 1)
                if args.len() != 2 {
                    return Ok(None);
                }
                // Evaluate the list argument first
                let evaluated_list = self.eval_concrete(&args[0])?;

                if let Expression::List(elements) = &evaluated_list {
                    let n = self.extract_f64(&args[1]).unwrap_or(0.0) as usize;
                    if elements.is_empty() {
                        return Ok(Some(Expression::List(vec![])));
                    }
                    let n = n % elements.len();
                    let mut result = elements[n..].to_vec();
                    result.extend(elements[..n].to_vec());
                    return Ok(Some(Expression::List(result)));
                }
                Ok(None)
            }

            "reverse" | "builtin_reverse" => {
                // reverse([a, b, c]) → [c, b, a]
                if args.len() != 1 {
                    return Ok(None);
                }
                let evaluated_list = self.eval_concrete(&args[0])?;
                match &evaluated_list {
                    Expression::Object(s) if s == "Nil" => {
                        Ok(Some(Expression::Object("Nil".to_string())))
                    }
                    Expression::List(elements) => {
                        let mut reversed = elements.clone();
                        reversed.reverse();
                        Ok(Some(Expression::List(reversed)))
                    }
                    Expression::Operation {
                        name, args: inner, ..
                    } if name == "Cons" && inner.len() == 2 => {
                        // Convert Cons list to vec, reverse, return as List
                        let mut elements = vec![];
                        let mut current = evaluated_list.clone();
                        while let Expression::Operation {
                            name, args: inner, ..
                        } = &current
                        {
                            if name == "Cons" && inner.len() == 2 {
                                elements.push(inner[0].clone());
                                current = inner[1].clone();
                            } else {
                                break;
                            }
                        }
                        elements.reverse();
                        Ok(Some(Expression::List(elements)))
                    }
                    _ => Ok(None),
                }
            }

            "foldr" | "builtin_foldr" => {
                // foldr(f, z, [a, b, c]) → f(a, f(b, f(c, z)))
                if args.len() != 3 {
                    return Ok(None);
                }
                let func = &args[0];
                let z = &args[1];
                let evaluated_list = self.eval_concrete(&args[2])?;

                if let Expression::List(elements) = &evaluated_list {
                    let mut acc = z.clone();
                    for elem in elements.iter().rev() {
                        let reduced = self.beta_reduce_multi(func, &[elem.clone(), acc])?;
                        acc = self.eval_concrete(&reduced)?;
                    }
                    return Ok(Some(acc));
                }
                if let Expression::Object(s) = &evaluated_list
                    && s == "Nil"
                {
                    return Ok(Some(z.clone()));
                }
                Ok(None)
            }

            "sum" | "builtin_sum" => {
                // sum([1, 2, 3]) → 6
                if args.len() != 1 {
                    return Ok(None);
                }
                let evaluated_list = self.eval_concrete(&args[0])?;
                if let Expression::List(elements) = &evaluated_list {
                    let mut total = 0.0;
                    for e in elements {
                        if let Some(n) = self.as_number(e) {
                            total += n;
                        } else {
                            return Ok(None);
                        }
                    }
                    return Ok(Some(Self::const_from_f64(total)));
                }
                Ok(None)
            }

            "product" | "builtin_product" => {
                // product([2, 3, 4]) → 24
                if args.len() != 1 {
                    return Ok(None);
                }
                let evaluated_list = self.eval_concrete(&args[0])?;
                if let Expression::List(elements) = &evaluated_list {
                    let mut total = 1.0;
                    for e in elements {
                        if let Some(n) = self.as_number(e) {
                            total *= n;
                        } else {
                            return Ok(None);
                        }
                    }
                    return Ok(Some(Self::const_from_f64(total)));
                }
                Ok(None)
            }

            "all" | "builtin_all" => {
                // all(p, [a, b, c]) → true if p(x) is true for all x
                if args.len() != 2 {
                    return Ok(None);
                }
                let pred = &args[0];
                let evaluated_list = self.eval_concrete(&args[1])?;

                if let Expression::List(elements) = &evaluated_list {
                    for elem in elements {
                        let reduced = self.beta_reduce(pred, elem)?;
                        let result = self.eval_concrete(&reduced)?;
                        if let Some(false) = self.as_bool(&result) {
                            return Ok(Some(Expression::Object("false".to_string())));
                        }
                    }
                    return Ok(Some(Expression::Object("true".to_string())));
                }
                if let Expression::Object(s) = &evaluated_list
                    && s == "Nil"
                {
                    return Ok(Some(Expression::Object("true".to_string())));
                }
                Ok(None)
            }

            "any" | "builtin_any" => {
                // any(p, [a, b, c]) → true if p(x) is true for any x
                if args.len() != 2 {
                    return Ok(None);
                }
                let pred = &args[0];
                let evaluated_list = self.eval_concrete(&args[1])?;

                if let Expression::List(elements) = &evaluated_list {
                    for elem in elements {
                        let reduced = self.beta_reduce(pred, elem)?;
                        let result = self.eval_concrete(&reduced)?;
                        if let Some(true) = self.as_bool(&result) {
                            return Ok(Some(Expression::Object("true".to_string())));
                        }
                    }
                    return Ok(Some(Expression::Object("false".to_string())));
                }
                if let Expression::Object(s) = &evaluated_list
                    && s == "Nil"
                {
                    return Ok(Some(Expression::Object("false".to_string())));
                }
                Ok(None)
            }

            "range" => {
                // range(n) → [0, 1, 2, ..., n-1]
                // range(start, end) → [start, start+1, ..., end-1]
                if args.is_empty() {
                    return Ok(None);
                }
                let (start, end) = if args.len() == 1 {
                    (0, self.extract_f64(&args[0])? as i64)
                } else {
                    (
                        self.extract_f64(&args[0])? as i64,
                        self.extract_f64(&args[1])? as i64,
                    )
                };
                let result: Vec<Expression> = (start..end)
                    .map(|i| Expression::Const(i.to_string()))
                    .collect();
                Ok(Some(Expression::List(result)))
            }
            "linspace" => {
                // linspace(start, end) → 50 evenly spaced values (default)
                // linspace(start, end, count) → count evenly spaced values
                if args.len() < 2 {
                    return Err("linspace requires at least start and end".to_string());
                }
                let start = self.extract_f64(&args[0])?;
                let end = self.extract_f64(&args[1])?;
                let count = if args.len() >= 3 {
                    self.extract_f64(&args[2])? as usize
                } else {
                    50 // Default like numpy/Lilaq
                };
                if count == 0 {
                    return Ok(Some(Expression::List(vec![])));
                }
                if count == 1 {
                    return Ok(Some(Expression::List(vec![Expression::Const(
                        start.to_string(),
                    )])));
                }
                let step = (end - start) / (count - 1) as f64;
                let result: Vec<Expression> = (0..count)
                    .map(|i| Expression::Const((start + i as f64 * step).to_string()))
                    .collect();
                Ok(Some(Expression::List(result)))
            }
            "random" | "random_uniform" => {
                // random(count) → list of pseudo-random values in [0, 1]
                // random(count, seed) → reproducible random values
                // Uses a simple LCG for reproducibility
                if args.is_empty() {
                    return Err("random requires count".to_string());
                }
                let count = self.extract_f64(&args[0])? as usize;
                let seed = if args.len() >= 2 {
                    self.extract_f64(&args[1])? as u64
                } else {
                    42 // Default seed
                };
                // Simple LCG: x_{n+1} = (a * x_n + c) mod m
                let a: u64 = 1664525;
                let c: u64 = 1013904223;
                let m: u64 = 1 << 32;
                let mut x = seed;
                let result: Vec<Expression> = (0..count)
                    .map(|_| {
                        x = (a.wrapping_mul(x).wrapping_add(c)) % m;
                        Expression::Const((x as f64 / m as f64).to_string())
                    })
                    .collect();
                Ok(Some(Expression::List(result)))
            }
            "random_normal" => {
                // random_normal(count) → list of pseudo-random values from N(0, 1)
                // random_normal(count, seed) → reproducible
                // random_normal(count, seed, scale) → N(0, scale)
                // Uses Box-Muller transform
                if args.is_empty() {
                    return Err("random_normal requires count".to_string());
                }
                let count = self.extract_f64(&args[0])? as usize;
                let seed = if args.len() >= 2 {
                    self.extract_f64(&args[1])? as u64
                } else {
                    42
                };
                let scale = if args.len() >= 3 {
                    self.extract_f64(&args[2])?
                } else {
                    1.0
                };
                // Simple LCG
                let a: u64 = 1664525;
                let c: u64 = 1013904223;
                let m: u64 = 1 << 32;
                let mut x = seed;
                let mut uniform = || {
                    x = (a.wrapping_mul(x).wrapping_add(c)) % m;
                    (x as f64 / m as f64).max(1e-10) // Avoid log(0)
                };
                // Box-Muller transform
                let mut result: Vec<Expression> = Vec::with_capacity(count);
                while result.len() < count {
                    let u1 = uniform();
                    let u2 = uniform();
                    let z0 = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
                    let z1 = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).sin();
                    result.push(Expression::Const((z0 * scale).to_string()));
                    if result.len() < count {
                        result.push(Expression::Const((z1 * scale).to_string()));
                    }
                }
                Ok(Some(Expression::List(result)))
            }
            "vec_add" => {
                // Element-wise vector addition: vec_add([a, b], [c, d]) = [a+c, b+d]
                if args.len() != 2 {
                    return Err("vec_add requires two lists".to_string());
                }
                let list1 = self.extract_number_list_v2(&args[0])?;
                let list2 = self.extract_number_list_v2(&args[1])?;
                if list1.len() != list2.len() {
                    return Err("vec_add: lists must have same length".to_string());
                }
                let result: Vec<Expression> = list1
                    .iter()
                    .zip(list2.iter())
                    .map(|(a, b)| Expression::Const((a + b).to_string()))
                    .collect();
                Ok(Some(Expression::List(result)))
            }
            "cos" => {
                if args.len() != 1 {
                    return Ok(None);
                }
                let x = self.extract_f64(&args[0])?;
                Ok(Some(Expression::Const(x.cos().to_string())))
            }
            "sin" => {
                if args.len() != 1 {
                    return Ok(None);
                }
                let x = self.extract_f64(&args[0])?;
                Ok(Some(Expression::Const(x.sin().to_string())))
            }
            "sqrt" => {
                if args.len() != 1 {
                    return Ok(None);
                }
                let x = self.extract_f64(&args[0])?;
                Ok(Some(Expression::Const(x.sqrt().to_string())))
            }
            "pi" => Ok(Some(Expression::Const(std::f64::consts::PI.to_string()))),
            "deg_to_rad" | "radians" => {
                // Convert degrees to radians
                if args.len() != 1 {
                    return Ok(None);
                }
                let deg = self.extract_f64(&args[0])?;
                Ok(Some(Expression::Const(deg.to_radians().to_string())))
            }

            "tan" => {
                if args.len() != 1 {
                    return Ok(None);
                }
                let x = self.extract_f64(&args[0])?;
                Ok(Some(Expression::Const(x.tan().to_string())))
            }
            "asin" | "arcsin" => {
                if args.len() != 1 {
                    return Ok(None);
                }
                let x = self.extract_f64(&args[0])?;
                Ok(Some(Expression::Const(x.asin().to_string())))
            }
            "acos" | "arccos" => {
                if args.len() != 1 {
                    return Ok(None);
                }
                let x = self.extract_f64(&args[0])?;
                Ok(Some(Expression::Const(x.acos().to_string())))
            }
            "atan" | "arctan" => {
                if args.len() != 1 {
                    return Ok(None);
                }
                let x = self.extract_f64(&args[0])?;
                Ok(Some(Expression::Const(x.atan().to_string())))
            }
            "atan2" | "arctan2" => {
                // atan2(y, x) - 2-argument arctangent
                if args.len() != 2 {
                    return Ok(None);
                }
                let y = self.extract_f64(&args[0])?;
                let x = self.extract_f64(&args[1])?;
                Ok(Some(Expression::Const(y.atan2(x).to_string())))
            }

            // Hyperbolic functions
            "sinh" => {
                if args.len() != 1 {
                    return Ok(None);
                }
                let x = self.extract_f64(&args[0])?;
                Ok(Some(Expression::Const(x.sinh().to_string())))
            }
            "cosh" => {
                if args.len() != 1 {
                    return Ok(None);
                }
                let x = self.extract_f64(&args[0])?;
                Ok(Some(Expression::Const(x.cosh().to_string())))
            }
            "tanh" => {
                if args.len() != 1 {
                    return Ok(None);
                }
                let x = self.extract_f64(&args[0])?;
                Ok(Some(Expression::Const(x.tanh().to_string())))
            }
            "asinh" | "arcsinh" => {
                if args.len() != 1 {
                    return Ok(None);
                }
                let x = self.extract_f64(&args[0])?;
                Ok(Some(Expression::Const(x.asinh().to_string())))
            }
            "acosh" | "arccosh" => {
                if args.len() != 1 {
                    return Ok(None);
                }
                let x = self.extract_f64(&args[0])?;
                Ok(Some(Expression::Const(x.acosh().to_string())))
            }
            "atanh" | "arctanh" => {
                if args.len() != 1 {
                    return Ok(None);
                }
                let x = self.extract_f64(&args[0])?;
                Ok(Some(Expression::Const(x.atanh().to_string())))
            }

            // Exponential and logarithmic functions
            "exp" => {
                if args.len() != 1 {
                    return Ok(None);
                }
                let x = self.extract_f64(&args[0])?;
                Ok(Some(Expression::Const(x.exp().to_string())))
            }
            "exp2" => {
                // 2^x
                if args.len() != 1 {
                    return Ok(None);
                }
                let x = self.extract_f64(&args[0])?;
                Ok(Some(Expression::Const(x.exp2().to_string())))
            }
            "ln" | "log" => {
                // Natural logarithm (base e)
                if args.len() != 1 {
                    return Ok(None);
                }
                let x = self.extract_f64(&args[0])?;
                Ok(Some(Expression::Const(x.ln().to_string())))
            }
            "log10" => {
                // Base-10 logarithm
                if args.len() != 1 {
                    return Ok(None);
                }
                let x = self.extract_f64(&args[0])?;
                Ok(Some(Expression::Const(x.log10().to_string())))
            }
            "log2" => {
                // Base-2 logarithm
                if args.len() != 1 {
                    return Ok(None);
                }
                let x = self.extract_f64(&args[0])?;
                Ok(Some(Expression::Const(x.log2().to_string())))
            }
            "pow" | "power" => {
                // x^y
                if args.len() != 2 {
                    return Ok(None);
                }
                let x = self.extract_f64(&args[0])?;
                let y = self.extract_f64(&args[1])?;
                Ok(Some(Expression::Const(x.powf(y).to_string())))
            }

            // Utility functions
            "abs" | "fabs" => {
                if args.len() != 1 {
                    return Ok(None);
                }
                let x = self.extract_f64(&args[0])?;
                Ok(Some(Expression::Const(x.abs().to_string())))
            }
            "floor" => {
                if args.len() != 1 {
                    return Ok(None);
                }
                let x = self.extract_f64(&args[0])?;
                Ok(Some(Expression::Const(x.floor().to_string())))
            }
            "ceil" | "ceiling" => {
                if args.len() != 1 {
                    return Ok(None);
                }
                let x = self.extract_f64(&args[0])?;
                Ok(Some(Expression::Const(x.ceil().to_string())))
            }
            "round" => {
                if args.len() != 1 {
                    return Ok(None);
                }
                let x = self.extract_f64(&args[0])?;
                Ok(Some(Expression::Const(x.round().to_string())))
            }
            "trunc" | "truncate" => {
                if args.len() != 1 {
                    return Ok(None);
                }
                let x = self.extract_f64(&args[0])?;
                Ok(Some(Expression::Const(x.trunc().to_string())))
            }
            "frac" | "fract" => {
                // Fractional part of x (x - floor(x))
                if args.len() != 1 {
                    return Ok(None);
                }
                let x = self.extract_f64(&args[0])?;
                Ok(Some(Expression::Const(x.fract().to_string())))
            }
            "sign" | "signum" => {
                // Sign of x: -1.0, 0.0, or 1.0
                if args.len() != 1 {
                    return Ok(None);
                }
                let x = self.extract_f64(&args[0])?;
                Ok(Some(Expression::Const(x.signum().to_string())))
            }
            "fmod" | "remainder" => {
                // Floating-point remainder (x mod y)
                // Note: "mod" is already handled earlier in this match
                if args.len() != 2 {
                    return Ok(None);
                }
                let x = self.extract_f64(&args[0])?;
                let y = self.extract_f64(&args[1])?;
                Ok(Some(Expression::Const((x % y).to_string())))
            }
            "min" => {
                if args.len() != 2 {
                    return Ok(None);
                }
                let a = self.extract_f64(&args[0])?;
                let b = self.extract_f64(&args[1])?;
                Ok(Some(Expression::Const(a.min(b).to_string())))
            }
            "max" => {
                if args.len() != 2 {
                    return Ok(None);
                }
                let a = self.extract_f64(&args[0])?;
                let b = self.extract_f64(&args[1])?;
                Ok(Some(Expression::Const(a.max(b).to_string())))
            }
            "hypot" => {
                // sqrt(x² + y²) computed stably
                if args.len() != 2 {
                    return Ok(None);
                }
                let x = self.extract_f64(&args[0])?;
                let y = self.extract_f64(&args[1])?;
                Ok(Some(Expression::Const(x.hypot(y).to_string())))
            }
            "e" => {
                // Euler's number
                Ok(Some(Expression::Const(std::f64::consts::E.to_string())))
            }
            "tau" => {
                // τ = 2π
                Ok(Some(Expression::Const(std::f64::consts::TAU.to_string())))
            }

            "matrix_add" | "builtin_matrix_add" => {
                // Matrix addition: element-wise addition of two matrices
                // Supports partial symbolic evaluation
                if args.len() != 2 {
                    return Ok(None);
                }
                if let (Some((m1, n1, elems1)), Some((m2, n2, elems2))) =
                    (self.extract_matrix(&args[0]), self.extract_matrix(&args[1]))
                {
                    if m1 != m2 || n1 != n2 {
                        return Err(format!(
                            "matrix_add: dimension mismatch: {}x{} vs {}x{}",
                            m1, n1, m2, n2
                        ));
                    }
                    let result: Vec<Expression> = elems1
                        .iter()
                        .zip(elems2.iter())
                        .map(|(a, b)| self.add_expressions(a, b))
                        .collect();
                    Ok(Some(self.make_matrix(m1, n1, result)))
                } else {
                    Ok(None)
                }
            }

            "matrix_sub" | "builtin_matrix_sub" => {
                // Matrix subtraction: element-wise subtraction
                // Supports partial symbolic evaluation
                if args.len() != 2 {
                    return Ok(None);
                }
                if let (Some((m1, n1, elems1)), Some((m2, n2, elems2))) =
                    (self.extract_matrix(&args[0]), self.extract_matrix(&args[1]))
                {
                    if m1 != m2 || n1 != n2 {
                        return Err(format!(
                            "matrix_sub: dimension mismatch: {}x{} vs {}x{}",
                            m1, n1, m2, n2
                        ));
                    }
                    let result: Vec<Expression> = elems1
                        .iter()
                        .zip(elems2.iter())
                        .map(|(a, b)| self.sub_expressions(a, b))
                        .collect();
                    Ok(Some(self.make_matrix(m1, n1, result)))
                } else {
                    Ok(None)
                }
            }

            "multiply" | "builtin_matrix_mul" | "matmul" => {
                // Matrix multiplication: (m×n) · (n×p) → (m×p)
                if args.len() != 2 {
                    return Ok(None);
                }
                if let (Some((m1, n1, elems1)), Some((m2, n2, elems2))) =
                    (self.extract_matrix(&args[0]), self.extract_matrix(&args[1]))
                {
                    if n1 != m2 {
                        return Err(format!(
                            "matrix multiply: inner dimensions don't match: {}x{} vs {}x{}",
                            m1, n1, m2, n2
                        ));
                    }
                    // Check all elements are numeric (no symbolic variables)
                    let all_numeric = elems1
                        .iter()
                        .chain(elems2.iter())
                        .all(|e| self.as_number(e).is_some());
                    if !all_numeric {
                        // Contains symbolic elements - return unevaluated
                        return Ok(None);
                    }
                    // Compute C[i,j] = sum(A[i,k] * B[k,j] for k in 0..n1)
                    let mut result = Vec::with_capacity(m1 * n2);
                    for i in 0..m1 {
                        for j in 0..n2 {
                            let mut sum = 0.0;
                            for k in 0..n1 {
                                let a_val = self.as_number(&elems1[i * n1 + k]).unwrap_or(0.0);
                                let b_val = self.as_number(&elems2[k * n2 + j]).unwrap_or(0.0);
                                sum += a_val * b_val;
                            }
                            if sum.fract() == 0.0 && sum.abs() < 1e15 {
                                result.push(Expression::Const(format!("{}", sum as i64)));
                            } else {
                                result.push(Expression::Const(format!("{}", sum)));
                            }
                        }
                    }
                    Ok(Some(self.make_matrix(m1, n2, result)))
                } else {
                    Ok(None)
                }
            }

            "transpose" | "builtin_transpose" => {
                // Matrix transpose: (m×n) → (n×m)
                if args.len() != 1 {
                    return Ok(None);
                }
                if let Some((m, n, elems)) = self.extract_matrix(&args[0]) {
                    // Transpose: result[j,i] = original[i,j]
                    let mut result = Vec::with_capacity(m * n);
                    for j in 0..n {
                        for i in 0..m {
                            result.push(elems[i * n + j].clone());
                        }
                    }
                    Ok(Some(self.make_matrix(n, m, result)))
                } else {
                    Ok(None)
                }
            }

            "trace" | "builtin_trace" => {
                // Matrix trace: sum of diagonal elements (square matrices only)
                if args.len() != 1 {
                    return Ok(None);
                }
                if let Some((m, n, elems)) = self.extract_matrix(&args[0]) {
                    if m != n {
                        return Err(format!("trace: matrix must be square, got {}x{}", m, n));
                    }
                    // Check diagonal elements are numeric
                    let diag_numeric = (0..m).all(|i| self.as_number(&elems[i * n + i]).is_some());
                    if !diag_numeric {
                        // Contains symbolic diagonal elements - return unevaluated
                        return Ok(None);
                    }
                    let mut sum = 0.0;
                    for i in 0..m {
                        if let Some(val) = self.as_number(&elems[i * n + i]) {
                            sum += val;
                        }
                    }
                    if sum.fract() == 0.0 && sum.abs() < 1e15 {
                        Ok(Some(Expression::Const(format!("{}", sum as i64))))
                    } else {
                        Ok(Some(Expression::Const(format!("{}", sum))))
                    }
                } else {
                    Ok(None)
                }
            }

            "det" | "builtin_determinant" => {
                // Matrix determinant (only 2x2 and 3x3 for now)
                if args.len() != 1 {
                    return Ok(None);
                }
                if let Some((m, n, elems)) = self.extract_matrix(&args[0]) {
                    if m != n {
                        return Err(format!("det: matrix must be square, got {}x{}", m, n));
                    }
                    // Check all elements are numeric (no symbolic variables)
                    if !elems.iter().all(|e| self.as_number(e).is_some()) {
                        // Contains symbolic elements - return unevaluated
                        return Ok(None);
                    }
                    let det = match m {
                        1 => self.as_number(&elems[0]).unwrap_or(0.0),
                        2 => {
                            // det([[a,b],[c,d]]) = ad - bc
                            let a = self.as_number(&elems[0]).unwrap_or(0.0);
                            let b = self.as_number(&elems[1]).unwrap_or(0.0);
                            let c = self.as_number(&elems[2]).unwrap_or(0.0);
                            let d = self.as_number(&elems[3]).unwrap_or(0.0);
                            a * d - b * c
                        }
                        3 => {
                            // Sarrus rule for 3x3
                            let a = |i: usize, j: usize| {
                                self.as_number(&elems[i * 3 + j]).unwrap_or(0.0)
                            };
                            a(0, 0) * (a(1, 1) * a(2, 2) - a(1, 2) * a(2, 1))
                                - a(0, 1) * (a(1, 0) * a(2, 2) - a(1, 2) * a(2, 0))
                                + a(0, 2) * (a(1, 0) * a(2, 1) - a(1, 1) * a(2, 0))
                        }
                        _ => {
                            return Err(format!(
                                "det: only 1x1, 2x2, 3x3 supported, got {}x{}",
                                m, n
                            ));
                        }
                    };
                    if det.fract() == 0.0 && det.abs() < 1e15 {
                        Ok(Some(Expression::Const(format!("{}", det as i64))))
                    } else {
                        Ok(Some(Expression::Const(format!("{}", det))))
                    }
                } else {
                    Ok(None)
                }
            }

            "scalar_matrix_mul" | "builtin_matrix_scalar_mul" => {
                // Scalar * Matrix: multiply all elements by scalar
                if args.len() != 2 {
                    return Ok(None);
                }
                // Try both orders: scalar * matrix or matrix * scalar
                let (scalar, matrix) = if let Some(s) = self.as_number(&args[0]) {
                    if let Some(mat) = self.extract_matrix(&args[1]) {
                        (s, mat)
                    } else {
                        return Ok(None);
                    }
                } else if let Some(s) = self.as_number(&args[1]) {
                    if let Some(mat) = self.extract_matrix(&args[0]) {
                        (s, mat)
                    } else {
                        return Ok(None);
                    }
                } else {
                    return Ok(None);
                };

                let (m, n, elems) = matrix;
                let result: Result<Vec<Expression>, String> = elems
                    .iter()
                    .map(|e| {
                        if let Some(val) = self.as_number(e) {
                            let product = scalar * val;
                            if product.fract() == 0.0 && product.abs() < 1e15 {
                                Ok(Expression::Const(format!("{}", product as i64)))
                            } else {
                                Ok(Expression::Const(format!("{}", product)))
                            }
                        } else {
                            Err("scalar_matrix_mul: non-numeric element".to_string())
                        }
                    })
                    .collect();
                Ok(Some(self.make_matrix(m, n, result?)))
            }

            "size" | "shape" | "dims" => {
                // Get matrix dimensions as a tuple/list
                // size(M) → [m, n]
                if args.len() != 1 {
                    return Ok(None);
                }
                if let Some((m, n, _)) = self.extract_matrix(&args[0]) {
                    Ok(Some(Expression::List(vec![
                        Expression::Const(m.to_string()),
                        Expression::Const(n.to_string()),
                    ])))
                } else {
                    Ok(None)
                }
            }

            "nrows" | "num_rows" => {
                // Get number of rows
                // nrows(M) → m
                if args.len() != 1 {
                    return Ok(None);
                }
                if let Some((m, _, _)) = self.extract_matrix(&args[0]) {
                    Ok(Some(Expression::Const(m.to_string())))
                } else {
                    Ok(None)
                }
            }

            "ncols" | "num_cols" => {
                // Get number of columns
                // ncols(M) → n
                if args.len() != 1 {
                    return Ok(None);
                }
                if let Some((_, n, _)) = self.extract_matrix(&args[0]) {
                    Ok(Some(Expression::Const(n.to_string())))
                } else {
                    Ok(None)
                }
            }

            "matrix_get" | "element" => {
                // Get element at (i, j) from matrix
                // matrix_get(M, i, j) → element
                if args.len() != 3 {
                    return Ok(None);
                }
                if let Some((m, n, elems)) = self.extract_matrix(&args[0]) {
                    let i = self.as_integer(&args[1]);
                    let j = self.as_integer(&args[2]);
                    if let (Some(i), Some(j)) = (i, j) {
                        let i = i as usize;
                        let j = j as usize;
                        if i < m && j < n {
                            let idx = i * n + j;
                            Ok(Some(elems[idx].clone()))
                        } else {
                            Err(format!(
                                "matrix_get: index ({}, {}) out of bounds for {}x{} matrix",
                                i, j, m, n
                            ))
                        }
                    } else {
                        Ok(None) // Symbolic indices - return unevaluated
                    }
                } else {
                    Ok(None)
                }
            }

            "matrix_row" | "row" => {
                // Get row i from matrix as a list
                // matrix_row(M, i) → [elements]
                if args.len() != 2 {
                    return Ok(None);
                }
                if let Some((m, n, elems)) = self.extract_matrix(&args[0]) {
                    if let Some(i) = self.as_integer(&args[1]) {
                        let i = i as usize;
                        if i < m {
                            let start = i * n;
                            let row: Vec<Expression> = elems[start..start + n].to_vec();
                            Ok(Some(Expression::List(row)))
                        } else {
                            Err(format!(
                                "matrix_row: row {} out of bounds for {}x{} matrix",
                                i, m, n
                            ))
                        }
                    } else {
                        Ok(None) // Symbolic index - return unevaluated
                    }
                } else {
                    Ok(None)
                }
            }

            "matrix_col" | "col" => {
                // Get column j from matrix as a list
                // matrix_col(M, j) → [elements]
                if args.len() != 2 {
                    return Ok(None);
                }
                if let Some((m, n, elems)) = self.extract_matrix(&args[0]) {
                    if let Some(j) = self.as_integer(&args[1]) {
                        let j = j as usize;
                        if j < n {
                            let col: Vec<Expression> =
                                (0..m).map(|i| elems[i * n + j].clone()).collect();
                            Ok(Some(Expression::List(col)))
                        } else {
                            Err(format!(
                                "matrix_col: column {} out of bounds for {}x{} matrix",
                                j, m, n
                            ))
                        }
                    } else {
                        Ok(None) // Symbolic index - return unevaluated
                    }
                } else {
                    Ok(None)
                }
            }

            "matrix_diag" | "diag" => {
                // Get diagonal elements from square matrix as a list
                // matrix_diag(M) → [diagonal elements]
                if args.len() != 1 {
                    return Ok(None);
                }
                if let Some((m, n, elems)) = self.extract_matrix(&args[0]) {
                    if m != n {
                        return Err(format!(
                            "matrix_diag: matrix must be square, got {}x{}",
                            m, n
                        ));
                    }
                    let diag: Vec<Expression> = (0..m).map(|i| elems[i * n + i].clone()).collect();
                    Ok(Some(Expression::List(diag)))
                } else {
                    Ok(None)
                }
            }

            "set_element" | "set" => {
                // Set element at (i, j) to a new value, return new matrix
                // set_element(M, i, j, value) → new Matrix
                if args.len() != 4 {
                    return Ok(None);
                }
                if let Some((m, n, mut elems)) = self.extract_matrix(&args[0]) {
                    let i = self.as_integer(&args[1]);
                    let j = self.as_integer(&args[2]);
                    if let (Some(i), Some(j)) = (i, j) {
                        let i = i as usize;
                        let j = j as usize;
                        if i < m && j < n {
                            let idx = i * n + j;
                            // Evaluate the new value
                            let new_val = match self.eval_concrete(&args[3]) {
                                Ok(v) => v,
                                Err(_) => args[3].clone(),
                            };
                            elems[idx] = new_val;
                            Ok(Some(self.make_matrix(m, n, elems)))
                        } else {
                            Err(format!(
                                "set_element: index ({}, {}) out of bounds for {}x{} matrix",
                                i, j, m, n
                            ))
                        }
                    } else {
                        Ok(None) // Symbolic indices - return unevaluated
                    }
                } else {
                    Ok(None)
                }
            }

            "set_row" => {
                // Set row i to new values, return new matrix
                // set_row(M, i, [values]) → new Matrix
                if args.len() != 3 {
                    return Ok(None);
                }
                if let Some((m, n, mut elems)) = self.extract_matrix(&args[0]) {
                    if let Some(i) = self.as_integer(&args[1]) {
                        let i = i as usize;
                        if i >= m {
                            return Err(format!(
                                "set_row: row {} out of bounds for {}x{} matrix",
                                i, m, n
                            ));
                        }
                        // Get the new row values
                        match &args[2] {
                            Expression::List(new_row) => {
                                if new_row.len() != n {
                                    return Err(format!(
                                        "set_row: row has {} elements but matrix has {} columns",
                                        new_row.len(),
                                        n
                                    ));
                                }
                                for (j, val) in new_row.iter().enumerate() {
                                    let new_val = match self.eval_concrete(val) {
                                        Ok(v) => v,
                                        Err(_) => val.clone(),
                                    };
                                    elems[i * n + j] = new_val;
                                }
                                Ok(Some(self.make_matrix(m, n, elems)))
                            }
                            _ => Ok(None),
                        }
                    } else {
                        Ok(None) // Symbolic index - return unevaluated
                    }
                } else {
                    Ok(None)
                }
            }

            "set_col" => {
                // Set column j to new values, return new matrix
                // set_col(M, j, [values]) → new Matrix
                if args.len() != 3 {
                    return Ok(None);
                }
                if let Some((m, n, mut elems)) = self.extract_matrix(&args[0]) {
                    if let Some(j) = self.as_integer(&args[1]) {
                        let j = j as usize;
                        if j >= n {
                            return Err(format!(
                                "set_col: column {} out of bounds for {}x{} matrix",
                                j, m, n
                            ));
                        }
                        // Get the new column values
                        match &args[2] {
                            Expression::List(new_col) => {
                                if new_col.len() != m {
                                    return Err(format!(
                                        "set_col: column has {} elements but matrix has {} rows",
                                        new_col.len(),
                                        m
                                    ));
                                }
                                for (i, val) in new_col.iter().enumerate() {
                                    let new_val = match self.eval_concrete(val) {
                                        Ok(v) => v,
                                        Err(_) => val.clone(),
                                    };
                                    elems[i * n + j] = new_val;
                                }
                                Ok(Some(self.make_matrix(m, n, elems)))
                            }
                            _ => Ok(None),
                        }
                    } else {
                        Ok(None) // Symbolic index - return unevaluated
                    }
                } else {
                    Ok(None)
                }
            }

            "set_diag" => {
                // Set diagonal elements to new values, return new matrix
                // set_diag(M, [values]) → new Matrix (square matrix only)
                if args.len() != 2 {
                    return Ok(None);
                }
                if let Some((m, n, mut elems)) = self.extract_matrix(&args[0]) {
                    if m != n {
                        return Err(format!("set_diag: matrix must be square, got {}x{}", m, n));
                    }
                    match &args[1] {
                        Expression::List(new_diag) => {
                            if new_diag.len() != m {
                                return Err(format!(
                                    "set_diag: diagonal has {} elements but matrix is {}x{}",
                                    new_diag.len(),
                                    m,
                                    n
                                ));
                            }
                            for (i, val) in new_diag.iter().enumerate() {
                                let new_val = match self.eval_concrete(val) {
                                    Ok(v) => v,
                                    Err(_) => val.clone(),
                                };
                                elems[i * n + i] = new_val;
                            }
                            Ok(Some(self.make_matrix(m, n, elems)))
                        }
                        _ => Ok(None),
                    }
                } else {
                    Ok(None)
                }
            }

            "eye" | "identity" => {
                // Create n×n identity matrix
                // eye(n) → Matrix(n, n, [1,0,0,...,0,1,0,...,0,0,1])
                if args.len() != 1 {
                    return Ok(None);
                }
                if let Some(n) = self.as_integer(&args[0]) {
                    if n <= 0 {
                        return Err(format!("eye: size must be positive, got {}", n));
                    }
                    let n = n as usize;
                    let mut elems = Vec::with_capacity(n * n);
                    for i in 0..n {
                        for j in 0..n {
                            if i == j {
                                elems.push(Expression::Const("1".to_string()));
                            } else {
                                elems.push(Expression::Const("0".to_string()));
                            }
                        }
                    }
                    Ok(Some(self.make_matrix(n, n, elems)))
                } else {
                    Ok(None)
                }
            }

            "zeros" => {
                // Create m×n zero matrix
                // zeros(m, n) → Matrix(m, n, [0,0,...,0])
                // zeros(n) → Matrix(n, n, [0,0,...,0])
                if args.is_empty() || args.len() > 2 {
                    return Ok(None);
                }
                let (m, n) = if args.len() == 1 {
                    let size = self.as_integer(&args[0]).unwrap_or(0) as usize;
                    (size, size)
                } else {
                    let m = self.as_integer(&args[0]).unwrap_or(0) as usize;
                    let n = self.as_integer(&args[1]).unwrap_or(0) as usize;
                    (m, n)
                };
                if m == 0 || n == 0 {
                    return Ok(None);
                }
                let elems: Vec<Expression> = vec![Expression::Const("0".to_string()); m * n];
                Ok(Some(self.make_matrix(m, n, elems)))
            }

            "ones" => {
                // Create m×n matrix of ones
                // ones(m, n) → Matrix(m, n, [1,1,...,1])
                // ones(n) → Matrix(n, n, [1,1,...,1])
                if args.is_empty() || args.len() > 2 {
                    return Ok(None);
                }
                let (m, n) = if args.len() == 1 {
                    let size = self.as_integer(&args[0]).unwrap_or(0) as usize;
                    (size, size)
                } else {
                    let m = self.as_integer(&args[0]).unwrap_or(0) as usize;
                    let n = self.as_integer(&args[1]).unwrap_or(0) as usize;
                    (m, n)
                };
                if m == 0 || n == 0 {
                    return Ok(None);
                }
                let elems: Vec<Expression> = vec![Expression::Const("1".to_string()); m * n];
                Ok(Some(self.make_matrix(m, n, elems)))
            }

            "diag_matrix" | "diagonal" => {
                // Create diagonal matrix from list
                // diag_matrix([a, b, c]) → Matrix(3, 3, [a,0,0, 0,b,0, 0,0,c])
                if args.len() != 1 {
                    return Ok(None);
                }
                if let Expression::List(values) = &args[0] {
                    let n = values.len();
                    if n == 0 {
                        return Ok(None);
                    }
                    let mut elems = Vec::with_capacity(n * n);
                    for (i, val) in values.iter().enumerate() {
                        for j in 0..n {
                            if i == j {
                                elems.push(val.clone());
                            } else {
                                elems.push(Expression::Const("0".to_string()));
                            }
                        }
                    }
                    Ok(Some(self.make_matrix(n, n, elems)))
                } else {
                    Ok(None)
                }
            }

            "matrix" => {
                // Create matrix from nested list (row-major)
                // matrix([[1, 2, 3], [4, 5, 6]]) → Matrix(2, 3, [1, 2, 3, 4, 5, 6])
                if args.len() != 1 {
                    return Ok(None);
                }
                if let Expression::List(rows) = &args[0] {
                    if rows.is_empty() {
                        return Err("matrix: empty matrix".to_string());
                    }

                    // Extract first row to get number of columns
                    let first_row = match &rows[0] {
                        Expression::List(r) => r,
                        _ => return Err("matrix: expected list of rows".to_string()),
                    };
                    let n_cols = first_row.len();
                    if n_cols == 0 {
                        return Err("matrix: rows cannot be empty".to_string());
                    }
                    let n_rows = rows.len();

                    // Flatten all rows into elements
                    let mut elems = Vec::with_capacity(n_rows * n_cols);
                    for (i, row) in rows.iter().enumerate() {
                        match row {
                            Expression::List(r) => {
                                if r.len() != n_cols {
                                    return Err(format!(
                                        "matrix: row {} has {} elements, expected {}",
                                        i,
                                        r.len(),
                                        n_cols
                                    ));
                                }
                                for elem in r {
                                    // Evaluate each element to handle expressions like -1
                                    match self.eval_concrete(elem) {
                                        Ok(e) => elems.push(e),
                                        Err(_) => elems.push(elem.clone()),
                                    }
                                }
                            }
                            _ => {
                                return Err(format!("matrix: row {} is not a list", i));
                            }
                        }
                    }

                    Ok(Some(self.make_matrix(n_rows, n_cols, elems)))
                } else {
                    Ok(None)
                }
            }

            "vstack" | "append_rows" => {
                // Vertical stack: append rows from B to bottom of A
                // vstack(A, B) where A is m×n and B is k×n → (m+k)×n
                if args.len() != 2 {
                    return Ok(None);
                }
                if let (Some((m1, n1, elems1)), Some((m2, n2, elems2))) =
                    (self.extract_matrix(&args[0]), self.extract_matrix(&args[1]))
                {
                    if n1 != n2 {
                        return Err(format!("vstack: column count mismatch: {} vs {}", n1, n2));
                    }
                    let mut result = elems1;
                    result.extend(elems2);
                    Ok(Some(self.make_matrix(m1 + m2, n1, result)))
                } else {
                    Ok(None)
                }
            }

            "hstack" | "append_cols" => {
                // Horizontal stack: append columns from B to right of A
                // hstack(A, B) where A is m×n and B is m×k → m×(n+k)
                if args.len() != 2 {
                    return Ok(None);
                }
                if let (Some((m1, n1, elems1)), Some((m2, n2, elems2))) =
                    (self.extract_matrix(&args[0]), self.extract_matrix(&args[1]))
                {
                    if m1 != m2 {
                        return Err(format!("hstack: row count mismatch: {} vs {}", m1, m2));
                    }
                    // Interleave columns: for each row, append B's columns after A's
                    let mut result = Vec::with_capacity(m1 * (n1 + n2));
                    for i in 0..m1 {
                        // Add row i from A
                        for j in 0..n1 {
                            result.push(elems1[i * n1 + j].clone());
                        }
                        // Add row i from B
                        for j in 0..n2 {
                            result.push(elems2[i * n2 + j].clone());
                        }
                    }
                    Ok(Some(self.make_matrix(m1, n1 + n2, result)))
                } else {
                    Ok(None)
                }
            }

            "prepend_row" => {
                // Add a row at the top of the matrix
                // prepend_row([a,b,c], M) where M is m×3 → (m+1)×3
                if args.len() != 2 {
                    return Ok(None);
                }
                if let (Expression::List(row), Some((m, n, elems))) =
                    (&args[0], self.extract_matrix(&args[1]))
                {
                    if row.len() != n {
                        return Err(format!(
                            "prepend_row: row has {} elements but matrix has {} columns",
                            row.len(),
                            n
                        ));
                    }
                    let mut result = row.clone();
                    result.extend(elems);
                    Ok(Some(self.make_matrix(m + 1, n, result)))
                } else {
                    Ok(None)
                }
            }

            "append_row" => {
                // Add a row at the bottom of the matrix
                // append_row(M, [a,b,c]) where M is m×3 → (m+1)×3
                if args.len() != 2 {
                    return Ok(None);
                }
                if let (Some((m, n, elems)), Expression::List(row)) =
                    (self.extract_matrix(&args[0]), &args[1])
                {
                    if row.len() != n {
                        return Err(format!(
                            "append_row: row has {} elements but matrix has {} columns",
                            row.len(),
                            n
                        ));
                    }
                    let mut result = elems;
                    result.extend(row.clone());
                    Ok(Some(self.make_matrix(m + 1, n, result)))
                } else {
                    Ok(None)
                }
            }

            "prepend_col" => {
                // Add a column at the left of the matrix
                // prepend_col([a,b], M) where M is 2×n → 2×(n+1)
                if args.len() != 2 {
                    return Ok(None);
                }
                if let (Expression::List(col), Some((m, n, elems))) =
                    (&args[0], self.extract_matrix(&args[1]))
                {
                    if col.len() != m {
                        return Err(format!(
                            "prepend_col: column has {} elements but matrix has {} rows",
                            col.len(),
                            m
                        ));
                    }
                    let mut result = Vec::with_capacity(m * (n + 1));
                    for i in 0..m {
                        result.push(col[i].clone());
                        for j in 0..n {
                            result.push(elems[i * n + j].clone());
                        }
                    }
                    Ok(Some(self.make_matrix(m, n + 1, result)))
                } else {
                    Ok(None)
                }
            }

            "append_col" => {
                // Add a column at the right of the matrix
                // append_col(M, [a,b]) where M is 2×n → 2×(n+1)
                if args.len() != 2 {
                    return Ok(None);
                }
                if let (Some((m, n, elems)), Expression::List(col)) =
                    (self.extract_matrix(&args[0]), &args[1])
                {
                    if col.len() != m {
                        return Err(format!(
                            "append_col: column has {} elements but matrix has {} rows",
                            col.len(),
                            m
                        ));
                    }
                    let mut result = Vec::with_capacity(m * (n + 1));
                    for i in 0..m {
                        for j in 0..n {
                            result.push(elems[i * n + j].clone());
                        }
                        result.push(col[i].clone());
                    }
                    Ok(Some(self.make_matrix(m, n + 1, result)))
                } else {
                    Ok(None)
                }
            }

            "complex_add" | "cadd" => {
                // Complex addition: (a+bi) + (c+di) = (a+c) + (b+d)i
                if args.len() != 2 {
                    return Ok(None);
                }
                if let (Some((re1, im1)), Some((re2, im2))) = (
                    self.extract_complex(&args[0]),
                    self.extract_complex(&args[1]),
                ) {
                    let re_sum = self.add_expressions(&re1, &re2);
                    let im_sum = self.add_expressions(&im1, &im2);
                    Ok(Some(self.make_complex(re_sum, im_sum)))
                } else {
                    Ok(None)
                }
            }

            "complex_sub" | "csub" => {
                // Complex subtraction: (a+bi) - (c+di) = (a-c) + (b-d)i
                if args.len() != 2 {
                    return Ok(None);
                }
                if let (Some((re1, im1)), Some((re2, im2))) = (
                    self.extract_complex(&args[0]),
                    self.extract_complex(&args[1]),
                ) {
                    let re_diff = self.sub_expressions(&re1, &re2);
                    let im_diff = self.sub_expressions(&im1, &im2);
                    Ok(Some(self.make_complex(re_diff, im_diff)))
                } else {
                    Ok(None)
                }
            }

            "complex_mul" | "cmul" => {
                // Complex multiplication: (a+bi)(c+di) = (ac-bd) + (ad+bc)i
                if args.len() != 2 {
                    return Ok(None);
                }
                if let (Some((re1, im1)), Some((re2, im2))) = (
                    self.extract_complex(&args[0]),
                    self.extract_complex(&args[1]),
                ) {
                    // Real part: ac - bd
                    let ac = self.mul_expressions(&re1, &re2);
                    let bd = self.mul_expressions(&im1, &im2);
                    let re_result = self.sub_expressions(&ac, &bd);

                    // Imaginary part: ad + bc
                    let ad = self.mul_expressions(&re1, &im2);
                    let bc = self.mul_expressions(&im1, &re2);
                    let im_result = self.add_expressions(&ad, &bc);

                    Ok(Some(self.make_complex(re_result, im_result)))
                } else {
                    Ok(None)
                }
            }

            "complex_conj" | "conj" | "conjugate" => {
                // Complex conjugate: conj(a+bi) = a-bi
                if args.len() != 1 {
                    return Ok(None);
                }
                if let Some((re, im)) = self.extract_complex(&args[0]) {
                    // Negate imaginary part
                    let neg_im = self.negate_expression(&im);
                    Ok(Some(self.make_complex(re, neg_im)))
                } else {
                    Ok(None)
                }
            }

            "complex_abs_squared" | "abs_sq" => {
                // |z|² = a² + b² (returns real)
                if args.len() != 1 {
                    return Ok(None);
                }
                if let Some((re, im)) = self.extract_complex(&args[0]) {
                    let re_sq = self.mul_expressions(&re, &re);
                    let im_sq = self.mul_expressions(&im, &im);
                    Ok(Some(self.add_expressions(&re_sq, &im_sq)))
                } else {
                    Ok(None)
                }
            }

            "Re" | "re" | "real_part" | "real" => {
                // Real part of complex number
                if args.len() != 1 {
                    return Ok(None);
                }
                if let Some((re, _im)) = self.extract_complex(&args[0]) {
                    Ok(Some(re))
                } else {
                    Ok(None)
                }
            }

            "Im" | "im" | "imag_part" | "imag" => {
                // Imaginary part of complex number
                if args.len() != 1 {
                    return Ok(None);
                }
                if let Some((_re, im)) = self.extract_complex(&args[0]) {
                    Ok(Some(im))
                } else {
                    Ok(None)
                }
            }

            "cmat_zero" | "builtin_cmat_zero" => {
                // Create zero complex matrix: (zeros(m,n), zeros(m,n))
                if args.len() != 2 {
                    return Ok(None);
                }
                if let (Some(m), Some(n)) = (self.as_nat(&args[0]), self.as_nat(&args[1])) {
                    let zeros =
                        self.make_matrix(m, n, vec![Expression::Const("0".to_string()); m * n]);
                    Ok(Some(Expression::List(vec![zeros.clone(), zeros])))
                } else {
                    Ok(None)
                }
            }

            "cmat_eye" | "builtin_cmat_eye" => {
                // Create complex identity matrix: (eye(n), zeros(n,n))
                if args.len() != 1 {
                    return Ok(None);
                }
                if let Some(n) = self.as_nat(&args[0]) {
                    let mut eye_elems = vec![Expression::Const("0".to_string()); n * n];
                    for i in 0..n {
                        eye_elems[i * n + i] = Expression::Const("1".to_string());
                    }
                    let eye = self.make_matrix(n, n, eye_elems);
                    let zeros =
                        self.make_matrix(n, n, vec![Expression::Const("0".to_string()); n * n]);
                    Ok(Some(Expression::List(vec![eye, zeros])))
                } else {
                    Ok(None)
                }
            }

            "cmat_from_real" | "builtin_cmat_from_real" | "as_complex" => {
                // Promote real matrix to complex: A → (A, zeros)
                if args.len() != 1 {
                    return Ok(None);
                }
                if let Some((m, n, elems)) = self.extract_matrix(&args[0]) {
                    let real_part = self.make_matrix(m, n, elems);
                    let zeros =
                        self.make_matrix(m, n, vec![Expression::Const("0".to_string()); m * n]);
                    Ok(Some(Expression::List(vec![real_part, zeros])))
                } else {
                    Ok(None)
                }
            }

            "cmat_from_imag" | "builtin_cmat_from_imag" | "as_imaginary" => {
                // Create pure imaginary matrix: B → (zeros, B)
                if args.len() != 1 {
                    return Ok(None);
                }
                if let Some((m, n, elems)) = self.extract_matrix(&args[0]) {
                    let imag_part = self.make_matrix(m, n, elems);
                    let zeros =
                        self.make_matrix(m, n, vec![Expression::Const("0".to_string()); m * n]);
                    Ok(Some(Expression::List(vec![zeros, imag_part])))
                } else {
                    Ok(None)
                }
            }

            "cmat_real" | "builtin_cmat_real" | "real_part_matrix" => {
                // Extract real part: (A, B) → A
                if args.len() != 1 {
                    return Ok(None);
                }
                if let Some((real, _imag)) = self.extract_complex_matrix(&args[0]) {
                    Ok(Some(real))
                } else {
                    Ok(None)
                }
            }

            "cmat_imag" | "builtin_cmat_imag" | "imag_part_matrix" => {
                // Extract imaginary part: (A, B) → B
                if args.len() != 1 {
                    return Ok(None);
                }
                if let Some((_real, imag)) = self.extract_complex_matrix(&args[0]) {
                    Ok(Some(imag))
                } else {
                    Ok(None)
                }
            }

            "cmat_add" | "builtin_cmat_add" => {
                // Complex matrix addition: (A₁,B₁) + (A₂,B₂) = (A₁+A₂, B₁+B₂)
                if args.len() != 2 {
                    return Ok(None);
                }
                if let (Some((real1, imag1)), Some((real2, imag2))) = (
                    self.extract_complex_matrix(&args[0]),
                    self.extract_complex_matrix(&args[1]),
                ) {
                    let sum_real = self
                        .eval_concrete(&Expression::operation("matrix_add", vec![real1, real2]))?;
                    let sum_imag = self
                        .eval_concrete(&Expression::operation("matrix_add", vec![imag1, imag2]))?;
                    Ok(Some(Expression::List(vec![sum_real, sum_imag])))
                } else {
                    Ok(None)
                }
            }

            "cmat_sub" | "builtin_cmat_sub" => {
                // Complex matrix subtraction: (A₁,B₁) - (A₂,B₂) = (A₁-A₂, B₁-B₂)
                if args.len() != 2 {
                    return Ok(None);
                }
                if let (Some((real1, imag1)), Some((real2, imag2))) = (
                    self.extract_complex_matrix(&args[0]),
                    self.extract_complex_matrix(&args[1]),
                ) {
                    let diff_real = self
                        .eval_concrete(&Expression::operation("matrix_sub", vec![real1, real2]))?;
                    let diff_imag = self
                        .eval_concrete(&Expression::operation("matrix_sub", vec![imag1, imag2]))?;
                    Ok(Some(Expression::List(vec![diff_real, diff_imag])))
                } else {
                    Ok(None)
                }
            }

            "cmat_mul" | "builtin_cmat_mul" => {
                // Complex matrix multiplication:
                // (A₁,B₁) · (A₂,B₂) = (A₁·A₂ - B₁·B₂, A₁·B₂ + B₁·A₂)
                if args.len() != 2 {
                    return Ok(None);
                }
                if let (Some((a1, b1)), Some((a2, b2))) = (
                    self.extract_complex_matrix(&args[0]),
                    self.extract_complex_matrix(&args[1]),
                ) {
                    // Real part: A₁·A₂ - B₁·B₂
                    let a1a2 = self.eval_concrete(&Expression::operation(
                        "multiply",
                        vec![a1.clone(), a2.clone()],
                    ))?;
                    let b1b2 = self.eval_concrete(&Expression::operation(
                        "multiply",
                        vec![b1.clone(), b2.clone()],
                    ))?;
                    let real_part =
                        self.eval_concrete(&Expression::operation("matrix_sub", vec![a1a2, b1b2]))?;

                    // Imag part: A₁·B₂ + B₁·A₂
                    let a1b2 =
                        self.eval_concrete(&Expression::operation("multiply", vec![a1, b2]))?;
                    let b1a2 =
                        self.eval_concrete(&Expression::operation("multiply", vec![b1, a2]))?;
                    let imag_part =
                        self.eval_concrete(&Expression::operation("matrix_add", vec![a1b2, b1a2]))?;

                    Ok(Some(Expression::List(vec![real_part, imag_part])))
                } else {
                    Ok(None)
                }
            }

            "cmat_conj" | "builtin_cmat_conj" => {
                // Complex conjugate: conj((A,B)) = (A, -B)
                if args.len() != 1 {
                    return Ok(None);
                }
                if let Some((real, imag)) = self.extract_complex_matrix(&args[0]) {
                    // Negate imaginary part
                    let neg_imag = self.eval_concrete(&Expression::operation(
                        "scalar_matrix_mul",
                        vec![Expression::Const("-1".to_string()), imag],
                    ))?;
                    Ok(Some(Expression::List(vec![real, neg_imag])))
                } else {
                    Ok(None)
                }
            }

            "cmat_transpose" | "builtin_cmat_transpose" => {
                // Transpose: transpose((A,B)) = (transpose(A), transpose(B))
                if args.len() != 1 {
                    return Ok(None);
                }
                if let Some((real, imag)) = self.extract_complex_matrix(&args[0]) {
                    let real_t =
                        self.eval_concrete(&Expression::operation("transpose", vec![real]))?;
                    let imag_t =
                        self.eval_concrete(&Expression::operation("transpose", vec![imag]))?;
                    Ok(Some(Expression::List(vec![real_t, imag_t])))
                } else {
                    Ok(None)
                }
            }

            "cmat_dagger" | "builtin_cmat_dagger" | "cmat_adjoint" => {
                // Conjugate transpose (Hermitian adjoint):
                // dagger((A,B)) = (transpose(A), -transpose(B))
                if args.len() != 1 {
                    return Ok(None);
                }
                if let Some((real, imag)) = self.extract_complex_matrix(&args[0]) {
                    let real_t =
                        self.eval_concrete(&Expression::operation("transpose", vec![real]))?;
                    let imag_t =
                        self.eval_concrete(&Expression::operation("transpose", vec![imag]))?;
                    let neg_imag_t = self.eval_concrete(&Expression::operation(
                        "scalar_matrix_mul",
                        vec![Expression::Const("-1".to_string()), imag_t],
                    ))?;
                    Ok(Some(Expression::List(vec![real_t, neg_imag_t])))
                } else {
                    Ok(None)
                }
            }

            "cmat_trace" | "builtin_cmat_trace" => {
                // Trace: trace((A,B)) = (trace(A), trace(B))
                if args.len() != 1 {
                    return Ok(None);
                }
                if let Some((real, imag)) = self.extract_complex_matrix(&args[0]) {
                    let trace_real =
                        self.eval_concrete(&Expression::operation("trace", vec![real]))?;
                    let trace_imag =
                        self.eval_concrete(&Expression::operation("trace", vec![imag]))?;
                    Ok(Some(Expression::List(vec![trace_real, trace_imag])))
                } else {
                    Ok(None)
                }
            }

            "cmat_scale_real" | "builtin_cmat_scale_real" => {
                // Scale by real scalar: r · (A,B) = (r·A, r·B)
                if args.len() != 2 {
                    return Ok(None);
                }
                let scalar = args[0].clone();
                if let Some((real, imag)) = self.extract_complex_matrix(&args[1]) {
                    let scaled_real = self.eval_concrete(&Expression::operation(
                        "scalar_matrix_mul",
                        vec![scalar.clone(), real],
                    ))?;
                    let scaled_imag = self.eval_concrete(&Expression::operation(
                        "scalar_matrix_mul",
                        vec![scalar, imag],
                    ))?;
                    Ok(Some(Expression::List(vec![scaled_real, scaled_imag])))
                } else {
                    Ok(None)
                }
            }

            "realify" | "builtin_realify" => {
                // Embed complex n×n matrix into real 2n×2n matrix:
                // realify((A, B)) = [[A, -B], [B, A]]
                if args.len() != 1 {
                    return Ok(None);
                }
                if let Some((a_expr, b_expr)) = self.extract_complex_matrix(&args[0]) {
                    if let (Some((n, m, a_elems)), Some((n2, m2, b_elems))) =
                        (self.extract_matrix(&a_expr), self.extract_matrix(&b_expr))
                    {
                        if n != m || n2 != m2 || n != n2 {
                            return Err("realify: complex matrix must be square".to_string());
                        }
                        // Build 2n×2n block matrix [[A, -B], [B, A]]
                        let n2_size = 2 * n;
                        let mut result =
                            vec![Expression::Const("0".to_string()); n2_size * n2_size];

                        for i in 0..n {
                            for j in 0..n {
                                // Top-left: A
                                result[i * n2_size + j] = a_elems[i * n + j].clone();
                                // Top-right: -B
                                let b_val = &b_elems[i * n + j];
                                result[i * n2_size + (j + n)] =
                                    Expression::operation("negate", vec![b_val.clone()]);
                                // Bottom-left: B
                                result[(i + n) * n2_size + j] = b_elems[i * n + j].clone();
                                // Bottom-right: A
                                result[(i + n) * n2_size + (j + n)] = a_elems[i * n + j].clone();
                            }
                        }
                        // Evaluate to simplify negations
                        let mut simplified = Vec::with_capacity(result.len());
                        for elem in result {
                            simplified.push(self.eval_concrete(&elem)?);
                        }
                        Ok(Some(self.make_matrix(n2_size, n2_size, simplified)))
                    } else {
                        Ok(None)
                    }
                } else {
                    Ok(None)
                }
            }

            "complexify" | "builtin_complexify" => {
                // Extract complex n×n from real 2n×2n with block structure [[A, -B], [B, A]]
                // complexify(M) → (A, B)
                if args.len() != 1 {
                    return Ok(None);
                }
                if let Some((m2, n2, elems)) = self.extract_matrix(&args[0]) {
                    if m2 != n2 || m2 % 2 != 0 {
                        return Err(
                            "complexify: matrix must be square with even dimension".to_string()
                        );
                    }
                    let n = m2 / 2;
                    // Extract A from top-left block and B from bottom-left block
                    let mut a_elems = Vec::with_capacity(n * n);
                    let mut b_elems = Vec::with_capacity(n * n);
                    for i in 0..n {
                        for j in 0..n {
                            a_elems.push(elems[i * m2 + j].clone());
                            b_elems.push(elems[(i + n) * m2 + j].clone());
                        }
                    }
                    let a = self.make_matrix(n, n, a_elems);
                    let b = self.make_matrix(n, n, b_elems);
                    Ok(Some(Expression::List(vec![a, b])))
                } else {
                    Ok(None)
                }
            }

            #[cfg(feature = "numerical")]
            "cmat_eigenvalues" | "cmat_eigvals" => {
                // Complex matrix eigenvalues via realification
                // eigenvalues of (A,B) come from eigenvalues of [[A,-B],[B,A]]
                // Real eigenvalues appear doubled; complex pairs appear as a ± bi
                if args.len() != 1 {
                    return Ok(None);
                }
                // First realify the complex matrix
                let realified =
                    self.eval_concrete(&Expression::operation("realify", args.to_vec()))?;
                // Get eigenvalues of the realified matrix
                let eigs =
                    self.eval_concrete(&Expression::operation("eigenvalues", vec![realified]))?;
                Ok(Some(eigs))
            }

            #[cfg(feature = "numerical")]
            "cmat_schur" | "schur_complex" => {
                // Complex Schur decomposition via realification
                // schur_complex((A,B)) computes Schur of [[A,-B],[B,A]] then complexifies
                if args.len() != 1 {
                    return Ok(None);
                }
                // First realify
                let realified =
                    self.eval_concrete(&Expression::operation("realify", args.to_vec()))?;
                let schur_result =
                    self.eval_concrete(&Expression::operation("schur", vec![realified]))?;
                Ok(Some(schur_result))
            }

            #[cfg(feature = "numerical")]
            "eigenvalues" | "eigvals" => self.lapack_eigenvalues(args),

            #[cfg(feature = "numerical")]
            "eig" => self.lapack_eig(args),

            #[cfg(feature = "numerical")]
            "svd" => self.lapack_svd(args),

            #[cfg(feature = "numerical")]
            "singular_values" | "svdvals" => self.lapack_singular_values(args),

            #[cfg(feature = "numerical")]
            "solve" | "linsolve" => self.lapack_solve(args),

            #[cfg(feature = "numerical")]
            "inv" | "inverse" => self.lapack_inv(args),

            #[cfg(feature = "numerical")]
            "qr" => self.lapack_qr(args),

            #[cfg(feature = "numerical")]
            "cholesky" | "chol" => self.lapack_cholesky(args),

            #[cfg(feature = "numerical")]
            "rank" | "matrix_rank" => self.lapack_rank(args),

            #[cfg(feature = "numerical")]
            "cond" | "condition_number" => self.lapack_cond(args),

            #[cfg(feature = "numerical")]
            "norm" | "matrix_norm" => self.lapack_norm(args),

            #[cfg(feature = "numerical")]
            "det_lapack" => {
                // Use LAPACK determinant for large matrices (>3x3)
                self.lapack_det(args)
            }

            #[cfg(feature = "numerical")]
            "schur" | "schur_decomp" => self.lapack_schur(args),

            #[cfg(feature = "numerical")]
            "care" | "riccati" => self.lapack_care(args),

            #[cfg(feature = "numerical")]
            "lqr" => self.lapack_lqr(args),

            #[cfg(feature = "numerical")]
            "dare" | "riccati_discrete" => self.lapack_dare(args),

            #[cfg(feature = "numerical")]
            "dlqr" => self.lapack_dlqr(args),

            #[cfg(feature = "numerical")]
            "expm" | "matrix_exp" => {
                // Matrix exponential exp(A)
                if args.len() != 1 {
                    return Ok(None);
                }
                if let Some((m, n, elements)) = self.extract_matrix(&args[0]) {
                    if m != n {
                        return Err(format!("expm requires a square matrix, got {}×{}", m, n));
                    }
                    let data: Result<Vec<f64>, _> = elements
                        .iter()
                        .map(|e| {
                            self.as_number(e)
                                .ok_or_else(|| "Symbolic elements not supported".to_string())
                        })
                        .collect();
                    let data = data?;

                    let result = crate::numerical::expm(&data, n).map_err(|e| e.to_string())?;

                    let result_exprs: Vec<Expression> =
                        result.iter().map(|&v| Self::const_from_f64(v)).collect();
                    Ok(Some(self.make_matrix(n, n, result_exprs)))
                } else {
                    Ok(None)
                }
            }

            "ode45" | "integrate" => self.builtin_ode45(args),

            "mpow" | "matrix_pow" => {
                // Matrix power A^k for integer k
                if args.len() != 2 {
                    return Ok(None);
                }
                if let (Some((m, n, _elements)), Some(k)) =
                    (self.extract_matrix(&args[0]), self.as_integer(&args[1]))
                {
                    if m != n {
                        return Err(format!("mpow requires a square matrix, got {}×{}", m, n));
                    }
                    #[cfg(feature = "numerical")]
                    if k < 0 {
                        // For negative powers, compute inv(A)^|k|
                        let inv_result = self
                            .eval_concrete(&Expression::operation("inv", vec![args[0].clone()]))?;
                        return self
                            .eval_concrete(&Expression::operation(
                                "mpow",
                                vec![inv_result, Expression::Const(format!("{}", -k))],
                            ))
                            .map(Some);
                    }
                    #[cfg(not(feature = "numerical"))]
                    if k < 0 {
                        return Err(
                            "mpow with negative exponent requires 'numerical' feature".to_string()
                        );
                    }
                    if k == 0 {
                        // A^0 = I
                        return self
                            .eval_concrete(&Expression::operation(
                                "eye",
                                vec![Expression::Const(format!("{}", n))],
                            ))
                            .map(Some);
                    }
                    if k == 1 {
                        return Ok(Some(args[0].clone()));
                    }

                    // Binary exponentiation
                    let mut result = self.eval_concrete(&Expression::operation(
                        "eye",
                        vec![Expression::Const(format!("{}", n))],
                    ))?;
                    let mut base = args[0].clone();
                    let mut exp = k as u64;

                    while exp > 0 {
                        if exp & 1 == 1 {
                            result = self
                                .apply_builtin("multiply", &[result.clone(), base.clone()])?
                                .unwrap_or(result);
                        }
                        base = self
                            .apply_builtin("multiply", &[base.clone(), base.clone()])?
                            .unwrap_or(base);
                        exp >>= 1;
                    }
                    Ok(Some(result))
                } else {
                    Ok(None)
                }
            }

            #[cfg(feature = "numerical")]
            "cmat_svd" => {
                // Complex SVD via realification
                // For M = (A,B), compute SVD of realify(M)
                if args.len() != 1 {
                    return Ok(None);
                }
                let realified =
                    self.eval_concrete(&Expression::operation("realify", args.to_vec()))?;
                let svd_result =
                    self.eval_concrete(&Expression::operation("svd", vec![realified]))?;
                Ok(Some(svd_result))
            }

            #[cfg(feature = "numerical")]
            "cmat_singular_values" | "cmat_svdvals" => {
                // Complex singular values via realification
                // Singular values of complex matrix = singular values of realified / sqrt(2)
                // (Actually each singular value appears twice in realified)
                if args.len() != 1 {
                    return Ok(None);
                }
                let realified =
                    self.eval_concrete(&Expression::operation("realify", args.to_vec()))?;
                let svs =
                    self.eval_concrete(&Expression::operation("singular_values", vec![realified]))?;
                Ok(Some(svs))
            }

            #[cfg(feature = "numerical")]
            "cmat_solve" | "cmat_linsolve" => {
                // Solve complex linear system (A+Bi)x = (c+di)
                // Using realification: [[A,-B],[B,A]][xr,xi]^T = [c,d]^T
                if args.len() != 2 {
                    return Ok(None);
                }
                // Get complex matrix and RHS
                if let (Some((_a, _b)), Some((c, d))) = (
                    self.extract_complex_matrix(&args[0]),
                    self.extract_complex_matrix(&args[1]),
                ) {
                    // Realify the matrix
                    let realified_mat = self
                        .eval_concrete(&Expression::operation("realify", vec![args[0].clone()]))?;
                    // Stack RHS: [c; d]
                    let rhs_stacked =
                        self.eval_concrete(&Expression::operation("vstack", vec![c, d]))?;
                    // Solve the real system
                    let sol = self.eval_concrete(&Expression::operation(
                        "solve",
                        vec![realified_mat, rhs_stacked],
                    ))?;
                    // Split solution into real and imaginary parts
                    // solve returns a List, not a Matrix
                    let sol_elems: Vec<Expression> = if let Expression::List(items) = &sol {
                        items.clone()
                    } else if let Some((_n2, _, elems)) = self.extract_matrix(&sol) {
                        elems
                    } else {
                        return Ok(Some(sol));
                    };
                    let n2 = sol_elems.len();
                    let n = n2 / 2;
                    let xr: Vec<_> = sol_elems[..n].to_vec();
                    let xi: Vec<_> = sol_elems[n..].to_vec();
                    let real_part = self.make_matrix(n, 1, xr);
                    let imag_part = self.make_matrix(n, 1, xi);
                    Ok(Some(Expression::List(vec![real_part, imag_part])))
                } else {
                    Ok(None)
                }
            }

            #[cfg(feature = "numerical")]
            "cmat_inv" | "cmat_inverse" => {
                // Complex matrix inverse via realification
                // inv((A,B)) = complexify(inv(realify((A,B))))
                if args.len() != 1 {
                    return Ok(None);
                }
                let realified =
                    self.eval_concrete(&Expression::operation("realify", args.to_vec()))?;
                let inv_real =
                    self.eval_concrete(&Expression::operation("inv", vec![realified]))?;
                let result =
                    self.eval_concrete(&Expression::operation("complexify", vec![inv_real]))?;
                Ok(Some(result))
            }

            #[cfg(feature = "numerical")]
            "cmat_qr" => {
                // Complex QR via realification
                // The Q and R of realified matrix can be complexified
                if args.len() != 1 {
                    return Ok(None);
                }
                let realified =
                    self.eval_concrete(&Expression::operation("realify", args.to_vec()))?;
                let qr_result =
                    self.eval_concrete(&Expression::operation("qr", vec![realified]))?;
                // Return QR of realified (user can complexify if needed)
                Ok(Some(qr_result))
            }

            #[cfg(feature = "numerical")]
            "cmat_rank" | "cmat_matrix_rank" => {
                // Complex matrix rank via realification
                // rank((A,B)) = rank(realify((A,B))) / 2
                if args.len() != 1 {
                    return Ok(None);
                }
                let realified =
                    self.eval_concrete(&Expression::operation("realify", args.to_vec()))?;
                let rank_real =
                    self.eval_concrete(&Expression::operation("rank", vec![realified]))?;
                // Divide by 2 since realification doubles the dimension
                if let Expression::Const(s) = &rank_real
                    && let Ok(r) = s.parse::<i64>()
                {
                    return Ok(Some(Expression::Const(format!("{}", r / 2))));
                }
                Ok(Some(rank_real))
            }

            #[cfg(feature = "numerical")]
            "cmat_cond" | "cmat_condition_number" => {
                // Complex condition number via realification
                // cond((A,B)) = cond(realify((A,B)))
                if args.len() != 1 {
                    return Ok(None);
                }
                let realified =
                    self.eval_concrete(&Expression::operation("realify", args.to_vec()))?;
                let cond = self.eval_concrete(&Expression::operation("cond", vec![realified]))?;
                Ok(Some(cond))
            }

            #[cfg(feature = "numerical")]
            "cmat_norm" | "cmat_matrix_norm" => {
                // Complex Frobenius norm: ||M||_F = sqrt(||A||_F^2 + ||B||_F^2)
                if args.len() != 1 {
                    return Ok(None);
                }
                if let Some((a, b)) = self.extract_complex_matrix(&args[0]) {
                    let norm_a = self.eval_concrete(&Expression::operation("norm", vec![a]))?;
                    let norm_b = self.eval_concrete(&Expression::operation("norm", vec![b]))?;
                    // ||M||_F = sqrt(||A||^2 + ||B||^2)
                    if let (Some(na), Some(nb)) = (self.as_number(&norm_a), self.as_number(&norm_b))
                    {
                        let norm = (na * na + nb * nb).sqrt();
                        return Ok(Some(Expression::Const(format!("{}", norm))));
                    }
                }
                Ok(None)
            }

            #[cfg(feature = "numerical")]
            "cmat_det" | "cmat_determinant" => {
                // Complex determinant via realification
                // |det(M)|^2 = det(realify(M))
                // So det(M) = sqrt(det(realify(M))) * phase
                // For now, return the magnitude squared
                if args.len() != 1 {
                    return Ok(None);
                }
                let realified =
                    self.eval_concrete(&Expression::operation("realify", args.to_vec()))?;
                let det_real =
                    self.eval_concrete(&Expression::operation("det_lapack", vec![realified]))?;
                // det_real = |det(M)|^2, return as (det_real, 0) to indicate it's real
                if let Some(d) = self.as_number(&det_real) {
                    // Take square root for magnitude (sign handling is complex)
                    let mag = d.abs().sqrt();
                    return Ok(Some(Expression::List(vec![
                        Expression::Const(format!("{}", mag)),
                        Expression::Const("0".to_string()),
                    ])));
                }
                Ok(Some(det_real))
            }

            #[cfg(feature = "numerical")]
            "cmat_eig" => {
                // Full complex eigendecomposition via realification
                // Returns eigenvalues and eigenvectors
                if args.len() != 1 {
                    return Ok(None);
                }
                let realified =
                    self.eval_concrete(&Expression::operation("realify", args.to_vec()))?;
                let eig_result =
                    self.eval_concrete(&Expression::operation("eig", vec![realified]))?;
                Ok(Some(eig_result))
            }

            #[cfg(feature = "numerical")]
            "cmat_expm" | "cmat_matrix_exp" => {
                // Complex matrix exponential via realification
                // exp((A,B)) = complexify(exp(realify((A,B))))
                if args.len() != 1 {
                    return Ok(None);
                }
                let realified =
                    self.eval_concrete(&Expression::operation("realify", args.to_vec()))?;
                let exp_real =
                    self.eval_concrete(&Expression::operation("expm", vec![realified]))?;
                let result =
                    self.eval_concrete(&Expression::operation("complexify", vec![exp_real]))?;
                Ok(Some(result))
            }

            #[cfg(feature = "numerical")]
            "cmat_mpow" | "cmat_matrix_pow" => {
                // Complex matrix power (A+Bi)^k for integer k
                if args.len() != 2 {
                    return Ok(None);
                }
                if let (Some(_), Some(k)) = (
                    self.extract_complex_matrix(&args[0]),
                    self.as_integer(&args[1]),
                ) {
                    if k < 0 {
                        // For negative powers, compute inv(M)^|k|
                        let inv_result = self.eval_concrete(&Expression::operation(
                            "cmat_inv",
                            vec![args[0].clone()],
                        ))?;
                        return self
                            .eval_concrete(&Expression::operation(
                                "cmat_mpow",
                                vec![inv_result, Expression::Const(format!("{}", -k))],
                            ))
                            .map(Some);
                    }
                    if k == 0 {
                        // M^0 = I (complex identity)
                        // Need to get the dimension first
                        if let Some((a, _b)) = self.extract_complex_matrix(&args[0])
                            && let Some((n, _, _)) = self.extract_matrix(&a)
                        {
                            return self
                                .apply_builtin("cmat_eye", &[Expression::Const(format!("{}", n))]);
                        }
                        return Ok(None);
                    }
                    if k == 1 {
                        return Ok(Some(args[0].clone()));
                    }

                    // Binary exponentiation
                    let dim = if let Some((a, _)) = self.extract_complex_matrix(&args[0]) {
                        if let Some((n, _, _)) = self.extract_matrix(&a) {
                            n
                        } else {
                            return Ok(None);
                        }
                    } else {
                        return Ok(None);
                    };

                    let mut result = self
                        .apply_builtin("cmat_eye", &[Expression::Const(format!("{}", dim))])?
                        .unwrap_or(Expression::Const("error".to_string()));
                    let mut base = args[0].clone();
                    let mut exp = k as u64;

                    while exp > 0 {
                        if exp & 1 == 1 {
                            result = self
                                .apply_builtin("cmat_mul", &[result.clone(), base.clone()])?
                                .unwrap_or(result);
                        }
                        base = self
                            .apply_builtin("cmat_mul", &[base.clone(), base.clone()])?
                            .unwrap_or(base);
                        exp >>= 1;
                    }
                    Ok(Some(result))
                } else {
                    Ok(None)
                }
            }

            #[cfg(feature = "numerical")]
            "ndarray_reshape" => self.ndarray_reshape(args),

            #[cfg(feature = "numerical")]
            "ndarray_contract" => self.ndarray_contract(args),

            #[cfg(feature = "numerical")]
            "ndarray_moveaxis" => self.ndarray_moveaxis(args),

            #[cfg(feature = "numerical")]
            "ndarray_flatten" => self.ndarray_flatten(args),

            #[cfg(feature = "numerical")]
            "dft" => self.builtin_dft(args),

            #[cfg(feature = "numerical")]
            "fft" => self.builtin_fft(args),

            #[cfg(feature = "numerical")]
            "idft" => self.builtin_idft(args),

            #[cfg(feature = "numerical")]
            "ifft" => self.builtin_ifft(args),

            // Not a built-in
            _ => Ok(None),
        }
    }

    /// Check if a name looks like a constructor (starts with uppercase)
    pub(crate) fn is_constructor_name(&self, name: &str) -> bool {
        name.chars()
            .next()
            .map(|c| c.is_uppercase())
            .unwrap_or(false)
    }

    fn builtin_arithmetic<F>(
        &self,
        args: &[Expression],
        op: F,
    ) -> Result<Option<Expression>, String>
    where
        F: Fn(f64, f64) -> f64,
    {
        if args.len() != 2 {
            return Ok(None);
        }
        if let (Some(a), Some(b)) = (self.as_number(&args[0]), self.as_number(&args[1])) {
            let result = op(a, b);
            // Format nicely: integers without decimal point
            if result.fract() == 0.0 && result.abs() < 1e15 {
                Ok(Some(Expression::Const(format!("{}", result as i64))))
            } else {
                Ok(Some(Expression::Const(format!("{}", result))))
            }
        } else {
            Ok(None)
        }
    }

    /// Concatenate strings/objects into a single string/object
    ///
    /// Usage: concat(a, b, c, ...)
    /// - Accepts String, Const, or Object
    /// - If any arg is Object, result is Object; otherwise String
    pub(crate) fn builtin_concat(&self, args: &[Expression]) -> Result<Option<Expression>, String> {
        if args.is_empty() {
            return Ok(None);
        }

        let mut parts = Vec::with_capacity(args.len());
        let mut has_object = false;

        for a in args {
            let v = self.eval_concrete(a)?;
            match v {
                Expression::String(s) | Expression::Const(s) => {
                    parts.push(s);
                }
                Expression::Object(s) => {
                    has_object = true;
                    parts.push(s);
                }
                other => {
                    return Err(format!(
                        "concat(): unsupported argument type {:?}, expected string/object",
                        other
                    ));
                }
            }
        }

        let joined = parts.join("");
        if has_object {
            Ok(Some(Expression::Object(joined)))
        } else {
            Ok(Some(Expression::String(joined)))
        }
    }

    /// String equality comparison
    ///
    /// Usage: str_eq(a, b)
    /// Returns: true if a == b (as strings), false otherwise
    fn builtin_str_eq(&self, args: &[Expression]) -> Result<Option<Expression>, String> {
        if args.len() != 2 {
            return Err("str_eq() requires exactly 2 arguments".to_string());
        }

        let a = self.eval_concrete(&args[0])?;
        let b = self.eval_concrete(&args[1])?;

        let a_str = match &a {
            Expression::String(s) | Expression::Const(s) | Expression::Object(s) => s.clone(),
            _ => return Ok(None), // Can't compare non-strings
        };

        let b_str = match &b {
            Expression::String(s) | Expression::Const(s) | Expression::Object(s) => s.clone(),
            _ => return Ok(None), // Can't compare non-strings
        };

        if a_str == b_str {
            Ok(Some(Expression::Object("true".to_string())))
        } else {
            Ok(Some(Expression::Object("false".to_string())))
        }
    }

    /// ODE solver using Dormand-Prince 5(4) method
    ///
    /// Usage: ode45(f, y0, t_span, dt?)
    ///   f: dynamics function (t, y) -> [dy/dt...]
    ///   y0: initial state, e.g., [1, 0]
    ///   t_span: [t0, t1]
    ///   dt: initial step (optional, default 0.1)
    ///
    /// Example:
    ///   // Harmonic oscillator: x'' = -x
    ///   let f = (t, y) => [y[1], neg(y[0])]
    ///   ode45(f, [1, 0], [0, 10], 0.1)
    ///
    /// Returns: list of [t, [y0, y1, ...]] pairs
    fn builtin_ode45(&self, args: &[Expression]) -> Result<Option<Expression>, String> {
        if args.len() < 3 || args.len() > 4 {
            return Err("ode45 requires 3-4 arguments: f, y0, t_span, dt?\n\
                 Example: ode45((t, y) => [y[1], neg(y[0])], [1, 0], [0, 10])"
                .to_string());
        }

        // args[0] should be a lambda: (t, y) => ...
        let f_lambda = &args[0];

        // Extract initial state y0
        let y0: Vec<f64> = if let Expression::List(elems) = &args[1] {
            elems
                .iter()
                .map(|e| {
                    self.as_number(e)
                        .ok_or_else(|| "y0 must be numeric".to_string())
                })
                .collect::<Result<Vec<_>, _>>()?
        } else {
            return Err("y0 must be a list".to_string());
        };

        // Extract time span [t0, t1]
        let (t0, t1) = if let Expression::List(elems) = &args[2] {
            if elems.len() != 2 {
                return Err("t_span must be [t0, t1]".to_string());
            }
            let t0 = self
                .as_number(&elems[0])
                .ok_or_else(|| "t0 must be numeric".to_string())?;
            let t1 = self
                .as_number(&elems[1])
                .ok_or_else(|| "t1 must be numeric".to_string())?;
            (t0, t1)
        } else {
            return Err("t_span must be [t0, t1]".to_string());
        };

        // Extract dt (optional)
        let dt = if args.len() == 4 {
            self.as_number(&args[3])
                .ok_or_else(|| "dt must be numeric".to_string())?
        } else {
            0.1
        };

        let dim = y0.len();
        let f_clone = f_lambda.clone();

        // We need the evaluator to properly evaluate complex lambda bodies.
        // Use a raw pointer to self - this is safe because:
        // 1. The closure is only called during integrate_dopri5
        // 2. self is valid for the entire duration of this function
        // 3. The closure doesn't escape builtin_ode45
        let eval_ptr = self as *const Evaluator;

        // Create dynamics function that calls the lambda
        let dynamics = move |t: f64, y: &[f64]| -> Vec<f64> {
            // Build: f(t, [y0, y1, ...])
            let t_expr = Expression::Const(format!("{}", t));
            let y_expr = Expression::List(
                y.iter()
                    .map(|&v| Expression::Const(format!("{}", v)))
                    .collect(),
            );

            // Apply lambda using the evaluator
            if let Expression::Lambda { params, .. } = &f_clone
                && params.len() >= 2
            {
                // SAFETY: eval_ptr points to self which is valid for this function's duration
                let evaluator = unsafe { &*eval_ptr };

                // Use beta reduction to apply lambda: (λ t y . body)(t_val, y_val)
                if let Ok(reduced) = evaluator.beta_reduce_multi(&f_clone, &[t_expr, y_expr]) {
                    // Evaluate the reduced expression
                    if let Ok(Expression::List(elems)) = evaluator.eval_concrete(&reduced) {
                        let nums: Option<Vec<f64>> = elems.iter().map(eval_numeric).collect();
                        if let Some(v) = nums {
                            return v;
                        }
                    }
                }
            }
            vec![0.0; dim]
        };

        // Integrate
        let result =
            crate::ode::integrate_dopri5(dynamics, &y0, (t0, t1), dt).map_err(|e| e.to_string())?;

        // Convert to Kleis list of [t, [y...]]
        let trajectory: Vec<Expression> = result
            .into_iter()
            .map(|(t, y)| {
                Expression::List(vec![
                    Expression::Const(format!("{}", t)),
                    Expression::List(
                        y.into_iter()
                            .map(|v| Expression::Const(format!("{}", v)))
                            .collect(),
                    ),
                ])
            })
            .collect();

        Ok(Some(Expression::List(trajectory)))
    }
}
