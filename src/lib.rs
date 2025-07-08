pub mod syntax;
pub mod ast;
pub mod atoms;
pub mod macros;
pub mod runtime;
pub mod cli;

use crate::ast::{Expr, Span, WithSpan};
use crate::atoms::OutputSink;
use crate::cli::output::StdoutSink;
use crate::syntax::error::macro_error;
use crate::syntax::error::SutraError;
use crate::runtime::eval::eval_expr;
use crate::runtime::eval::EvalOptions;
use crate::macros::{expand_macros, MacroDef, MacroEnv, MacroRegistry, MacroTemplate};
use crate::runtime::registry::build_default_atom_registry;
use crate::syntax::validate::validate;
use crate::runtime::world::World;

/// New API: Run Sutra source with injectable output sink.
pub fn run_sutra_source_with_output(
    source: &str,
    output: &mut dyn OutputSink,
) -> Result<(), SutraError> {
    // 1. Parse the source into AST nodes
    let ast_nodes = syntax::parser::parse(source).map_err(|e| e.with_source(source))?;

    // 2. Partition AST nodes: macro definitions vs user code
    let (macro_defs, user_code): (Vec<_>, Vec<_>) =
        ast_nodes.into_iter().partition(is_macro_definition);

    // 3. Build macro registry from macro_defs
    let mut user_macros = MacroRegistry::new();
    for macro_expr in macro_defs {
        let (name, template) = parse_macro_definition(&macro_expr)?;
        if user_macros.macros.contains_key(&name) {
            return Err(macro_error(
                format!("Duplicate macro name '{}'.", name),
                None,
            ));
        }
        user_macros
            .macros
            .insert(name, MacroDef::Template(template));
    }

    // 4. Build core macro registry (standard macros)
    let mut core_macros = MacroRegistry::new();
    macros::std::register_std_macros(&mut core_macros);

    // 5. Build MacroEnv
    let mut env = MacroEnv {
        user_macros: user_macros.macros,
        core_macros: core_macros.macros,
        trace: Vec::new(),
    };

    // 6. Wrap user_code in a (do ...) if needed
    let program = wrap_in_do(user_code);

    // 7. Expand macros
    let expanded = expand_macros(program, &mut env).map_err(|e| {
        output.emit(&format!("Macro expansion error: {:?}", e), None);
        macro_error(format!("Macro expansion error: {:?}", e), None)
    })?;

    // 8. Validation step
    let atom_registry = build_default_atom_registry();
    validate(&expanded, &env, &atom_registry).inspect_err(|e| {
        let span = match e {
            SutraError {
                span: Some(span), ..
            } => Some(span.clone()),
            _ => None,
        };
        output.emit(&e.to_string(), span.as_ref());
    })?;

    // 9. Evaluate the expanded AST
    let world = World::default();
    let opts = EvalOptions {
        max_depth: 1000,
        atom_registry,
    };
    let result = eval_expr(&expanded, &world, output, &opts, 0).map_err(|e| {
        output.emit(&format!("Evaluation error: {:?}", e), None);
        e
    })?;

    // 10. Print evaluation result
    if !matches!(result.0, crate::ast::value::Value::Nil) {
        output.emit(&format!("{}", result.0), None);
    }

    Ok(())
}

/// Original API: Run Sutra source and print output to stdout.
pub fn run_sutra_source(source: &str, _filename: Option<&str>) -> Result<(), SutraError> {
    let mut stdout_sink = StdoutSink;
    run_sutra_source_with_output(source, &mut stdout_sink)
}

fn is_macro_definition(expr: &WithSpan<Expr>) -> bool {
    match &expr.value {
        Expr::List(items, _) if items.len() == 3 => {
            if let Expr::Symbol(def, _) = &items[0].value {
                def == "define"
            } else {
                false
            }
        }
        _ => false,
    }
}

fn parse_macro_definition(expr: &WithSpan<Expr>) -> Result<(String, MacroTemplate), SutraError> {
    use crate::ast::Expr;
    use crate::macros::MacroTemplate;
    let Expr::List(items, _) = &expr.value else {
        return Err(macro_error("Not a macro definition list.", None));
    };
    if items.len() != 3 {
        return Err(macro_error("Macro definition must have 3 elements.", None));
    }
    let Expr::Symbol(def, _) = &items[0].value else {
        return Err(macro_error("First element must be 'define'.", None));
    };
    if def != "define" {
        return Err(macro_error("First element must be 'define'.", None));
    }
    let Expr::ParamList(param_list) = &items[1].value else {
        return Err(macro_error(
            "Second element must be a parameter list.",
            None,
        ));
    };
    let macro_name = param_list
        .required
        .first()
        .cloned()
        .ok_or_else(|| macro_error("Macro name missing in parameter list.", None))?;
    let params = crate::ast::ParamList {
        required: param_list.required[1..].to_vec(),
        rest: param_list.rest.clone(),
        span: param_list.span.clone(),
    };
    let template = MacroTemplate::new(params, Box::new(items[2].clone()))?;
    Ok((macro_name, template))
}

fn wrap_in_do(exprs: Vec<WithSpan<Expr>>) -> WithSpan<Expr> {
    match exprs.len() {
        0 => WithSpan {
            value: Expr::List(vec![], Span { start: 0, end: 0 }),
            span: Span { start: 0, end: 0 },
        },
        1 => exprs
            .into_iter()
            .next()
            .expect("wrap_in_do: exprs should have at least one element"),
        _ => {
            let span = Span {
                start: exprs
                    .first()
                    .expect("wrap_in_do: exprs should have at least one element")
                    .span
                    .start,
                end: exprs
                    .last()
                    .expect("wrap_in_do: exprs should have at least one element")
                    .span
                    .end,
            };
            let do_symbol = WithSpan {
                value: Expr::Symbol("do".to_string(), span.clone()),
                span: span.clone(),
            };
            let mut items = Vec::with_capacity(exprs.len() + 1);
            items.push(do_symbol);
            items.extend(exprs);
            WithSpan {
                value: Expr::List(items, span.clone()),
                span,
            }
        }
    }
}
