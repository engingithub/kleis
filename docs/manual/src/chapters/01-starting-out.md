# Starting Out

## Your First Kleis Expression

The simplest things in Kleis are **expressions**. An expression is anything that has a value:

```kleis
define answer = 42              // A number
define pi_approx = 3.14159      // A decimal
define sum(x, y) = x + y        // An arithmetic expression
define angle_sin(θ) = sin(θ)    // A function call
```

## The REPL

The easiest way to experiment with Kleis is the **REPL** (Read-Eval-Print Loop):

```bash
$ cargo run --bin repl
🧮 Kleis REPL v1.0.0
   Type :help for commands, :quit to exit

λ> 2 + 2
2 + 2

λ>  let x = 5 in x * x
times(5, 5)
```

## Basic Arithmetic

Kleis supports the usual arithmetic operations:

```kleis
define add_example = 2 + 3       // Addition: 5
define sub_example = 10 - 4      // Subtraction: 6
define mul_example = 3 * 7       // Multiplication: 21
define div_example = 15 / 3      // Division: 5
define pow_example = 2 ^ 10      // Exponentiation: 1024
```

## Variables and Definitions

Use `define` to create named values:

```kleis
define pi = 3.14159
define e = 2.71828
define golden_ratio = (1 + sqrt(5)) / 2
```

Functions are defined similarly:

```kleis
define square(x) = x * x
define cube(x) = x * x * x
define area_circle(r) = pi * r^2
```

## Comments

Kleis uses C-style comments:

```kleis
// This is a single-line comment
define x = 42  // Inline comment

/* 
   Multi-line comments
   use slash-star syntax
*/
```

## Unicode Support

Kleis embraces mathematical notation with full Unicode support:

```kleis example
// Greek letters
define α = 0.5
define β = 1.0
define θ = π / 4

// Mathematical symbols in axioms
axiom reflexivity : ∀(x : ℝ). x = x           // Universal quantifier
axiom positive_exists : ∃(y : ℝ). y > 0       // Existential quantifier
```

You can use ASCII alternatives too:

| Unicode | ASCII Alternative |
|---------|-------------------|
| `∀`     | `forall`          |
| `∃`     | `exists`          |
| `→`     | `->`              |


## What's Next?

Now that you can write basic expressions, let's learn about the type system!

→ [Next: Types and Values](./02-types.md)
