use crate::ast::{AstNode, Expr, ParamList};
use crate::macros::MacroTemplate;
use crate::SutraError;
use crate::err_msg;

/// Returns true if the given expression is a macro definition of the form (define ...).
pub fn is_macro_definition(expr: &AstNode) -> bool {
    let Expr::List(items, _) = &*expr.value else {
        return false;
    };
    if items.len() != 3 {
        return false;
    }
    let Expr::Symbol(def, _) = &*items[0].value else {
        return false;
    };
    def == "define"
}

/// Parses a macro definition AST node into a (name, MacroTemplate) pair.
pub fn parse_macro_definition(expr: &AstNode) -> Result<(String, MacroTemplate), SutraError> {
    let Expr::List(items, _) = &*expr.value else {
        return Err(err_msg!(Internal, "Not a macro definition list."));
    };
    if items.len() != 3 {
        return Err(err_msg!(Internal, "Macro definition must have 3 elements."));
    }
    let Expr::Symbol(def, _) = &*items[0].value else {
        return Err(err_msg!(Internal, "First element must be 'define'."));
    };
    if def != "define" {
        return Err(err_msg!(Internal, "First element must be 'define'."));
    }
    let Expr::ParamList(param_list) = &*items[1].value else {
        return Err(err_msg!(Internal, "Second element must be a parameter list."));
    };
    let macro_name = param_list
        .required
        .first()
        .cloned()
        .ok_or_else(|| err_msg!(Internal, "Macro name missing in parameter list."))?;
    let params = ParamList {
        required: param_list.required[1..].to_vec(),
        rest: param_list.rest.clone(),
        span: param_list.span,
    };
    let template = MacroTemplate::new(params, Box::new(items[2].clone()))?;
    Ok((macro_name, template))
}