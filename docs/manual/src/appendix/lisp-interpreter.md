# Appendix: LISP Interpreter in Kleis

This appendix presents a LISP interpreter written entirely in Kleis. This is a **demonstration** of Kleis's power as a meta-language — proving that Kleis can parse and execute programs written in other languages.

> **Note:** This is a proof-of-concept, not a production LISP implementation. See [Known Limitations](#known-limitations) for details.

The interpreter includes:
- **Recursive descent S-expression parser**
- **Full evaluator** with special forms, arithmetic, comparisons, and list operations
- **Lexical closures** with `lambda`
- **Recursive functions** with `letrec`

## Running in the REPL

```
$ cargo run --bin repl
🧮 Kleis REPL v1.0.0

λ> :load examples/meta-programming/lisp_parser.kleis
✅ Loaded: 2 files, 60 functions, 15 structures, 5 data types

λ> :eval run("(+ 2 3)")
✅ VNum(5)

λ> :eval run("(* 4 5)")  
✅ VNum(20)

λ> :eval run("(if (< 3 5) 100 200)")
✅ VNum(100)

λ> :eval run("((lambda (x) (* x x)) 7)")
✅ VNum(49)

λ> :eval run("(let ((x 10)) (+ x 5))")
✅ VNum(15)
```

### Factorial

```
λ> :eval run("(letrec ((fact (lambda (n) (if (<= n 1) 1 (* n (fact (- n 1))))))) (fact 5))")
✅ VNum(120)
```

### Fibonacci

```
λ> :eval run("(letrec ((fib (lambda (n) (if (< n 2) n (+ (fib (- n 1)) (fib (- n 2))))))) (fib 10))")
✅ VNum(55)
```

---

## Complete Source Code

The complete LISP interpreter is in `examples/meta-programming/lisp_parser.kleis`. Below is the full implementation.

### Part 1: S-Expression Data Types

```kleis
import "stdlib/prelude.kleis"

// S-Expression: atoms and lists
data SExpr =
    SAtom(value: String)
  | SList(elements: List(SExpr))

// Parser result: success with remaining input, or error
data ParseResult =
    ParseOK(expr: SExpr, rest: String)
  | ParseErr(message: String)
```

### Part 2: Parser Helper Functions

```kleis
// Check if character is whitespace
define is_ws(c: String) : Bool =
    or(eq(c, " "), or(eq(c, "\n"), eq(c, "\t")))

// Check if character is a delimiter
define is_delim(c: String) : Bool =
    or(is_ws(c), or(eq(c, "("), eq(c, ")")))

// Skip leading whitespace
define skip_ws(s: String) : String =
    if le(strlen(s), 0) then s
    else if is_ws(charAt(s, 0)) then skip_ws(substr(s, 1, strlen(s) - 1))
    else s

// Read atom characters until delimiter
define read_atom(s: String, acc: String) : ParseResult =
    if le(strlen(s), 0) then ParseOK(SAtom(acc), "")
    else if is_delim(charAt(s, 0)) then ParseOK(SAtom(acc), s)
    else read_atom(substr(s, 1, strlen(s) - 1), concat(acc, charAt(s, 0)))
```

### Part 3: Recursive Descent Parser

```kleis
// Parse a single S-expression
define parse_sexpr(s: String) : ParseResult =
    let trimmed = skip_ws(s) in
    if le(strlen(trimmed), 0) then ParseErr("Unexpected end of input")
    else if eq(charAt(trimmed, 0), "(") then 
        parse_list(substr(trimmed, 1, strlen(trimmed) - 1), Nil)
    else read_atom(trimmed, "")

// Parse list elements until ")"
define parse_list(s: String, acc: List(SExpr)) : ParseResult =
    let trimmed = skip_ws(s) in
    if le(strlen(trimmed), 0) then ParseErr("Expected ')'")
    else if eq(charAt(trimmed, 0), ")") then 
        ParseOK(SList(rev(acc)), substr(trimmed, 1, strlen(trimmed) - 1))
    else 
        match parse_sexpr(trimmed) {
            ParseOK(expr, rest) => parse_list(rest, Cons(expr, acc))
          | ParseErr(msg) => ParseErr(msg)
        }

// Reverse a list
define rev(xs: List(SExpr)) : List(SExpr) =
    rev_acc(xs, Nil)

define rev_acc(xs: List(SExpr), acc: List(SExpr)) : List(SExpr) =
    match xs {
        Nil => acc
      | Cons(h, t) => rev_acc(t, Cons(h, acc))
    }

// User-facing parse function
define parse(s: String) : SExpr =
    match parse_sexpr(s) {
        ParseOK(expr, rest) => expr
      | ParseErr(msg) => SAtom(concat("Error: ", msg))
    }
```

### Part 4: LISP Value Types and Environment

```kleis
// Values in our LISP
data LispVal =
    VNum(n: ℤ)                              // Integer
  | VSym(s: String)                         // Symbol (for errors/unbound)
  | VList(xs: List(LispVal))                // List value
  | VBool(b: Bool)                          // Boolean
  | VLambda(params: List(String), body: SExpr, env: Env)  // Closure

// Environment: list of (name, value) bindings
data Binding = Bind(name: String, val: LispVal)
data Env = Env(bindings: List(Binding))

// Empty environment
define empty_env : Env = Env(Nil)

// Look up a variable in the environment
define lookup(name: String, env: Env) : LispVal =
    match env {
        Env(bindings) => lookup_list(name, bindings)
    }

define lookup_list(name: String, bs: List(Binding)) : LispVal =
    match bs {
        Nil => VSym(concat("Unbound: ", name))
      | Cons(Bind(n, v), rest) => 
            if eq(n, name) then v else lookup_list(name, rest)
    }

// Extend environment with a new binding
define extend(name: String, val: LispVal, env: Env) : Env =
    match env {
        Env(bindings) => Env(Cons(Bind(name, val), bindings))
    }

// Extend with multiple bindings (for function application)
define extend_all(names: List(String), vals: List(LispVal), env: Env) : Env =
    match names {
        Nil => env
      | Cons(n, ns) => 
            match vals {
                Nil => env
              | Cons(v, vs) => extend_all(ns, vs, extend(n, v, env))
            }
    }
```

### Part 5: Integer Parsing

```kleis
define is_digit_char(c: String) : Bool =
    or(eq(c, "0"), or(eq(c, "1"), or(eq(c, "2"), or(eq(c, "3"), or(eq(c, "4"),
    or(eq(c, "5"), or(eq(c, "6"), or(eq(c, "7"), or(eq(c, "8"), eq(c, "9"))))))))))

define is_number_str(s: String) : Bool =
    if le(strlen(s), 0) then false
    else if eq(charAt(s, 0), "-") then 
        if le(strlen(s), 1) then false 
        else all_digits(substr(s, 1, strlen(s) - 1))
    else all_digits(s)

define all_digits(s: String) : Bool =
    if le(strlen(s), 0) then true
    else if is_digit_char(charAt(s, 0)) then all_digits(substr(s, 1, strlen(s) - 1))
    else false

define parse_int(s: String) : ℤ =
    if eq(charAt(s, 0), "-") then 0 - parse_int_pos(substr(s, 1, strlen(s) - 1))
    else parse_int_pos(s)

define parse_int_pos(s: String) : ℤ =
    parse_int_acc(s, 0)

define parse_int_acc(s: String, acc: ℤ) : ℤ =
    if le(strlen(s), 0) then acc
    else 
        let d = digit_val(charAt(s, 0)) in
        parse_int_acc(substr(s, 1, strlen(s) - 1), acc * 10 + d)

define digit_val(c: String) : ℤ =
    if eq(c, "0") then 0 else if eq(c, "1") then 1 else if eq(c, "2") then 2
    else if eq(c, "3") then 3 else if eq(c, "4") then 4 else if eq(c, "5") then 5
    else if eq(c, "6") then 6 else if eq(c, "7") then 7 else if eq(c, "8") then 8
    else 9
```

### Part 6: Main Evaluator

```kleis
define eval_lisp(expr: SExpr, env: Env) : LispVal =
    match expr {
        SAtom(s) => 
            if is_number_str(s) then VNum(parse_int(s))
            else if eq(s, "true") then VBool(true)
            else if eq(s, "false") then VBool(false)
            else lookup(s, env)
      | SList(elements) => eval_list(elements, env)
    }

define eval_list(elements: List(SExpr), env: Env) : LispVal =
    match elements {
        Nil => VList(Nil)  // Empty list is a value
      | Cons(head, rest) => eval_form(head, rest, env)
    }

// Evaluate a special form or function call
define eval_form(head: SExpr, args: List(SExpr), env: Env) : LispVal =
    match head {
        SAtom(op) => 
            // Special forms
            if eq(op, "if") then eval_if(args, env)
            else if eq(op, "quote") then eval_quote(args)
            else if eq(op, "lambda") then eval_lambda(args, env)
            else if eq(op, "let") then eval_let(args, env)
            else if eq(op, "letrec") then eval_letrec(args, env)
            // Arithmetic
            else if eq(op, "+") then eval_add(args, env)
            else if eq(op, "-") then eval_sub(args, env)
            else if eq(op, "*") then eval_mul(args, env)
            else if eq(op, "/") then eval_div(args, env)
            // Comparison
            else if eq(op, "<") then eval_lt(args, env)
            else if eq(op, ">") then eval_gt(args, env)
            else if eq(op, "=") then eval_eq(args, env)
            else if eq(op, "<=") then eval_le(args, env)
            else if eq(op, ">=") then eval_ge(args, env)
            // List operations
            else if eq(op, "list") then eval_list_op(args, env)
            else if eq(op, "car") then eval_car(args, env)
            else if eq(op, "cdr") then eval_cdr(args, env)
            else if eq(op, "cons") then eval_cons(args, env)
            else if eq(op, "null?") then eval_null(args, env)
            // Function call
            else eval_call(op, args, env)
      | SList(_) => 
            // First element is an expression (e.g., lambda)
            let fn_val = eval_lisp(head, env) in
            eval_apply(fn_val, args, env)
    }
```

### Part 7: Special Forms

```kleis
define eval_if(args: List(SExpr), env: Env) : LispVal =
    match args {
        Cons(cond, Cons(then_br, Cons(else_br, Nil))) =>
            let cv = eval_lisp(cond, env) in
            if is_truthy(cv) then eval_lisp(then_br, env) else eval_lisp(else_br, env)
      | _ => VSym("Error: if requires 3 arguments")
    }

define is_truthy(v: LispVal) : Bool =
    match v {
        VBool(b) => b
      | VNum(n) => not(eq(n, 0))
      | VList(Nil) => false
      | VList(_) => true
      | VSym(_) => false
      | VLambda(_, _, _) => true
    }

define eval_quote(args: List(SExpr)) : LispVal =
    match args {
        Cons(expr, Nil) => sexpr_to_val(expr)
      | _ => VSym("Error: quote requires 1 argument")
    }

define sexpr_to_val(expr: SExpr) : LispVal =
    match expr {
        SAtom(s) => if is_number_str(s) then VNum(parse_int(s)) else VSym(s)
      | SList(elements) => VList(map_sexpr_to_val(elements))
    }

define map_sexpr_to_val(xs: List(SExpr)) : List(LispVal) =
    match xs {
        Nil => Nil
      | Cons(h, t) => Cons(sexpr_to_val(h), map_sexpr_to_val(t))
    }

define eval_lambda(args: List(SExpr), env: Env) : LispVal =
    match args {
        Cons(SList(params), Cons(body, Nil)) =>
            VLambda(extract_param_names(params), body, env)
      | _ => VSym("Error: lambda requires (params) body")
    }

define extract_param_names(params: List(SExpr)) : List(String) =
    match params {
        Nil => Nil
      | Cons(SAtom(name), rest) => Cons(name, extract_param_names(rest))
      | _ => Nil
    }

define eval_let(args: List(SExpr), env: Env) : LispVal =
    match args {
        Cons(SList(bindings), Cons(body, Nil)) =>
            let new_env = eval_let_bindings(bindings, env) in
            eval_lisp(body, new_env)
      | _ => VSym("Error: let requires ((bindings)) body")
    }

define eval_let_bindings(bindings: List(SExpr), env: Env) : Env =
    match bindings {
        Nil => env
      | Cons(SList(Cons(SAtom(name), Cons(val_expr, Nil))), rest) =>
            let val = eval_lisp(val_expr, env) in
            eval_let_bindings(rest, extend(name, val, env))
      | _ => env
    }

// letrec: evaluate lambda in an environment that already contains the binding
// This enables recursion: (letrec ((fact (lambda (n) ...))) (fact 5))
define eval_letrec(args: List(SExpr), env: Env) : LispVal =
    match args {
        Cons(SList(bindings), Cons(body, Nil)) =>
            let rec_env = eval_letrec_bindings(bindings, env) in
            eval_lisp(body, rec_env)
      | _ => VSym("Error: letrec requires ((bindings)) body")
    }

define eval_letrec_bindings(bindings: List(SExpr), env: Env) : Env =
    match bindings {
        Nil => env
      | Cons(SList(Cons(SAtom(name), Cons(SList(Cons(SAtom(lambda_kw), 
            Cons(SList(params), Cons(body, Nil)))), Nil))), rest) =>
            let dummy_env = extend(name, VSym("placeholder"), env) in
            let lambda_val = VLambda(extract_param_names(params), body, dummy_env) in
            let new_env = extend(name, lambda_val, env) in
            let fixed_lambda = VLambda(extract_param_names(params), body, new_env) in
            eval_letrec_bindings(rest, extend(name, fixed_lambda, env))
      | _ => env
    }
```

### Part 8: Arithmetic Operations

```kleis
define eval_add(args: List(SExpr), env: Env) : LispVal =
    match args {
        Cons(a, Cons(b, Nil)) =>
            match eval_lisp(a, env) {
                VNum(x) => match eval_lisp(b, env) {
                    VNum(y) => VNum(x + y)
                  | _ => VSym("Error: + requires numbers")
                }
              | _ => VSym("Error: + requires numbers")
            }
      | _ => VSym("Error: + requires 2 arguments")
    }

define eval_sub(args: List(SExpr), env: Env) : LispVal =
    match args {
        Cons(a, Cons(b, Nil)) =>
            match eval_lisp(a, env) {
                VNum(x) => match eval_lisp(b, env) {
                    VNum(y) => VNum(x - y)
                  | _ => VSym("Error: - requires numbers")
                }
              | _ => VSym("Error: - requires numbers")
            }
      | _ => VSym("Error: - requires 2 arguments")
    }

define eval_mul(args: List(SExpr), env: Env) : LispVal =
    match args {
        Cons(a, Cons(b, Nil)) =>
            match eval_lisp(a, env) {
                VNum(x) => match eval_lisp(b, env) {
                    VNum(y) => VNum(x * y)
                  | _ => VSym("Error: * requires numbers")
                }
              | _ => VSym("Error: * requires numbers")
            }
      | _ => VSym("Error: * requires 2 arguments")
    }

define eval_div(args: List(SExpr), env: Env) : LispVal =
    match args {
        Cons(a, Cons(b, Nil)) =>
            match eval_lisp(a, env) {
                VNum(x) => match eval_lisp(b, env) {
                    VNum(y) => if eq(y, 0) then VSym("Error: division by zero") 
                               else VNum(x / y)
                  | _ => VSym("Error: / requires numbers")
                }
              | _ => VSym("Error: / requires numbers")
            }
      | _ => VSym("Error: / requires 2 arguments")
    }
```

### Part 9: Comparison Operations

```kleis
define eval_lt(args: List(SExpr), env: Env) : LispVal =
    match args {
        Cons(a, Cons(b, Nil)) =>
            match eval_lisp(a, env) {
                VNum(x) => match eval_lisp(b, env) {
                    VNum(y) => VBool(lt(x, y))
                  | _ => VSym("Error: < requires numbers")
                }
              | _ => VSym("Error: < requires numbers")
            }
      | _ => VSym("Error: < requires 2 arguments")
    }

define eval_gt(args: List(SExpr), env: Env) : LispVal =
    match args {
        Cons(a, Cons(b, Nil)) =>
            match eval_lisp(a, env) {
                VNum(x) => match eval_lisp(b, env) {
                    VNum(y) => VBool(gt(x, y))
                  | _ => VSym("Error: > requires numbers")
                }
              | _ => VSym("Error: > requires numbers")
            }
      | _ => VSym("Error: > requires 2 arguments")
    }

define eval_eq(args: List(SExpr), env: Env) : LispVal =
    match args {
        Cons(a, Cons(b, Nil)) =>
            match eval_lisp(a, env) {
                VNum(x) => match eval_lisp(b, env) {
                    VNum(y) => VBool(eq(x, y))
                  | _ => VSym("Error: = requires numbers")
                }
              | _ => VSym("Error: = requires numbers")
            }
      | _ => VSym("Error: = requires 2 arguments")
    }

define eval_le(args: List(SExpr), env: Env) : LispVal =
    match args {
        Cons(a, Cons(b, Nil)) =>
            match eval_lisp(a, env) {
                VNum(x) => match eval_lisp(b, env) {
                    VNum(y) => VBool(le(x, y))
                  | _ => VSym("Error: <= requires numbers")
                }
              | _ => VSym("Error: <= requires numbers")
            }
      | _ => VSym("Error: <= requires 2 arguments")
    }

define eval_ge(args: List(SExpr), env: Env) : LispVal =
    match args {
        Cons(a, Cons(b, Nil)) =>
            match eval_lisp(a, env) {
                VNum(x) => match eval_lisp(b, env) {
                    VNum(y) => VBool(ge(x, y))
                  | _ => VSym("Error: >= requires numbers")
                }
              | _ => VSym("Error: >= requires numbers")
            }
      | _ => VSym("Error: >= requires 2 arguments")
    }
```

### Part 10: List Operations

```kleis
define eval_list_op(args: List(SExpr), env: Env) : LispVal =
    VList(eval_all(args, env))

define eval_all(args: List(SExpr), env: Env) : List(LispVal) =
    match args {
        Nil => Nil
      | Cons(h, t) => Cons(eval_lisp(h, env), eval_all(t, env))
    }

define eval_car(args: List(SExpr), env: Env) : LispVal =
    match args {
        Cons(lst, Nil) =>
            match eval_lisp(lst, env) {
                VList(Cons(h, _)) => h
              | _ => VSym("Error: car requires non-empty list")
            }
      | _ => VSym("Error: car requires 1 argument")
    }

define eval_cdr(args: List(SExpr), env: Env) : LispVal =
    match args {
        Cons(lst, Nil) =>
            match eval_lisp(lst, env) {
                VList(Cons(_, t)) => VList(t)
              | _ => VSym("Error: cdr requires non-empty list")
            }
      | _ => VSym("Error: cdr requires 1 argument")
    }

define eval_cons(args: List(SExpr), env: Env) : LispVal =
    match args {
        Cons(h, Cons(t, Nil)) =>
            let hv = eval_lisp(h, env) in
            match eval_lisp(t, env) {
                VList(lst) => VList(Cons(hv, lst))
              | _ => VSym("Error: cons requires list as second arg")
            }
      | _ => VSym("Error: cons requires 2 arguments")
    }

define eval_null(args: List(SExpr), env: Env) : LispVal =
    match args {
        Cons(lst, Nil) =>
            match eval_lisp(lst, env) {
                VList(Nil) => VBool(true)
              | VList(_) => VBool(false)
              | _ => VSym("Error: null? requires list")
            }
      | _ => VSym("Error: null? requires 1 argument")
    }
```

### Part 11: Function Application

```kleis
define eval_call(name: String, args: List(SExpr), env: Env) : LispVal =
    let fn_val = lookup(name, env) in
    eval_apply(fn_val, args, env)

define eval_apply(fn_val: LispVal, args: List(SExpr), env: Env) : LispVal =
    match fn_val {
        VLambda(params, body, closure_env) =>
            let arg_vals = eval_all(args, env) in
            // Merge current env into closure env for recursive calls
            let merged_env = merge_envs(env, closure_env) in
            let new_env = extend_all(params, arg_vals, merged_env) in
            eval_lisp(body, new_env)
      | VSym(msg) => VSym(msg)  // Error propagation
      | _ => VSym("Error: not a function")
    }

// Merge two environments: first takes precedence
// This allows letrec functions to see their own definitions
define merge_envs(e1: Env, e2: Env) : Env =
    match e1 {
        Env(b1) => match e2 {
            Env(b2) => Env(append_bindings(b1, b2))
        }
    }

define append_bindings(b1: List(Binding), b2: List(Binding)) : List(Binding) =
    match b1 {
        Nil => b2
      | Cons(h, t) => Cons(h, append_bindings(t, b2))
    }
```

### Part 12: User-Facing Run Function

```kleis
// Run a LISP program from string
define run(code: String) : LispVal =
    eval_lisp(parse(code), empty_env)

// Run with an environment (for multiple expressions)
define run_with_env(code: String, env: Env) : LispVal =
    eval_lisp(parse(code), env)
```

---

## Known Limitations

This LISP interpreter is a **demonstration**, not a production-level implementation. It proves that Kleis can parse and execute programs, but has known limitations:

| Issue | Description | Status |
|-------|-------------|--------|
| **Trailing junk ignored** | `parse("(+ 1 2) garbage")` parses successfully, ignoring "garbage" | Known |
| **Errors as atoms** | Parse errors return `SAtom("Error: ...")` instead of a proper error type | Known |
| **No quote syntax** | Standard LISP `'(1 2 3)` is not supported; use `(list 1 2 3)` instead | Known |
| **Limited special forms** | Only `if`, `lambda`, `let`, `letrec`, `define` are implemented | By design |
| **No macros** | LISP macros are not supported | By design |
| **Integer-only arithmetic** | No floating-point numbers | By design |

### Why These Limitations Exist

This interpreter demonstrates **Kleis's meta-language capabilities**, not LISP completeness. The goal is to show:

1. Kleis can parse arbitrary grammars using recursive descent
2. Kleis can build and traverse ASTs (S-expressions as Kleis data types)
3. Kleis can execute recursive programs via `:eval`
4. Kleis is Turing-complete

For production LISP, use Racket, SBCL, or Clojure.

---

## Summary

This LISP interpreter demonstrates that **Kleis is Turing-complete** and can serve as a host language for other programming languages. The implementation uses:

| Feature | Kleis Construct |
|---------|-----------------|
| **Data types** | `data SExpr`, `data LispVal`, `data Env` |
| **Pattern matching** | `match expr { ... }` |
| **Recursion** | Recursive function definitions |
| **Higher-order functions** | `lambda`, closures with captured environments |
| **String operations** | `charAt`, `substr`, `concat`, `strlen` |
| **List operations** | `Cons`, `Nil`, pattern matching on lists |

### Key Insights

1. **`:eval` enables execution** — The `:eval` REPL command executes Kleis functions directly, without going through Z3's symbolic unrolling.

2. **Environment merging for recursion** — `letrec` works by merging the current environment (which contains the function binding) into the closure's environment.

3. **60 pure functions** — The entire interpreter is implemented in ~560 lines of pure functional Kleis code.

4. **Meta-circular potential** — With minor extensions, this could interpret a subset of Kleis itself, demonstrating meta-circularity.

