use crate::ast::{Stmt, StmtKind};
use crate::error::Warning;

/// Collect a warning for every declared identifier that is not a Latin word.
/// Only declaration sites are visited, so each name is reported at most once
/// per declaration. Results are sorted by line.
pub fn lint(program: &[Stmt]) -> Vec<Warning> {
    let mut out = Vec::new();
    for s in program {
        lint_stmt(s, &mut out);
    }
    out.sort_by_key(|w| w.line);
    out
}

/// Single-character names (`a`, `b`, `i`) are conventional counters and are
/// exempt; everything longer must be recognizable Latin.
fn check_name(name: &str, line: usize, out: &mut Vec<Warning>) {
    if name.chars().count() > 1 && !latinitas::is_latin(name) {
        out.push(Warning {
            message: format!("nomen '{name}' fortasse non Latinum est"),
            line,
        });
    }
}

fn lint_block(body: &[Stmt], out: &mut Vec<Warning>) {
    for s in body {
        lint_stmt(s, out);
    }
}

fn lint_stmt(s: &Stmt, out: &mut Vec<Warning>) {
    match &s.kind {
        StmtKind::Let { name, .. } => check_name(name, s.line, out),
        StmtKind::Func(f) => {
            check_name(&f.name, s.line, out);
            for (_, param) in &f.params {
                // Parameters are written in the `capit (...)` clause, which for a
                // header split across lines sits below the `munus`/name line;
                // report them at `def_line`, not the enclosing `Stmt.line`.
                check_name(param, f.def_line, out);
            }
            lint_block(&f.body, out);
        }
        StmtKind::If { then, else_, .. } => {
            lint_block(then, out);
            if let Some(eb) = else_ {
                lint_block(eb, out);
            }
        }
        StmtKind::While { body, .. } => lint_block(body, out),
        StmtKind::For { init, step, body, .. } => {
            lint_stmt(init, out);
            lint_stmt(step, out);
            lint_block(body, out);
        }
        // Assignments and expressions only *use* names declared elsewhere.
        StmtKind::Assign { .. }
        | StmtKind::Print(_)
        | StmtKind::ExprStmt(_)
        | StmtKind::Return(_)
        | StmtKind::Break
        | StmtKind::Continue => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::tokenize;
    use crate::parser::parse;

    fn lint_src(src: &str) -> Vec<Warning> {
        lint(&parse(&tokenize(src).unwrap()).unwrap())
    }

    #[test]
    fn single_letter_names_are_exempt() {
        let w = lint_src("ni a = °V; ni b = °I;");
        assert!(w.is_empty(), "single letters should not warn, got {w:?}");
    }

    #[test]
    fn flags_a_non_latin_variable() {
        let w = lint_src("ni result = °V;");
        assert_eq!(w.len(), 1);
        assert_eq!(w[0].line, 1);
        assert!(w[0].message.contains("result"));
    }

    #[test]
    fn accepts_a_latin_variable() {
        assert!(lint_src("ni numerus = °V;").is_empty());
    }

    #[test]
    fn flags_a_non_latin_function_name_and_parameter() {
        // `foobar` and `widget` are invented words. Avoid plausible-looking
        // English here: `compute` and `value` would be poor choices, because
        // `computare` and `valere` are real Latin verbs whose stems are in the
        // dictionary.
        let w = lint_src("munus foobar capit (ni widget) reddit ni { redde widget; }");
        let names: Vec<&str> = w.iter().map(|x| x.message.as_str()).collect();
        assert_eq!(w.len(), 2, "expected function + param warnings, got {names:?}");
        assert!(names.iter().any(|m| m.contains("foobar")));
        assert!(names.iter().any(|m| m.contains("widget")));
    }

    #[test]
    fn parameters_are_reported_on_the_definition_line() {
        // The header may be split across lines: `munus foobar` on line 1 and the
        // `capit (...)` clause — where the parameter is actually written — on
        // line 2. Reporting the parameter at the enclosing `Stmt.line` would
        // blame line 1. Every other lint test keeps the header on one line,
        // where the two coincide and the bug would be invisible.
        let src = "munus foobar\n\
                   capit (ni widget) reddit ni {\n\
                   ni result = widget;\n\
                   redde result;\n\
                   }";
        let w = lint_src(src);

        let param = w.iter().find(|x| x.message.contains("widget"))
            .expect("expected a warning for the parameter 'widget'");
        assert_eq!(param.line, 2, "parameter must be reported on the line it is written");

        // The function name still belongs to the `munus` line, and the body
        // declaration to its own line — so this really is pinning the parameter,
        // not a blanket line shift.
        let name = w.iter().find(|x| x.message.contains("foobar"))
            .expect("expected a warning for the function name 'foobar'");
        assert_eq!(name.line, 1);
        let local = w.iter().find(|x| x.message.contains("result"))
            .expect("expected a warning for the local 'result'");
        assert_eq!(local.line, 3);
    }

    #[test]
    fn reaches_declarations_inside_nested_blocks() {
        let src = "si (verus) { indem (verus) { ni result = °V; } }";
        let w = lint_src(src);
        assert_eq!(w.len(), 1);
        assert!(w[0].message.contains("result"));
    }

    #[test]
    fn reaches_declarations_inside_function_bodies() {
        let w = lint_src("munus summa reddit ni { ni result = °V; redde result; }");
        assert!(w.iter().any(|x| x.message.contains("result")));
    }

    #[test]
    fn ignores_assignments_to_already_declared_names() {
        // `result` is declared once and assigned once: exactly one warning.
        let w = lint_src("ni result = °V; result = °VI;");
        assert_eq!(w.len(), 1);
    }

    #[test]
    fn warnings_are_sorted_by_line() {
        // Every name here must genuinely be rejected. An earlier version used
        // `alpha` and `beta`, which are real Latin (borrowed Greek letter
        // names), so only ONE warning came back — and a 1-element vector is
        // trivially sorted, making the ordering assertion unfailable.
        let w = lint_src("ni result = °I;\nni foobar = °II;\nni widget = °III;");

        let lines: Vec<usize> = w.iter().map(|x| x.line).collect();
        assert_eq!(lines.len(), 3, "expected three warnings to order, got {w:?}");
        assert_eq!(lines, vec![1, 2, 3]);
    }

    #[test]
    fn reaches_declarations_inside_else_block() {
        // `then` is empty, so `result` can only be found if the walk visits
        // the `else_` arm. If a regression dropped `else_` handling from the
        // `If` case, this warning would silently disappear.
        let w = lint_src("si (falsus) { } aut { ni result = °V; }");
        assert_eq!(w.len(), 1, "expected exactly one warning, got {w:?}");
        assert!(w[0].message.contains("result"));
    }

    #[test]
    fn reaches_declarations_in_for_init() {
        // `foobar` is a deliberately non-Latin, multi-character init variable:
        // a normal single-letter counter (`i`) would be exempt and wouldn't
        // catch a regression that skipped the `init` field of `For`.
        let src = "dum (ni foobar = °I; foobar minor_est °X; foobar = foobar + °I) { }";
        let w = lint_src(src);
        assert_eq!(w.len(), 1, "expected exactly one warning, got {w:?}");
        assert!(w[0].message.contains("foobar"));
    }

    #[test]
    fn reaches_declarations_in_for_body() {
        // The counter `i` is single-letter and exempt; `widget` inside the
        // loop body is the only name that should be flagged.
        let src = "dum (ni i = °I; i minor_est °III; i = i + °I) { ni widget = °V; }";
        let w = lint_src(src);
        assert_eq!(w.len(), 1, "expected exactly one warning, got {w:?}");
        assert!(w[0].message.contains("widget"));
    }
}
