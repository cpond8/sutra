// Test execution engine and phases for Sutra test harness.
use super::test_discovery::TestCase;
use sutra::ast::{AstNode, Expr, Span, WithSpan};
use sutra::cli::output::OutputBuffer;
use sutra::macros::{expand_macros, MacroDef, MacroTemplate};
use sutra::runtime::eval::eval;
use sutra::runtime::registry::build_default_atom_registry;
use sutra::runtime::world::World;
use sutra::syntax::parser;
use std::fmt;
use std::sync::Arc;

pub enum TestResult {
    Pass { file: String, name: String },
    Fail { file: String, name: String, error: String, expanded: Option<String>, eval: Option<String> },
    Skipped { file: String, name: String, reason: String },
}

pub struct PhaseState {
    pub world: World,
    pub atom_registry: sutra::atoms::AtomRegistry,
    pub output_sink: OutputBuffer,
    pub expanded: Option<String>,
    pub eval: Option<String>,
}

pub enum SutraTestError {
    Setup(String),
    Parse(String),
    MacroDef(String),
    MacroExpand(String, Option<String>),
    Eval(String, Option<String>, Option<String>),
}

// --- Private helpers for macro detection and wrapping ---
fn is_macro_definition(node: &AstNode) -> bool {
    if let Expr::List(ref list, _) = *node.value {
        if let Some(first) = list.first() {
            if let Expr::Symbol(ref sym, _) = *first.value {
                return sym == "macro";
            }
        }
    }
    false
}

fn wrap_in_do_if_needed(nodes: Vec<AstNode>, _input: &str) -> AstNode {
    if nodes.len() == 1 {
        nodes.into_iter().next().unwrap()
    } else {
        let span = Span::default();
        let mut list = Vec::with_capacity(nodes.len() + 1);
        list.push(AstNode::from(Expr::Symbol("do".to_string(), span)));
        list.extend(nodes);
        AstNode::new(Arc::new(Expr::List(list, span)), span)
    }
}

pub fn run_test_case(file: String, case: TestCase, eval_limit: usize) -> TestResult {
    fn matches_error_code(error_msg: &str, expected_code: &str) -> bool {
        match expected_code {
            "ARITY_ERROR" => error_msg.contains("Arity error:"),
            "TYPE_ERROR" => error_msg.contains("expected") && error_msg.contains("got"),
            "DIVISION_BY_ZERO" => error_msg.contains("division by zero"),
            "PARSE_ERROR" => error_msg.contains("Parse Error:"),
            "MACRO_ERROR" => error_msg.contains("Macro Error:"),
            "VALIDATION_ERROR" => error_msg.contains("Validation Error:"),
            "EVAL_ERROR" => error_msg.contains("Evaluation Error:"),
            _ => false,
        }
    }
    fn make_error_result(
        error_msg: String,
        case: &TestCase,
        file: &str,
        expanded: Option<String>,
        eval: Option<String>,
    ) -> TestResult {
        let matches = if let Some(expected_code) = case.expect_error_code.as_deref() {
            matches_error_code(&error_msg, expected_code)
        } else if let Some(expected) = case.expect_error.as_deref() {
            error_msg.contains(expected)
        } else {
            false
        };
        if matches {
            TestResult::Pass {
                file: file.to_string(),
                name: case.name.clone(),
            }
        } else {
            TestResult::Fail {
                file: file.to_string(),
                name: case.name.clone(),
                error: error_msg,
                expanded,
                eval,
            }
        }
    }
    fn format_output_mismatch(expected: &str, actual: &str) -> String {
        format!(
            "Output did not match expected\n  Expected: {}\n  Actual:   {}",
            expected.trim(),
            actual.trim()
        )
    }
    fn expected_error(case: &TestCase) -> bool {
        case.expect_error.is_some() || case.expect_error_code.is_some()
    }
    fn expected_error_message(case: &TestCase, val: &impl fmt::Debug) -> String {
        if let Some(err) = case.expect_error.as_deref() {
            format!("Expected error '{}' but evaluation succeeded with result: {:?}", err, val)
        } else if let Some(code) = case.expect_error_code.as_deref() {
            format!("Expected error code '{}' but evaluation succeeded with result: {:?}", code, val)
        } else {
            String::new()
        }
    }
    fn setup_env_phase() -> Result<PhaseState, SutraTestError> {
        let mut world = match World::default() {
            w => w,
        };
        world.macros = match sutra::runtime::registry::build_canonical_macro_env() {
            Ok(macros) => macros,
            Err(e) => return Err(SutraTestError::Setup(format!("Setup error: {}", e))),
        };
        let atom_registry = build_default_atom_registry();
        let output_sink = OutputBuffer::default();
        Ok(PhaseState {
            world,
            atom_registry,
            output_sink,
            expanded: None,
            eval: None,
        })
    }
    fn parse_phase(state: PhaseState, case: &TestCase) -> Result<(PhaseState, Vec<AstNode>), SutraTestError> {
        match parser::parse(&case.input) {
            Ok(nodes) => Ok((state, nodes)),
            Err(e) => Err(SutraTestError::Parse(format!("Parse error: {}", e))),
        }
    }
    fn macro_phase(mut state: PhaseState, macro_defs: Vec<AstNode>, case: &TestCase, file: &str) -> Result<PhaseState, SutraTestError> {
        for macro_expr in macro_defs {
            match parse_macro_definition(&macro_expr) {
                Ok((name, template)) => {
                    let macro_template = MacroTemplate::from(template);
                    state.world.macros.user_macros.insert(name, MacroDef::Template(macro_template));
                }
                Err(e) => {
                    return Err(SutraTestError::MacroDef(format!("Macro definition error: {}", e)));
                }
            }
        }
        Ok(state)
    }
    fn expand_phase(mut state: PhaseState, program: AstNode, case: &TestCase, file: &str) -> Result<PhaseState, SutraTestError> {
        match expand_macros(program, &mut state.world.macros) {
            Ok(expanded) => {
                state.expanded = Some(expanded.value.pretty());
                Ok(state)
            }
            Err(e) => Err(SutraTestError::MacroExpand(format!("Macro expansion error: {}", e), state.expanded.clone())),
        }
    }
    fn eval_phase(
        mut state: PhaseState,
        expanded: AstNode,
        case: &TestCase,
        eval_limit: usize,
    ) -> Result<PhaseState, SutraTestError> {
        let eval_result = eval(
            &expanded,
            &mut state.world,
            &mut state.output_sink,
            &state.atom_registry,
            eval_limit,
        );
        state.eval = eval_result.as_ref().ok().map(|val| format!("{:?}", val));
        if let Ok(val) = &eval_result {
            if expected_error(case) {
                return Err(SutraTestError::Eval(
                    expected_error_message(case, val),
                    state.expanded.clone(),
                    state.eval.clone(),
                ));
            }
            return Ok(state);
        }
        Err(SutraTestError::Eval(
            format!("Eval error: {}", eval_result.unwrap_err()),
            state.expanded.clone(),
            state.eval.clone(),
        ))
    }
    fn compare_and_report(state: PhaseState, case: &TestCase, file: &str) -> TestResult {
        let actual_output = state.output_sink.as_str();
        let passed = match (case.expected.as_deref(), case.expect_error.as_deref(), case.expect_error_code.as_deref()) {
            (Some(expected), None, None) => actual_output.trim() == expected.trim(),
            _ => true, // error cases handled elsewhere
        };
        if passed {
            TestResult::Pass {
                file: file.to_string(),
                name: case.name.clone(),
            }
        } else {
            TestResult::Fail {
                file: file.to_string(),
                name: case.name.clone(),
                error: format_output_mismatch(case.expected.as_deref().unwrap_or(""), actual_output),
                expanded: state.expanded,
                eval: state.eval,
            }
        }
    }
    fn handle_error(err: SutraTestError, case: &TestCase, file: &str) -> TestResult {
        match err {
            SutraTestError::Setup(msg) => TestResult::Fail {
                file: file.to_string(),
                name: case.name.clone(),
                error: msg,
                expanded: None,
                eval: None,
            },
            SutraTestError::Parse(msg) => TestResult::Fail {
                file: file.to_string(),
                name: case.name.clone(),
                error: msg,
                expanded: None,
                eval: None,
            },
            SutraTestError::MacroDef(msg) => TestResult::Fail {
                file: file.to_string(),
                name: case.name.clone(),
                error: msg,
                expanded: None,
                eval: None,
            },
            SutraTestError::MacroExpand(msg, expanded) => TestResult::Fail {
                file: file.to_string(),
                name: case.name.clone(),
                error: msg,
                expanded,
                eval: None,
            },
            SutraTestError::Eval(msg, expanded, eval) => TestResult::Fail {
                file: file.to_string(),
                name: case.name.clone(),
                error: msg,
                expanded,
                eval,
            },
        }
    }
    // Main test case runner
    let result = setup_env_phase()
        .and_then(|state| {
            let (state, ast_nodes) = parse_phase(state, &case)?;
            let (macro_defs, user_code): (Vec<_>, Vec<_>) = ast_nodes.into_iter().partition(is_macro_definition);
            let state = macro_phase(state, macro_defs, &case, &file)?;
            let program = wrap_in_do_if_needed(user_code, &case.input);
            let state = expand_phase(state, program, &case, &file)?;
            // Re-parse expanded for eval
            let expanded_ast = parser::parse(&state.expanded.as_ref().unwrap()).map_err(|e| SutraTestError::Parse(format!("Parse error after macro expansion: {}", e)))?;
            let expanded = wrap_in_do_if_needed(expanded_ast, &case.input);
            let state = eval_phase(state, expanded, &case, eval_limit)?;
            Ok(state)
        });
    match result {
        Ok(state) => compare_and_report(state, &case, &file),
        Err(err) => handle_error(err, &case, &file),
    }
}

// --- Macro definition parser ---
fn parse_macro_definition(node: &AstNode) -> Result<(String, AstNode), String> {
    if let Expr::List(ref list, _) = *node.value {
        if list.len() < 3 {
            return Err("Macro definition must have at least 3 elements".to_string());
        }
        if let Expr::Symbol(ref name, _) = *list[1].value {
            let template = list[2].clone();
            Ok((name.clone(), template))
        } else {
            Err("Macro name must be a symbol".to_string())
        }
    } else {
        Err("Not a macro definition list".to_string())
    }
}