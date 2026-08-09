use crate::ast::{ParsedProgram, Stmt, StmtKind};
use crate::error::Warning;
use crate::spatium::Spatium;

/// One warning per declaration site whose name is not a Latin word, sorted by line.
pub fn lint(program: &[Stmt]) -> Vec<Warning> {
    let mut out = Vec::new();
    for s in program {
        lint_stmt(s, &mut out);
    }
    out.sort_by_key(|w| w.line);
    out
}

/// Package folder names (§4.2) first, then every file's declarations, attributed to it.
pub fn lint_program(prog: &ParsedProgram) -> Vec<Warning> {
    let mut out = Vec::new();
    for name in prog.packages.keys() {
        if name.chars().count() > 1 && !latinitas::is_latin(name) {
            out.push(Warning {
                message: format!("nomen fasciculi '{name}' fortasse non Latinum est"),
                file: None,
                line: None,
                spatium: None,
                fragmentum: None,
            });
        }
    }
    let package_files = prog.packages.values().flatten();
    for file in prog.root.iter().chain(package_files) {
        let mut ws = lint(&file.ast.funcs);
        for w in &mut ws {
            w.file = Some(file.path.clone());
            // Here and nowhere later: `strip` clears the file name off a
            // single-source warning further down the pipeline, so a renderer
            // resolving source by name would have nothing left to resolve.
            w.fragmentum = w.spatium.and_then(|s| crate::fragmentum::extract(&file.src, s));
        }
        out.extend(ws);
    }
    out
}

/// Single-character names are conventional counters and are exempt.
fn check_name(name: &str, name_span: Spatium, out: &mut Vec<Warning>) {
    if name.chars().count() > 1 && !latinitas::is_latin(name) {
        out.push(Warning {
            message: format!("nomen '{name}' fortasse non Latinum est"),
            file: None,
            line: Some(name_span.linea()),
            spatium: Some(name_span),
            fragmentum: None,
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
        StmtKind::Let { name, name_span, .. } => check_name(name, *name_span, out),
        StmtKind::Func(f) => {
            check_name(&f.name, f.name_span, out);
            for (_, param, span) in &f.params {
                // The parameter's own span, not the statement's: a split header
                // writes its parameters lower down than its `munus`.
                check_name(param, *span, out);
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
        | StmtKind::IndexAssign { .. }
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
    use crate::parser::parse_file;

    /// Lint a statement sequence through a one-line `munus initium` wrapper (D-19),
    /// so every asserted line number is unchanged.
    fn lint_src(src: &str) -> Vec<Warning> {
        let wrapped = format!("munus initium reddit ni {{ {src} }}");
        let ast = parse_file(&tokenize(&wrapped).unwrap()).unwrap();
        match ast.funcs.first().map(|s| &s.kind) {
            Some(crate::ast::StmtKind::Func(f)) => lint(&f.body),
            other => panic!("the wrapper did not produce one function: {other:?}"),
        }
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
        assert_eq!(w[0].line, Some(1));
        assert!(w[0].message.contains("result"));
    }

    #[test]
    fn accepts_a_latin_variable() {
        assert!(lint_src("ni numerus = °V;").is_empty());
    }

    #[test]
    fn flags_a_non_latin_function_name_and_parameter() {
        let w = lint_src("munus foobar capit (ni widget) reddit ni { redde widget; }");
        let names: Vec<&str> = w.iter().map(|x| x.message.as_str()).collect();
        assert_eq!(w.len(), 2, "expected function + param warnings, got {names:?}");
        assert!(names.iter().any(|m| m.contains("foobar")));
        assert!(names.iter().any(|m| m.contains("widget")));
    }

    #[test]
    fn parameters_are_reported_on_the_definition_line() {
        let src = "munus foobar\n\
                   capit (ni widget) reddit ni {\n\
                   ni result = widget;\n\
                   redde result;\n\
                   }";
        let w = lint_src(src);

        let param = w.iter().find(|x| x.message.contains("widget"))
            .expect("expected a warning for the parameter 'widget'");
        assert_eq!(param.line, Some(2), "parameter must be reported on the line it is written");

        let name = w.iter().find(|x| x.message.contains("foobar"))
            .expect("expected a warning for the function name 'foobar'");
        assert_eq!(name.line, Some(1));
        let local = w.iter().find(|x| x.message.contains("result"))
            .expect("expected a warning for the local 'result'");
        assert_eq!(local.line, Some(3));
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
        let w = lint_src("ni result = °V; result = °VI;");
        assert_eq!(w.len(), 1);
    }

    #[test]
    fn warnings_are_sorted_by_line() {
        let w = lint_src("ni result = °I;\nni foobar = °II;\nni widget = °III;");

        let lines: Vec<Option<usize>> = w.iter().map(|x| x.line).collect();
        assert_eq!(lines.len(), 3, "expected three warnings to order, got {w:?}");
        assert_eq!(lines, vec![Some(1), Some(2), Some(3)]);
    }

    #[test]
    fn reaches_declarations_inside_else_block() {
        let w = lint_src("si (falsus) { } aut { ni result = °V; }");
        assert_eq!(w.len(), 1, "expected exactly one warning, got {w:?}");
        assert!(w[0].message.contains("result"));
    }

    #[test]
    fn reaches_declarations_in_for_init() {
        let src = "dum (ni foobar = °I; foobar minor_est °X; foobar = foobar + °I) { }";
        let w = lint_src(src);
        assert_eq!(w.len(), 1, "expected exactly one warning, got {w:?}");
        assert!(w[0].message.contains("foobar"));
    }

    #[test]
    fn reaches_declarations_in_for_body() {
        let src = "dum (ni i = °I; i minor_est °III; i = i + °I) { ni widget = °V; }";
        let w = lint_src(src);
        assert_eq!(w.len(), 1, "expected exactly one warning, got {w:?}");
        assert!(w[0].message.contains("widget"));
    }

    use crate::ast::{ParsedFile, ParsedProgram};
    use std::collections::BTreeMap;

    fn pfile(path: &str, src: &str) -> ParsedFile {
        ParsedFile {
            path: path.into(),
            src: src.into(),
            ast: crate::parser::parse_file(&tokenize(src).unwrap()).unwrap(),
        }
    }

    fn prog_with_package(pkg: &str) -> ParsedProgram {
        let mut packages = BTreeMap::new();
        packages.insert(pkg.to_string(), vec![
            pfile(&format!("{pkg}/intus.liss"), "munus quinque reddit ni { redde °V; }"),
        ]);
        ParsedProgram {
            root: vec![pfile("principale.liss",
                "munus initium reddit ni { ni result = °V; scribe(result); redde nullus; }")],
            packages,
        }
    }

    #[test]
    fn non_latin_package_name_warns_without_file_or_line() {
        let ws = lint_program(&prog_with_package("foobar"));
        let pkg_warning = ws.iter().find(|w| w.message.contains("fasciculi"))
            .expect("expected a package-name warning");
        assert!(pkg_warning.message.contains("foobar"));
        assert_eq!(pkg_warning.file, None);
        assert_eq!(pkg_warning.line, None);
    }

    #[test]
    fn latin_package_name_does_not_warn() {
        let ws = lint_program(&prog_with_package("mathematica"));
        assert!(ws.iter().all(|w| !w.message.contains("fasciculi")), "got {ws:?}");
    }

    #[test]
    fn file_warnings_carry_their_path() {
        let ws = lint_program(&prog_with_package("mathematica"));
        let w = ws.iter().find(|w| w.message.contains("result"))
            .expect("expected the 'result' warning");
        assert_eq!(w.file.as_deref(), Some("principale.liss"));
        assert_eq!(w.line, Some(1));
    }
}
