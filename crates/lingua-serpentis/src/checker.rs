use crate::ast::*;
use crate::error::{LinguaError, LResult, ProgramError};
use crate::spatium::Spatium;
use crate::roman::Roman;
use ir as ir;
use std::collections::{HashMap, HashSet};

/// `FloatLit` is the not-yet-resolved type of a fractional literal, and of arithmetic
/// combining only such literals; every other expression is `Known`.
#[derive(Clone, PartialEq)]
enum InferTy {
    Known(Type),
    FloatLit,
}

type Sig = (Vec<Type>, Type); // (param types, return type)

/// A function's identity for numbering: its namespace (`None` for the root), and name.
type FuncKey = (Option<String>, String);

/// `mnr` and `nr` collapse into one `Frac` variant carrying its grain; only the
/// checker's diagnostics need the two names apart (spec §4.1).
fn ty_of(t: &Type) -> ir::Ty {
    match t {
        Type::Int => ir::Ty::Int,
        Type::Float => ir::Ty::Frac(ir::Grain::Uncia),
        Type::Double => ir::Ty::Frac(ir::Grain::Scripulum),
        Type::Text => ir::Ty::Text,
        Type::Char => ir::Ty::Char,
        Type::Bool => ir::Ty::Bool,
        Type::List(inner) => ir::Ty::List(Box::new(ty_of(inner))),
    }
}

/// An unresolved fractional literal takes `nr`'s grain, its default type.
fn infer_ir_ty(t: &InferTy) -> ir::Ty {
    match t {
        InferTy::Known(ty) => ty_of(ty),
        InferTy::FloatLit => ir::Ty::Frac(ir::Grain::Scripulum),
    }
}

/// The grain a float type counts in, or `None` for anything that is not one.
fn grain_of(t: &Type) -> Option<ir::Grain> {
    match t {
        Type::Float => Some(ir::Grain::Uncia),
        Type::Double => Some(ir::Grain::Scripulum),
        _ => None,
    }
}

/// The number of a grain's units in one `as`: XII for `mnr`, CCLXXXVIII for `nr`.
fn grain_denom(g: ir::Grain) -> Roman {
    match g {
        ir::Grain::Uncia => crate::fraction::uncia_grain(),
        ir::Grain::Scripulum => crate::fraction::scripulum_grain(),
    }
}

fn konst(ty: ir::Ty, c: ir::Const) -> ir::Expr {
    ir::Expr { ty, kind: ir::ExprKind::Const(c) }
}

struct Checker {
    scopes: Vec<HashMap<String, Type>>,
    // Parallel to `scopes`. Slots are per function and never reused, so leaving a
    // block pops the NAME binding and keeps the number (spec §4.2).
    slots: Vec<HashMap<String, ir::SlotId>>,
    // The current function's locals, indexed by `SlotId`; parameters are `0..params`.
    slot_types: Vec<ir::Ty>,
    // The slots each open scope introduced, in declaration order; on pop this becomes
    // that block's `Block::declares`, the ownership machinery of spec §7.3.
    scope_slots: Vec<Vec<ir::SlotId>>,
    funcs: HashMap<String, Sig>,                       // current namespace (folder)
    packages: HashMap<String, HashMap<String, Sig>>,   // program-wide package signatures
    imports: HashSet<String>,                          // current file's affer set
    // Every declaration in the program, numbered.
    func_ids: HashMap<FuncKey, ir::FuncId>,
    // The file whose bodies are being checked, for `Func::file`.
    file: Option<String>,
    /// The return type of the function whose body is being checked. Not an `Option`:
    /// with `checker::check` gone (D-19) the "no enclosing function" state cannot be
    /// constructed, so the type rules it out instead of an `unreachable!`.
    current_ret: Type,
    loop_depth: usize,         // how many `indem`/`dum` bodies enclose the current statement
    // `None` is the program's root. It decides the mangled symbol (§4.3), and whether
    // an `initium` here is the entry point.
    namespace: Option<String>,
}

/// Does every path through this statement leave the function? Deliberately
/// conservative: `true` only where a return is provable, because D-01.
fn always_returns(s: &Stmt) -> bool {
    match &s.kind {
        StmtKind::Return(_) => true,
        StmtKind::If { then, else_: Some(eb), .. } => block_returns(then) && block_returns(eb),
        // A loop may run zero times; `indem (verus)` is deliberately not special-cased.
        _ => false,
    }
}

/// `any`, not "the last statement": a `redde` followed by unreachable code still
/// returns, and this language has no dead-code lint to disagree with.
fn block_returns(body: &[Stmt]) -> bool {
    body.iter().any(always_returns)
}

/// A statement that evaluates nothing and owns nothing. Unreachable, since
/// `assign_no_semi` makes a `dum` step an `Assign`; here because `For` needs a `Stmt`.
fn nop(line: usize) -> ir::Stmt {
    ir::Stmt {
        line,
        kind: ir::StmtKind::ExprStmt(konst(ir::Ty::Bool, ir::Const::Bool(true))),
    }
}

impl Checker {
    /// `namespace` is a parameter and not a default: a default that quietly said
    /// "root" would hand the entry point's exemption to whoever forgot to set it.
    fn new(namespace: Option<String>) -> Self {
        Checker {
            scopes: vec![HashMap::new()],
            slots: vec![HashMap::new()],
            slot_types: Vec::new(),
            scope_slots: vec![Vec::new()],
            funcs: HashMap::new(),
            packages: HashMap::new(),
            imports: HashSet::new(),
            func_ids: HashMap::new(),
            file: None,
            current_ret: Type::Int,
            loop_depth: 0,
            namespace,
        }
    }

    /// Only the root's `initium` is the entry point, and only it may fall off its end.
    fn root_namespace(&self) -> bool {
        self.namespace.is_none()
    }

    /// Names, slot bindings and the declaration list, always pushed the three together.
    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
        self.slots.push(HashMap::new());
        self.scope_slots.push(Vec::new());
    }

    /// Yields the slots it introduced, in declaration order; the numbers are not
    /// returned to a pool, which is what makes an inner `ni x` a different variable.
    fn pop_scope(&mut self) -> Vec<ir::SlotId> {
        self.scopes.pop();
        self.slots.pop();
        self.scope_slots.pop().unwrap_or_default()
    }

    /// Check a loop body with `abrumpe`/`continua` enabled inside it.
    fn check_loop_body(&mut self, body: &mut [Stmt]) -> LResult<ir::Block> {
        self.loop_depth += 1;
        let r = self.check_block(body);
        self.loop_depth -= 1;
        r
    }

    /// The innermost binding of `name`. The two stacks are read in lockstep, so a
    /// frame can only answer for a name it has in both; `declare` writes both.
    fn lookup(&self, name: &str) -> Option<(Type, ir::SlotId)> {
        self.scopes.iter().zip(self.slots.iter()).rev().find_map(|(tys, sls)| {
            match (tys.get(name), sls.get(name)) {
                (Some(t), Some(s)) => Some((t.clone(), *s)),
                _ => None,
            }
        })
    }
    /// `lookup`, raising the out-of-scope diagnostic. One site, not three (D-24).
    fn lookup_or_err(&self, name: &str, spatium: Spatium) -> LResult<(Type, ir::SlotId)> {
        let line = spatium.linea();
        self.lookup(name).ok_or_else(|| LinguaError {
            message: format!("variabilis ignota: '{}'", name),
            line, spatium: Some(spatium),
        })
    }
    /// Bind `name` to a freshly allocated slot of the current function.
    fn declare(&mut self, name: String, ty: Type) -> ir::SlotId {
        let slot = ir::SlotId(self.slot_types.len() as u32);
        self.slot_types.push(ty_of(&ty));
        self.scopes.last_mut().unwrap().insert(name.clone(), ty);
        self.slots.last_mut().unwrap().insert(name, slot);
        if let Some(open) = self.scope_slots.last_mut() {
            open.push(slot);
        }
        slot
    }

    /// Total by construction: `collect_sigs` and `assign_func_ids` see exactly the
    /// same declarations, in the same walk.
    fn func_id(&self, ns: Option<&str>, name: &str) -> ir::FuncId {
        self.func_ids
            .get(&(ns.map(str::to_string), name.to_string()))
            .copied()
            .expect("every resolved call target was numbered by assign_func_ids")
    }

    fn check_args(&self, fname: &str, params: &[Type], args: &mut [Expr], spatium: Spatium)
        -> LResult<Vec<ir::Expr>>
    {
        let line = spatium.linea();
        if params.len() != args.len() {
            return Err(LinguaError {
                message: format!("functio '{}' numerum argumentorum falsum accepit", fname),
                line, spatium: Some(spatium),
            });
        }
        let mut lowered = Vec::with_capacity(args.len());
        for (want, arg) in params.iter().zip(args.iter_mut()) {
            let (got, node) = self.resolve_expr(arg, Some(want.clone()))?;
            if got != *want {
                return Err(LinguaError {
                    message: format!("argumentum typum '{}' expectat, sed '{}' datum est",
                        Self::type_name(want), Self::type_name(&got)),
                    line, spatium: Some(spatium),
                });
            }
            lowered.push(node);
        }
        Ok(lowered)
    }

    fn type_name(t: &Type) -> String {
        match t {
            Type::Int => "ni".into(), Type::Float => "mnr".into(), Type::Double => "nr".into(),
            Type::Text => "voc".into(), Type::Char => "lit".into(), Type::Bool => "veritas".into(),
            Type::List(inner) => format!("{} lista", Self::type_name(inner)),
        }
    }

    /// A bare fractional literal reports as `nr`, its default type.
    fn infer_name(t: &InferTy) -> String {
        match t {
            InferTy::Known(ty) => Self::type_name(ty),
            InferTy::FloatLit => "nr".into(),
        }
    }

    fn check_expr(&self, e: &mut Expr) -> LResult<(InferTy, ir::Expr)> {
        // The expression's own span, not the statement's: this is what puts a
        // caret under the subexpression a diagnostic is actually about.
        let spatium = e.spatium;
        let line = spatium.linea();
        match &mut e.kind {
            ExprKind::Int(r) => Ok((InferTy::Known(Type::Int),
                konst(ir::Ty::Int, ir::Const::Int(r.clone())))),
            ExprKind::FloatLit { numer, den } => {
                // Still abstract, so the IR takes `nr`'s grain; `resolve_pair`
                // re-narrows onto `mnr`'s grid exactly, because truncation composes.
                let count = crate::fraction::from_rational(
                    numer, den, &crate::fraction::scripulum_grain());
                Ok((InferTy::FloatLit, konst(
                    ir::Ty::Frac(ir::Grain::Scripulum),
                    ir::Const::Frac(ir::Grain::Scripulum, count))))
            }
            ExprKind::Float(c) => Ok((InferTy::Known(Type::Float), konst(
                ir::Ty::Frac(ir::Grain::Uncia),
                ir::Const::Frac(ir::Grain::Uncia, c.clone())))),
            ExprKind::Double(c) => Ok((InferTy::Known(Type::Double), konst(
                ir::Ty::Frac(ir::Grain::Scripulum),
                ir::Const::Frac(ir::Grain::Scripulum, c.clone())))),
            ExprKind::Text(s) => Ok((InferTy::Known(Type::Text),
                konst(ir::Ty::Text, ir::Const::Text(s.clone())))),
            ExprKind::Char(c) => Ok((InferTy::Known(Type::Char),
                konst(ir::Ty::Char, ir::Const::Char(*c)))),
            ExprKind::Bool(b) => Ok((InferTy::Known(Type::Bool),
                konst(ir::Ty::Bool, ir::Const::Bool(*b)))),
            ExprKind::Ident(name) => {
                let (t, slot) = self.lookup_or_err(name, spatium)?;
                let ty = ty_of(&t);
                Ok((InferTy::Known(t), ir::Expr { ty, kind: ir::ExprKind::Load(slot) }))
            }
            ExprKind::Unary { op, expr } => {
                let (t, arg) = self.check_expr(expr)?;
                let res = match op {
                    UnOp::Not if t == InferTy::Known(Type::Bool) => InferTy::Known(Type::Bool),
                    UnOp::Neg => match t {
                        InferTy::FloatLit => InferTy::FloatLit,
                        InferTy::Known(Type::Int) => InferTy::Known(Type::Int),
                        InferTy::Known(Type::Float) => InferTy::Known(Type::Float),
                        InferTy::Known(Type::Double) => InferTy::Known(Type::Double),
                        _ => return Err(LinguaError { message: "negatio numerum requirit".into(), line, spatium: Some(spatium), }),
                    },
                    _ => return Err(LinguaError { message: "'non' veritatem requirit".into(), line, spatium: Some(spatium), }),
                };
                let ty = infer_ir_ty(&res);
                Ok((res, ir::Expr { ty, kind: ir::ExprKind::Unary { op: *op, arg: Box::new(arg) } }))
            }
            ExprKind::Binary { op, left, right } => {
                let (lt, lir) = self.check_expr(left)?;
                let (rt, rir) = self.check_expr(right)?;
                self.check_binary(*op, lt, rt, left, right, lir, rir, spatium)
            }
            ExprKind::Call { package: Some(p), name, args } => {
                if !self.imports.contains(p) {
                    return Err(LinguaError {
                        message: format!("fasciculus '{}' non allatus est", p),
                        line, spatium: Some(spatium),
                    });
                }
                // Dead by construction, kept rather than `.expect`ed because an
                // unreachable panic is still a panic; it reads UNCOVERED (D-24).
                let sigs = self.packages.get(p).ok_or_else(|| LinguaError {
                    message: format!("fasciculus '{}' inter allatos non inventus", p),
                    line, spatium: Some(spatium),
                })?;
                let (params, ret) = sigs.get(name).cloned().ok_or_else(|| LinguaError {
                    message: format!("functio '{}' in fasciculo '{}' non inventa", name, p),
                    line, spatium: Some(spatium),
                })?;
                let lowered = self.check_args(name, &params, args, spatium)?;
                let func = self.func_id(Some(p), name);
                let ty = ty_of(&ret);
                Ok((InferTy::Known(ret),
                    ir::Expr { ty, kind: ir::ExprKind::Call { func, args: lowered } }))
            }
            ExprKind::Call { package: None, name, args } => {
                if name.as_str() == "longitudo" {
                    if args.len() != 1 {
                        return Err(LinguaError { message: "'longitudo' unum argumentum accipit".into(), line, spatium: Some(spatium), });
                    }
                    let (t, arg) = self.resolve_expr(&mut args[0], None)?;
                    return match t {
                        Type::List(_) => Ok((InferTy::Known(Type::Int),
                            ir::Expr { ty: ir::Ty::Int, kind: ir::ExprKind::Len(Box::new(arg)) })),
                        other => Err(LinguaError {
                            message: format!("'longitudo' seriem expectat, non '{}'", Self::type_name(&other)),
                            line, spatium: Some(spatium),
                        }),
                    };
                }
                if name.as_str() == "adde" {
                    if args.len() != 2 {
                        return Err(LinguaError { message: "'adde' duo argumenta accipit".into(), line, spatium: Some(spatium), });
                    }
                    // A bare empty list literal takes its element type from the value.
                    let empty_list_arg = matches!(&args[0].kind, ExprKind::ListLit(v) if v.is_empty());
                    if empty_list_arg {
                        let (vt, item) = self.resolve_expr(&mut args[1], None)?;
                        let (_, list) = self.resolve_expr(
                            &mut args[0], Some(Type::List(Box::new(vt.clone()))))?;
                        let ty = ir::Ty::List(Box::new(ty_of(&vt)));
                        return Ok((InferTy::Known(Type::List(Box::new(vt))), ir::Expr { ty,
                            kind: ir::ExprKind::Append { list: Box::new(list), item: Box::new(item) } }));
                    }
                    let (lt, list) = self.resolve_expr(&mut args[0], None)?;
                    let elem = match lt {
                        Type::List(inner) => *inner,
                        other => return Err(LinguaError {
                            message: format!("'adde' seriem expectat, non '{}'", Self::type_name(&other)),
                            line, spatium: Some(spatium),
                        }),
                    };
                    let (vt, item) = self.resolve_expr(&mut args[1], Some(elem.clone()))?;
                    if vt != elem {
                        return Err(LinguaError {
                            message: format!("'adde': series '{}' elementum '{}' non accipit",
                                Self::type_name(&elem), Self::type_name(&vt)),
                            line, spatium: Some(spatium),
                        });
                    }
                    let ty = ir::Ty::List(Box::new(ty_of(&elem)));
                    return Ok((InferTy::Known(Type::List(Box::new(elem))), ir::Expr { ty,
                        kind: ir::ExprKind::Append { list: Box::new(list), item: Box::new(item) } }));
                }
                let (params, ret) = self.funcs.get(name)
                    .cloned()
                    .ok_or_else(|| LinguaError { message: format!("functio ignota: '{}'", name), line, spatium: Some(spatium), })?;
                let lowered = self.check_args(name, &params, args, spatium)?;
                // A bare call resolves in the checker's own namespace, never the root's;
                // codegen/tests/vocationes.rs is the only test that would notice.
                let func = self.func_id(self.namespace.as_deref(), name);
                let ty = ty_of(&ret);
                Ok((InferTy::Known(ret),
                    ir::Expr { ty, kind: ir::ExprKind::Call { func, args: lowered } }))
            }
            ExprKind::ListLit(elems) => {
                let (elem, items) = self.check_list_elems(elems, None, spatium)?;
                let ty = ir::Ty::List(Box::new(ty_of(&elem)));
                Ok((InferTy::Known(Type::List(Box::new(elem))),
                    ir::Expr { ty, kind: ir::ExprKind::ListLit { items } }))
            }
            ExprKind::Index { base, index } => {
                let (base_ty, base_ir) = self.resolve_expr(base, None)?;
                let elem = match base_ty {
                    Type::List(inner) => *inner,
                    other => return Err(LinguaError {
                        message: format!("res typi '{}' subscribi non potest (non est series)",
                            Self::type_name(&other)),
                        line, spatium: Some(spatium),
                    }),
                };
                let (it, index_ir) = self.resolve_expr(index, Some(Type::Int))?;
                if it != Type::Int {
                    return Err(LinguaError {
                        message: "index seriei 'ni' esse debet".into(), line, spatium: Some(spatium), });
                }
                let ty = ty_of(&elem);
                Ok((InferTy::Known(elem), ir::Expr { ty, kind: ir::ExprKind::Index {
                    base: Box::new(base_ir), index: Box::new(index_ir) } }))
            }
        }
    }

    /// `expected` fixes the element type; `None` infers it from the first element.
    /// Returns the ELEMENT type, which callers wrap in `Type::List`.
    fn check_list_elems(&self, elems: &mut [Expr], expected: Option<Type>, spatium: Spatium)
        -> LResult<(Type, Vec<ir::Expr>)>
    {
        let line = spatium.linea();
        if elems.is_empty() {
            return match expected {
                Some(t) => Ok((t, Vec::new())),
                None => Err(LinguaError {
                    message: "typus elementorum seriei vacuae ignotus".into(), line, spatium: Some(spatium), }),
            };
        }
        let (first_el, rest) = elems.split_first_mut().unwrap();
        let (first, first_ir) = self.resolve_expr(first_el, expected.clone())?;
        let mut items = vec![first_ir];
        for el in rest {
            let want = expected.clone().or_else(|| Some(first.clone()));
            let (t, node) = self.resolve_expr(el, want)?;
            if t != first {
                return Err(LinguaError {
                    message: format!("elementa seriei discrepant: '{}' et '{}'",
                        Self::type_name(&first), Self::type_name(&t)),
                    line, spatium: Some(spatium),
                });
            }
            items.push(node);
        }
        Ok((first, items))
    }

    /// Type a list literal against an optional expected list type.
    fn resolve_list(&self, e: &mut Expr, expected: Option<Type>)
        -> LResult<(Type, ir::Expr)>
    {
        let spatium = e.spatium;
        let elem_expected = match &expected {
            Some(Type::List(inner)) => Some((**inner).clone()),
            _ => None,
        };
        let elems = match &mut e.kind {
            ExprKind::ListLit(v) => v,
            _ => unreachable!("resolve_list called on non-list"),
        };
        let (elem, items) = self.check_list_elems(elems, elem_expected, spatium)?;
        let ty = ir::Ty::List(Box::new(ty_of(&elem)));
        Ok((Type::List(Box::new(elem)), ir::Expr { ty, kind: ir::ExprKind::ListLit { items } }))
    }

    /// A `FloatLit` becomes the expected float type; with no float context it defaults
    /// to `nr`; demanded as a non-float type it is an error.
    fn resolve_expr(&self, e: &mut Expr, expected: Option<Type>)
        -> LResult<(Type, ir::Expr)>
    {
        let spatium = e.spatium;
        let line = spatium.linea();
        if matches!(e.kind, ExprKind::ListLit(_)) {
            return self.resolve_list(e, expected);
        }
        match self.check_expr(e)? {
            (InferTy::Known(t), node) => Ok((t, node)),
            (InferTy::FloatLit, mut node) => {
                let ty = match expected {
                    Some(Type::Float) => Type::Float,
                    Some(Type::Double) => Type::Double,
                    Some(other) => return Err(LinguaError {
                        message: format!("numerus fractus typum 'mnr' vel 'nr' requirit, non '{}'",
                            Self::type_name(&other)),
                        line, spatium: Some(spatium),
                    }),
                    None => Type::Double,
                };
                resolve_pair(e, &mut node, ty.clone());
                Ok((ty, node))
            }
        }
    }

    // Each operand arrives as three: synthesized type, AST node, IR node.
    #[allow(clippy::too_many_arguments)]
    fn check_binary(&self, op: BinOp, lt: InferTy, rt: InferTy,
                    left: &mut Expr, right: &mut Expr,
                    mut lir: ir::Expr, mut rir: ir::Expr, spatium: Spatium)
        -> LResult<(InferTy, ir::Expr)>
    {
        let line = spatium.linea();
        use InferTy::*;
        let mismatch = || LinguaError {
            message: format!("operatio typis '{}' et '{}' non convenit",
                Self::infer_name(&lt), Self::infer_name(&rt)),
            line, spatium: Some(spatium),
        };
        let res = match op {
            BinOp::Add => match (&lt, &rt) {
                (FloatLit, FloatLit) => Ok(FloatLit),
                (FloatLit, Known(Type::Float)) => { resolve_pair(left, &mut lir, Type::Float); Ok(Known(Type::Float)) }
                (FloatLit, Known(Type::Double)) => { resolve_pair(left, &mut lir, Type::Double); Ok(Known(Type::Double)) }
                (Known(Type::Float), FloatLit) => { resolve_pair(right, &mut rir, Type::Float); Ok(Known(Type::Float)) }
                (Known(Type::Double), FloatLit) => { resolve_pair(right, &mut rir, Type::Double); Ok(Known(Type::Double)) }
                (Known(Type::Int), Known(Type::Int)) => Ok(Known(Type::Int)),
                (Known(Type::Float), Known(Type::Float)) => Ok(Known(Type::Float)),
                (Known(Type::Double), Known(Type::Double)) => Ok(Known(Type::Double)),
                (Known(Type::Text), Known(Type::Text)) => Ok(Known(Type::Text)),
                _ => Err(mismatch()),
            },
            BinOp::Sub | BinOp::Mul | BinOp::Div => match (&lt, &rt) {
                (FloatLit, FloatLit) => Ok(FloatLit),
                (FloatLit, Known(Type::Float)) => { resolve_pair(left, &mut lir, Type::Float); Ok(Known(Type::Float)) }
                (FloatLit, Known(Type::Double)) => { resolve_pair(left, &mut lir, Type::Double); Ok(Known(Type::Double)) }
                (Known(Type::Float), FloatLit) => { resolve_pair(right, &mut rir, Type::Float); Ok(Known(Type::Float)) }
                (Known(Type::Double), FloatLit) => { resolve_pair(right, &mut rir, Type::Double); Ok(Known(Type::Double)) }
                (Known(Type::Int), Known(Type::Int)) => Ok(Known(Type::Int)),
                (Known(Type::Float), Known(Type::Float)) => Ok(Known(Type::Float)),
                (Known(Type::Double), Known(Type::Double)) => Ok(Known(Type::Double)),
                _ => Err(mismatch()),
            },
            BinOp::Mod => match (&lt, &rt) {
                (Known(Type::Int), Known(Type::Int)) => Ok(Known(Type::Int)),
                _ => Err(mismatch()),
            },
            BinOp::Eq | BinOp::Ne => match (&lt, &rt) {
                (FloatLit, FloatLit) => {
                    resolve_pair(left, &mut lir, Type::Double);
                    resolve_pair(right, &mut rir, Type::Double);
                    Ok(Known(Type::Bool))
                }
                (FloatLit, Known(Type::Float)) => { resolve_pair(left, &mut lir, Type::Float); Ok(Known(Type::Bool)) }
                (FloatLit, Known(Type::Double)) => { resolve_pair(left, &mut lir, Type::Double); Ok(Known(Type::Bool)) }
                (Known(Type::Float), FloatLit) => { resolve_pair(right, &mut rir, Type::Float); Ok(Known(Type::Bool)) }
                (Known(Type::Double), FloatLit) => { resolve_pair(right, &mut rir, Type::Double); Ok(Known(Type::Bool)) }
                (Known(a), Known(b)) if a == b => Ok(Known(Type::Bool)),
                _ => Err(mismatch()),
            },
            BinOp::Gt | BinOp::Lt => match (&lt, &rt) {
                (FloatLit, FloatLit) => {
                    resolve_pair(left, &mut lir, Type::Double);
                    resolve_pair(right, &mut rir, Type::Double);
                    Ok(Known(Type::Bool))
                }
                (FloatLit, Known(Type::Float)) => { resolve_pair(left, &mut lir, Type::Float); Ok(Known(Type::Bool)) }
                (FloatLit, Known(Type::Double)) => { resolve_pair(left, &mut lir, Type::Double); Ok(Known(Type::Bool)) }
                (Known(Type::Float), FloatLit) => { resolve_pair(right, &mut rir, Type::Float); Ok(Known(Type::Bool)) }
                (Known(Type::Double), FloatLit) => { resolve_pair(right, &mut rir, Type::Double); Ok(Known(Type::Bool)) }
                (Known(Type::List(_)), Known(Type::List(_))) => Err(LinguaError {
                    message: "series ordine comparari non potest (par_est/par_non_est tantum)".into(),
                    line, spatium: Some(spatium),
                }),
                (Known(a), Known(b)) if a == b => Ok(Known(Type::Bool)),
                _ => Err(mismatch()),
            },
            BinOp::And | BinOp::Or => match (&lt, &rt) {
                // `et` and `vel` do NOT short-circuit and must not become a branch
                // (spec §3.5). Pinned by conformance/errores/{vel,et}-utrumque-latus.
                (Known(Type::Bool), Known(Type::Bool)) => Ok(Known(Type::Bool)),
                _ => Err(mismatch()),
            },
        }?;
        // Carried separately from the node's own type, because a comparison yields
        // `veritas` from operands that are not (§4.1, property 2).
        let operand_ty = lir.ty.clone();
        let ty = infer_ir_ty(&res);
        Ok((res, ir::Expr { ty, kind: ir::ExprKind::Binary {
            op, operand_ty, lhs: Box::new(lir), rhs: Box::new(rir) } }))
    }

    /// Check one statement, lowering it. `None` is a statement that lowers to nothing,
    /// which only a `munus` declaration does.
    fn check_stmt(&mut self, s: &mut Stmt) -> LResult<Option<ir::Stmt>> {
        let line = s.line;
        let spatium = s.spatium;
        match &mut s.kind {
            StmtKind::Let { ty, name, value, .. } => {
                let (vt, value_ir) = self.resolve_expr(value, Some(ty.clone()))?;
                if vt != *ty {
                    return Err(LinguaError {
                        message: format!("'{}' typum '{}' expectat, sed '{}' datum est",
                            name, Self::type_name(ty), Self::type_name(&vt)),
                        line, spatium: Some(spatium),
                    });
                }
                let slot = self.declare(name.clone(), ty.clone());
                Ok(Some(ir::Stmt { line, kind: ir::StmtKind::Let { slot, value: value_ir } }))
            }
            StmtKind::Assign { name, value } => {
                let (target, slot) = self.lookup_or_err(name, spatium)?;
                let (vt, value_ir) = self.resolve_expr(value, Some(target.clone()))?;
                // `iam` distinguishes assigning from declaring; `Let` above keeps the
                // original wording (D-24). This note stays outside the constructor.
                if vt != target {
                    return Err(LinguaError {
                        message: format!("'{}' iam typum '{}' habet, sed '{}' datum est",
                            name, Self::type_name(&target), Self::type_name(&vt)),
                        line, spatium: Some(spatium),
                    });
                }
                Ok(Some(ir::Stmt { line, kind: ir::StmtKind::Assign { slot, value: value_ir } }))
            }
            StmtKind::Print(e) => {
                let (_, node) = self.resolve_expr(e, None)?;
                Ok(Some(ir::Stmt { line, kind: ir::StmtKind::Print(node) }))
            }
            StmtKind::ExprStmt(e) => {
                let (_, node) = self.resolve_expr(e, None)?;
                Ok(Some(ir::Stmt { line, kind: ir::StmtKind::ExprStmt(node) }))
            }
            StmtKind::If { cond, then, else_ } => {
                let cond_ir = self.expect_bool(cond)?;
                let then_ir = self.check_block(then)?;
                let else_ir = match else_ {
                    Some(eb) => Some(self.check_block(eb)?),
                    None => None,
                };
                Ok(Some(ir::Stmt { line, kind: ir::StmtKind::If {
                    cond: cond_ir, then: then_ir, else_: else_ir } }))
            }
            StmtKind::While { cond, body } => {
                let cond_ir = self.expect_bool(cond)?;
                let body_ir = self.check_loop_body(body)?;
                Ok(Some(ir::Stmt { line, kind: ir::StmtKind::While { cond: cond_ir, body: body_ir } }))
            }
            StmtKind::For { init, cond, step, body } => {
                self.push_scope();
                let r = (|| {
                    // `init`, `cond` and `step` are the loop's header, not its body:
                    // an `abrumpe` there is as loose as one outside (D-26).
                    let init_ir = self.check_stmt(init)?;
                    let cond_ir = self.expect_bool(cond)?;
                    let step_ir = self.check_stmt(step)?;
                    let body_ir = self.check_loop_body(body)?;
                    Ok((init_ir, cond_ir, step_ir, body_ir))
                })();
                // The header is a scope, so it is a `Block`; one outside the `Block`
                // tree would leak, since §7.3.4 frees every enclosing block.
                let declares = self.pop_scope();
                let (init_ir, cond_ir, step_ir, body_ir) = r?;
                Ok(Some(ir::Stmt { line, kind: ir::StmtKind::For {
                    header: ir::Block { stmts: init_ir.into_iter().collect(), declares },
                    cond: cond_ir,
                    step: Box::new(step_ir.unwrap_or_else(|| nop(line))),
                    body: body_ir,
                } }))
            }
            StmtKind::Func(f) => {
                // Reaching here means a `munus` in a `dum` header, which the parser
                // accepts and nothing can call: checked, then dropped.
                self.check_func_decl(f, spatium)?;
                Ok(None)
            }
            StmtKind::Return(e) => {
                let want = self.current_ret.clone();
                let (got, node) = self.resolve_expr(e, Some(want.clone()))?;
                if got != want {
                    return Err(LinguaError {
                        message: format!("reditus typum '{}' expectat, sed '{}' datum est",
                            Self::type_name(&want), Self::type_name(&got)),
                        line, spatium: Some(spatium),
                    });
                }
                Ok(Some(ir::Stmt { line, kind: ir::StmtKind::Return(node) }))
            }
            StmtKind::IndexAssign { name, index, value } => {
                let (target, slot) = self.lookup_or_err(name, spatium)?;
                let elem = match target {
                    Type::List(inner) => *inner,
                    other => return Err(LinguaError {
                        message: format!("'{}' non est series (typus '{}')", name, Self::type_name(&other)),
                        line, spatium: Some(spatium),
                    }),
                };
                let (it, index_ir) = self.resolve_expr(index, Some(Type::Int))?;
                if it != Type::Int {
                    // The assigning form; conformance/errores/index-assignationis-non-ni.
                    return Err(LinguaError {
                        message: "index assignationis 'ni' esse debet".into(),
                        line, spatium: Some(spatium),
                    });
                }
                let (vt, value_ir) = self.resolve_expr(value, Some(elem.clone()))?;
                if vt != elem {
                    return Err(LinguaError {
                        message: format!("elementum typum '{}' expectat, sed '{}' datum est",
                            Self::type_name(&elem), Self::type_name(&vt)),
                        line, spatium: Some(spatium),
                    });
                }
                Ok(Some(ir::Stmt { line, kind: ir::StmtKind::IndexAssign {
                    slot, index: index_ir, value: value_ir } }))
            }
            // D-07
            StmtKind::Break if self.loop_depth == 0 => {
                Err(LinguaError { message: "'abrumpe' extra iterationem".into(), line, spatium: Some(spatium), })
            }
            StmtKind::Continue if self.loop_depth == 0 => {
                Err(LinguaError { message: "'continua' extra iterationem".into(), line, spatium: Some(spatium), })
            }
            StmtKind::Break => Ok(Some(ir::Stmt { line, kind: ir::StmtKind::Break })),
            StmtKind::Continue => Ok(Some(ir::Stmt { line, kind: ir::StmtKind::Continue })),
        }
    }

    /// The scope it pushes is the parameter frame, so the body block's `declares`
    /// opens with slots `0..params` and every `redde` frees them too (§7.3.4).
    fn check_func_decl(&mut self, f: &mut FuncDecl, spatium: Spatium) -> LResult<ir::Func> {
        let line = spatium.linea();
        self.push_scope();
        // Slots are numbered per function, so a fresh table; restored below.
        let outer_slots = std::mem::take(&mut self.slot_types);
        for (t, n, _) in &f.params { self.declare(n.clone(), t.clone()); }
        let saved = std::mem::replace(&mut self.current_ret, f.ret.clone());
        // A function body is not inside any loop, whatever encloses the declaration.
        let saved_depth = std::mem::take(&mut self.loop_depth);
        let r = self.check_block_inner(&mut f.body);
        self.loop_depth = saved_depth;
        self.current_ret = saved;
        let declares = self.pop_scope();
        let slots = std::mem::replace(&mut self.slot_types, outer_slots);
        // After the body, so a genuine error inside it wins.
        let mut stmts = r?;
        // D-01. Only the ROOT `initium` is exempt (§4.3), pinned end to end by
        // conformance/currunt/initium-sine-reditu.
        let is_entry = self.root_namespace() && f.name == "initium";
        let returns = block_returns(&f.body);
        if !is_entry && !returns {
            return Err(LinguaError {
                message: format!("munus '{}' non in omni via reddit", f.name),
                line, spatium: Some(spatium),
            });
        }
        if !returns {
            // The root `initium` falling off its end; lowering supplies the exit
            // status (§4.4). Every other function reaching here provably returns.
            stmts.push(ir::Stmt { line, kind: ir::StmtKind::Return(
                konst(ir::Ty::Int, ir::Const::Int(Roman::zero()))) });
        }
        Ok(ir::Func {
            symbol: ir::symbol(self.namespace.as_deref(), &f.name),
            file: self.file.clone(),
            is_entry,
            params: f.params.len() as u32,
            slots,
            ret: ty_of(&f.ret),
            body: ir::Block { stmts, declares },
        })
    }

    /// A block that opens a scope of its own; the pop happens on every path, and
    /// there is one nested-function guard rather than two identical ones (D-24).
    fn check_block(&mut self, body: &mut [Stmt]) -> LResult<ir::Block> {
        self.push_scope();
        let r = self.check_block_inner(body);
        let declares = self.pop_scope();
        Ok(ir::Block { stmts: r?, declares })
    }

    /// A block that runs in a scope the caller already pushed.
    fn check_block_inner(&mut self, body: &mut [Stmt]) -> LResult<Vec<ir::Stmt>> {
        let mut stmts = Vec::new();
        for s in body.iter_mut() {
            if let StmtKind::Func(f) = &s.kind {
                return Err(LinguaError {
                    message: format!("functio '{}' intra corpus definiri non potest; functiones nidificatae non sustinentur", f.name),
                    line: s.line, spatium: Some(s.spatium),
                });
            }
            stmts.extend(self.check_stmt(s)?);
        }
        Ok(stmts)
    }

    fn expect_bool(&self, cond: &mut Expr) -> LResult<ir::Expr> {
        let spatium = cond.spatium;
        let line = spatium.linea();
        let (t, node) = self.check_expr(cond)?;
        if t == InferTy::Known(Type::Bool) {
            Ok(node)
        } else {
            Err(LinguaError { message: "condicio 'veritas' esse debet".into(), line, spatium: Some(spatium), })
        }
    }
}

/// Narrow every still-abstract fractional literal onto the type's grain grid by
/// truncation. Only ever called where the synthesized type was `FloatLit`.
fn resolve_floatlit(e: &mut Expr, ty: Type) {
    match &mut e.kind {
        ExprKind::FloatLit { numer, den } => {
            let (numer, den) = (numer.clone(), den.clone());
            // The literal narrows in place; its span is unchanged, because it is
            // still written where it was written.
            e.kind = match ty {
                Type::Float => ExprKind::Float(
                    crate::fraction::from_rational(&numer, &den, &crate::fraction::uncia_grain())),
                Type::Double => ExprKind::Double(
                    crate::fraction::from_rational(&numer, &den, &crate::fraction::scripulum_grain())),
                _ => return,
            };
        }
        ExprKind::Unary { expr, .. } => resolve_floatlit(expr, ty),
        ExprKind::Binary { left, right, .. } => {
            resolve_floatlit(left, ty.clone());
            resolve_floatlit(right, ty);
        }
        _ => {}
    }
}

/// `resolve_floatlit`'s mirror over the typed IR, deliberately adjacent to it: the
/// same three arms in the same order, so the two cannot drift apart.
fn resolve_floatlit_ir(e: &mut ir::Expr, g: ir::Grain) {
    let narrowed = match &mut e.kind {
        ir::ExprKind::Const(ir::Const::Frac(had, count)) => {
            // The composition is exact in one direction only, and nothing textual
            // protects it, because `resolve_floatlit` is direction-free.
            debug_assert!(*had == ir::Grain::Scripulum || *had == g,
                "resolve_floatlit_ir is exact only narrowing from nr's grain");
            *count = crate::fraction::from_rational(count, &grain_denom(*had), &grain_denom(g));
            *had = g;
            true
        }
        ir::ExprKind::Unary { arg, .. } => { resolve_floatlit_ir(arg, g); true }
        ir::ExprKind::Binary { operand_ty, lhs, rhs, .. } => {
            resolve_floatlit_ir(lhs, g);
            resolve_floatlit_ir(rhs, g);
            *operand_ty = ir::Ty::Frac(g);
            true
        }
        _ => false,
    };
    if narrowed {
        e.ty = ir::Ty::Frac(g);
    }
}

/// Narrow one operand in both trees at once; the only caller of either half, so
/// neither rewrite can be written without the other.
fn resolve_pair(ast: &mut Expr, node: &mut ir::Expr, ty: Type) {
    if let Some(g) = grain_of(&ty) {
        resolve_floatlit_ir(node, g);
    }
    resolve_floatlit(ast, ty);
}

/// The names the language keeps for its own builtins. One copy, not two (D-24).
fn reserved_name_error(name: &str, spatium: Spatium) -> Option<LinguaError> {
        let line = spatium.linea();
    matches!(name, "longitudo" | "adde").then(|| LinguaError {
        message: format!("nomen '{}' reservatum est", name),
        line, spatium: Some(spatium),
    })
}

/// One folder's signatures across all its files, rejecting reserved names and
/// duplicates (§4.2); two `initium` in the root land here as the duplicate error.
fn collect_sigs(files: &[ParsedFile]) -> Result<HashMap<String, Sig>, ProgramError> {
    let mut sigs: HashMap<String, Sig> = HashMap::new();
    for file in files {
        for s in &file.ast.funcs {
            let StmtKind::Func(f) = &s.kind else { continue };
            if let Some(e) = reserved_name_error(&f.name, f.name_span) {
                return Err(ProgramError::in_file(&file.path, &file.src, e));
            }
            if sigs.contains_key(&f.name) {
                return Err(ProgramError::in_file(&file.path, &file.src, LinguaError {
                    message: format!("functio '{}' bis definita", f.name),
                    line: s.line, spatium: Some(s.spatium),
                }));
            }
            let ptypes = f.params.iter().map(|(t, _, _)| t.clone()).collect();
            sigs.insert(f.name.clone(), (ptypes, f.ret.clone()));
        }
    }
    Ok(sigs)
}

/// Numbered in the order `check_program_typed` lowers them, so `Programma::funcs[id]`
/// is the function `id` names, which `check_file` asserts as it goes.
fn assign_func_ids(prog: &ParsedProgram) -> HashMap<FuncKey, ir::FuncId> {
    let mut namespaces: Vec<(Option<&str>, &Vec<ParsedFile>)> = vec![(None, &prog.root)];
    for (name, files) in &prog.packages {
        namespaces.push((Some(name.as_str()), files));
    }
    let mut ids = HashMap::new();
    let mut next = 0u32;
    for (ns, files) in namespaces {
        for file in files {
            for s in &file.ast.funcs {
                let StmtKind::Func(f) = &s.kind else { continue };
                ids.insert((ns.map(str::to_string), f.name.clone()), ir::FuncId(next));
                next += 1;
            }
        }
    }
    ids
}

/// Check one file's bodies against its folder's namespace and its own imports.
fn check_file(
    file: &mut ParsedFile,
    ns: Option<&str>,
    ns_sigs: &HashMap<String, Sig>,
    pkg_sigs: &HashMap<String, HashMap<String, Sig>>,
    func_ids: &HashMap<FuncKey, ir::FuncId>,
    out: &mut Vec<ir::Func>,
) -> Result<(), ProgramError> {
    for imp in &file.ast.imports {
        if !pkg_sigs.contains_key(&imp.package) {
            return Err(ProgramError::in_file(&file.path, &file.src, LinguaError {
                message: format!("fasciculus '{}' non inventus", imp.package),
                line: imp.line, spatium: None,
            }));
        }
    }
    let mut c = Checker::new(ns.map(str::to_string));
    c.file = Some(file.path.clone());
    c.funcs = ns_sigs.clone();
    c.packages = pkg_sigs.clone();
    c.imports = file.ast.imports.iter().map(|i| i.package.clone()).collect();
    c.func_ids = func_ids.clone();
    for s in file.ast.funcs.iter_mut() {
        // Only `munus` declarations reach here, and `parse_file` is what guarantees it.
        let StmtKind::Func(f) = &mut s.kind else { continue };
        debug_assert_eq!(
            func_ids.get(&(ns.map(str::to_string), f.name.clone())),
            Some(&ir::FuncId(out.len() as u32)),
            "lowering order must be the order assign_func_ids numbered",
        );
        let lowered = c.check_func_decl(f, s.spatium).map_err(|e| ProgramError::in_file(&file.path, &file.src, e))?;
        out.push(lowered);
    }
    Ok(())
}

/// Check a whole parsed program, discarding the IR it built: the checking-only form,
/// so a checker test need not care about IR it is not asserting.
pub fn check_program(prog: &mut ParsedProgram) -> Result<(), ProgramError> {
    check_program_typed(prog).map(|_| ())
}

/// Per-namespace signatures, the `initium` entry point, import validity, then every
/// file's bodies, emitting the typed IR codegen consumes (§3.4, §4).
pub fn check_program_typed(prog: &mut ParsedProgram) -> Result<ir::Programma, ProgramError> {
    let root_sigs = collect_sigs(&prog.root)?;
    let mut pkg_sigs: HashMap<String, HashMap<String, Sig>> = HashMap::new();
    for (name, files) in &prog.packages {
        pkg_sigs.insert(name.clone(), collect_sigs(files)?);
    }

    match root_sigs.get("initium") {
        None => return Err(ProgramError::structural(
            "munus 'initium' in programmate non inventum")),
        Some((params, ret)) => {
            if !params.is_empty() || *ret != Type::Int {
                for file in &prog.root {
                    for s in &file.ast.funcs {
                        if let StmtKind::Func(f) = &s.kind {
                            if f.name == "initium" {
                                return Err(ProgramError::in_file(&file.path, &file.src, LinguaError {
                                    message: "munus 'initium' nihil capere et 'ni' reddere debet".into(),
                                    line: s.line, spatium: Some(s.spatium),
                                }));
                            }
                        }
                    }
                }
                unreachable!("initium was in root_sigs, so its declaration exists");
            }
        }
    }

    let func_ids = assign_func_ids(prog);
    let mut funcs = Vec::new();
    for file in prog.root.iter_mut() {
        check_file(file, None, &root_sigs, &pkg_sigs, &func_ids, &mut funcs)?;
    }
    for (name, files) in prog.packages.iter_mut() {
        let ns = &pkg_sigs[name];
        for file in files.iter_mut() {
            check_file(file, Some(name), ns, &pkg_sigs, &func_ids, &mut funcs)?;
        }
    }
    Ok(ir::Programma { funcs })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::tokenize;

    /// A statement sequence through a one-line `munus initium` wrapper (D-19), so
    /// asserted line numbers do not move. Sources declaring functions use
    /// [`check_totum`], since the wrapper would nest them.
    fn check_src(src: &str) -> LResult<()> {
        check_totum(&format!("munus initium reddit ni {{ {src} }}"))
    }

    /// A complete single-file program through the real entry point; the error is
    /// narrowed back to a `LinguaError`, and a structural one surfaces at line 1.
    fn check_totum(src: &str) -> LResult<()> {
        let mut p = pprog(&[("fons.liss", src)], &[]);
        check_program(&mut p)
            .map_err(|e| LinguaError { message: e.message, line: e.line.unwrap_or(1), spatium: None, })
    }

    #[test]
    fn well_typed_program_passes() {
        assert!(check_src("ni x = °V; x = x + °I; scribe(x);").is_ok());
    }

    #[test]
    fn a_function_must_return_on_every_path() {
        // D-01
        let p = pprog(
            &[("principale.liss",
               "// pagina prima\nmunus nomen reddit voc {\n  si (falsus) { redde \"a\"; }\n}\n\
                munus initium reddit ni { scribe(nomen()); redde nullus; }")],
            &[],
        );
        let e = check_program(&mut { p }).unwrap_err();
        assert_eq!(e.message, "munus 'nomen' non in omni via reddit");
    }

    #[test]
    fn both_arms_of_an_if_returning_is_enough() {
        let p = pprog(
            &[("principale.liss",
               "munus signum capit (ni n) reddit voc {\n  \
                si (n minor_est nullus) { redde \"negativus\"; } aut { redde \"positivus\"; }\n}\n\
                munus initium reddit ni { scribe(signum(°I)); redde nullus; }")],
            &[],
        );
        assert!(check_program(&mut { p }).is_ok());
    }

    #[test]
    fn only_the_root_initium_may_fall_off_its_end() {
        // §4.3; a package function that happens to be called `initium` is ordinary.
        let root_ok = pprog(&[("principale.liss", "munus initium reddit ni { scribe(°I); }")], &[]);
        assert!(check_program(&mut { root_ok }).is_ok());

        let pkg = pprog(
            &[("principale.liss", "affer mathematica;\nmunus initium reddit ni { redde nullus; }")],
            &[("mathematica", &[("summa.liss", "munus initium reddit ni { scribe(°I); }")] as &[_])],
        );
        let e = check_program(&mut { pkg }).unwrap_err();
        assert_eq!(e.message, "munus 'initium' non in omni via reddit");
    }

    #[test]
    fn a_loop_body_does_not_count_as_returning() {
        let p = pprog(
            &[("principale.liss",
               "munus semper reddit ni {\n  indem (verus) { redde °I; }\n}\n\
                munus initium reddit ni { redde semper(); }")],
            &[],
        );
        let e = check_program(&mut { p }).unwrap_err();
        assert_eq!(e.message, "munus 'semper' non in omni via reddit");
    }

    #[test]
    fn break_outside_a_loop_is_rejected() {
        // D-07
        let e = check_src("scribe(°I); abrumpe; scribe(°II);").unwrap_err();
        assert_eq!(e.message, "'abrumpe' extra iterationem");
    }

    #[test]
    fn continue_outside_a_loop_is_rejected() {
        let e = check_src("scribe(°I); continua; scribe(°II);").unwrap_err();
        assert_eq!(e.message, "'continua' extra iterationem");
    }

    #[test]
    fn break_and_continue_inside_a_loop_are_fine() {
        assert!(check_src("ni i = °I; indem (i minor_est °X) { i = i + °I; abrumpe; }").is_ok());
        assert!(check_src("ni i = °I; indem (i minor_est °X) { i = i + °I; continua; }").is_ok());
        assert!(check_src(
            "ni i = °I; indem (i minor_est °X) { si (i par_est °II) { abrumpe; } i = i + °I; }"
        )
        .is_ok());
    }

    #[test]
    fn a_loop_does_not_license_a_break_after_it() {
        let e = check_src(
            "ni i = °I; indem (i minor_est °X) { i = i + °I; } abrumpe;"
        )
        .unwrap_err();
        assert_eq!(e.message, "'abrumpe' extra iterationem");
    }

    #[test]
    fn assigning_text_to_int_fails() {
        let err = check_src("ni x = \"salve\";").unwrap_err();
        assert!(err.message.contains("typ"));
        assert_eq!(err.line, 1);
    }

    #[test]
    fn comparison_yields_bool() {
        assert!(check_src("veritas b = °V maior_est °III;").is_ok());
    }

    #[test]
    fn adding_int_and_text_fails() {
        assert!(check_src("ni x = °V + \"a\";").is_err());
    }

    #[test]
    fn using_undeclared_variable_fails() {
        assert!(check_src("scribe(y);").is_err());
    }

    #[test]
    fn non_bool_condition_fails() {
        let err = check_src("si (°V) { scribe(°I); }").unwrap_err();
        assert!(err.message.contains("veritas") || err.message.contains("condicio"));
    }

    #[test]
    fn function_return_type_checked() {
        assert!(check_totum(
            "munus iunge capit (ni a, ni b) reddit ni { redde a + b; }\n\
             munus initium reddit ni { scribe(iunge(°I, °II)); }",
        )
        .is_ok());
    }

    #[test]
    fn returning_wrong_type_fails() {
        assert!(check_totum(
            "munus f reddit ni { redde \"salve\"; }\nmunus initium reddit ni { }",
        ).is_err());
    }

    #[test]
    fn calling_with_wrong_arg_type_fails() {
        assert!(check_totum(
            "munus f capit (ni a) reddit ni { redde a; }\n\
             munus initium reddit ni { scribe(f(\"x\")); }",
        ).is_err());
    }

    #[test]
    fn nested_function_rejected() {
        let err = check_totum(
            "munus outer reddit ni { munus inner reddit ni { redde °I; } redde °I; }\n\
             munus initium reddit ni { }",
        )
        .unwrap_err();
        assert!(err.message.contains("nidificat"));
    }

    #[test]
    fn fractional_literal_adapts_to_mnr_and_nr() {
        assert!(check_src("mnr x = °I:I/II;").is_ok());
        assert!(check_src("nr y = °I:I/II;").is_ok());
    }

    #[test]
    fn unconstrained_fraction_defaults_ok() {
        assert!(check_src("scribe(°I:I/II);").is_ok());
    }

    #[test]
    fn fraction_arithmetic_adapts_to_declared_double() {
        assert!(check_src("nr z = °I:I/II + °I:I/IV;").is_ok());
    }

    #[test]
    fn mixing_mnr_and_nr_is_rejected() {
        assert!(check_src("mnr a = °I:I/II; nr b = °I:I/IV; scribe(a + b);").is_err());
        assert!(check_src("mnr a = °I:I/II; nr b = a;").is_err());
        assert!(check_src("mnr a = °I:I/II; nr b = °I:I/IV; scribe(a maior_est b);").is_err());
    }

    #[test]
    fn fractional_literal_into_int_slot_is_rejected() {
        let err = check_src("ni n = °I:I/II;").unwrap_err();
        assert!(err.message.contains("fractus"), "got: {}", err.message);
    }

    #[test]
    fn function_returning_fraction_adapts_to_nr() {
        assert!(check_totum(
            "munus tau reddit nr { redde °I:I/III; }\nmunus initium reddit ni { }",
        )
        .is_ok());
    }

    #[test]
    fn function_accepts_fraction_argument_as_mnr() {
        assert!(check_totum(
            "munus f capit (mnr a) reddit mnr { redde a; }\n\
             munus initium reddit ni { scribe(f(°I:I/II)); }",
        )
        .is_ok());
    }

    #[test]
    fn list_type_names_render_recursively() {
        assert_eq!(Checker::type_name(&Type::List(Box::new(Type::Int))), "ni lista");
        assert_eq!(
            Checker::type_name(&Type::List(Box::new(Type::List(Box::new(Type::Text))))),
            "voc lista lista"
        );
    }

    #[test]
    fn homogeneous_list_literal_checks() {
        assert!(check_src("ni lista xs = [°I, °II, °III];").is_ok());
        assert!(check_src("voc lista ns = [\"a\", \"b\"];").is_ok());
    }

    #[test]
    fn mixed_element_list_is_rejected() {
        let err = check_src("ni lista xs = [°I, \"a\"];").unwrap_err();
        assert!(err.message.contains("discrepant") || err.message.contains("expectat"),
            "got: {}", err.message);
    }

    #[test]
    fn empty_list_needs_a_declared_element_type() {
        assert!(check_src("ni lista xs = [];").is_ok());
        let err = check_src("scribe([]);").unwrap_err();
        assert!(err.message.contains("vacuae"), "got: {}", err.message);
    }

    #[test]
    fn list_of_fractions_adopts_declared_precision() {
        assert!(check_src("mnr lista xs = [°I:I/II, °I:I/IV];").is_ok());
        assert!(check_src("nr lista ys = [°I:I/II];").is_ok());
    }

    #[test]
    fn indexing_a_list_yields_element_type() {
        assert!(check_src("ni lista xs = [°I, °II]; ni y = xs[°I];").is_ok());
    }

    #[test]
    fn indexing_requires_int_index_and_a_list() {
        assert!(check_src("ni lista xs = [°I]; ni y = xs[°I:I/II];").is_err());
        assert!(check_src("ni n = °V; ni y = n[°I];").is_err());
    }

    #[test]
    fn index_assignment_checks_element_type() {
        assert!(check_src("ni lista xs = [°I, °II]; xs[°I] = °X;").is_ok());
        assert!(check_src("ni lista xs = [°I]; xs[°I] = \"a\";").is_err());
        assert!(check_src("ni n = °V; n[°I] = °X;").is_err());
        assert!(check_src("ni lista xs = [°I]; xs[°I:I/II] = °X;").is_err());
    }

    #[test]
    fn longitudo_returns_int() {
        assert!(check_src("ni lista xs = [°I, °II]; ni n = longitudo(xs);").is_ok());
        assert!(check_src("ni n = longitudo(°V);").is_err());
    }

    #[test]
    fn adde_appends_same_element_type() {
        assert!(check_src("ni lista xs = [°I]; xs = adde(xs, °II);").is_ok());
        assert!(check_src("ni lista xs = [°I]; xs = adde(xs, \"a\");").is_err());
        assert!(check_src("ni lista xs = adde([], °I);").is_ok());
    }

    #[test]
    fn reserved_builtin_names_cannot_be_user_functions() {
        assert!(check_totum(
            "munus adde capit (ni a) reddit ni { redde a; }\nmunus initium reddit ni { }",
        ).is_err());
        assert!(check_totum(
            "munus longitudo reddit ni { redde °I; }\nmunus initium reddit ni { }",
        )
        .is_err());
    }

    #[test]
    fn list_ordering_is_a_type_error() {
        let gt = check_src("ni lista xs = [°I]; ni lista ys = [°II]; scribe(xs maior_est ys);");
        assert!(gt.is_err());
        assert!(gt.unwrap_err().message.contains("ordine"));
        let lt = check_src("ni lista xs = [°I]; ni lista ys = [°II]; scribe(xs minor_est ys);");
        assert!(lt.is_err());
        assert!(lt.unwrap_err().message.contains("ordine"));
    }

    #[test]
    fn list_equality_is_allowed() {
        assert!(check_src("scribe([°I] par_est [°I]);").is_ok());
        assert!(check_src("scribe([°I] par_non_est [°I]);").is_ok());
    }

    use crate::ast::{ParsedFile, ParsedProgram};
    use crate::parser::parse_file;
    use std::collections::BTreeMap;

    fn pfile(path: &str, src: &str) -> ParsedFile {
        ParsedFile { path: path.into(), src: src.into(), ast: parse_file(&tokenize(src).unwrap()).unwrap() }
    }

    fn pprog(root: &[(&str, &str)], pkgs: &[(&str, &[(&str, &str)])]) -> ParsedProgram {
        let mut packages = BTreeMap::new();
        for (name, files) in pkgs {
            packages.insert(
                name.to_string(),
                files.iter().map(|(p, s)| pfile(p, s)).collect(),
            );
        }
        ParsedProgram { root: root.iter().map(|(p, s)| pfile(p, s)).collect(), packages }
    }

    #[test]
    fn program_with_import_and_qualified_call_checks() {
        let mut p = pprog(
            &[("principale.liss",
               "affer mathematica;\nmunus initium reddit ni { scribe(mathematica.summa(°I, °II)); redde nullus; }")],
            &[("mathematica",
               &[("mathematica/summa.liss",
                  "munus summa capit (ni a, ni b) reddit ni { redde a + b; }")])],
        );
        assert!(check_program(&mut p).is_ok());
    }

    #[test]
    fn missing_initium_is_a_structural_error() {
        let mut p = pprog(&[("a.liss", "munus f reddit ni { redde nullus; }")], &[]);
        let e = check_program(&mut p).unwrap_err();
        assert!(e.message.contains("initium"), "got: {}", e.message);
        assert_eq!(e.file, None);
        assert_eq!(e.line, None);
    }

    #[test]
    fn initium_with_wrong_signature_is_rejected() {
        let mut with_params = pprog(
            &[("a.liss", "munus initium capit (ni n) reddit ni { redde n; }")], &[]);
        let e = check_program(&mut with_params).unwrap_err();
        assert!(e.message.contains("nihil capere"), "got: {}", e.message);
        assert_eq!(e.file.as_deref(), Some("a.liss"));

        let mut wrong_ret = pprog(
            &[("a.liss", "munus initium reddit voc { redde \"salve\"; }")], &[]);
        let e = check_program(&mut wrong_ret).unwrap_err();
        assert!(e.message.contains("reddere"), "got: {}", e.message);
    }

    #[test]
    fn duplicate_function_across_files_of_one_folder_is_rejected() {
        let mut p = pprog(
            &[("a.liss", "munus initium reddit ni { redde nullus; }")],
            &[("mathematica", &[
                ("mathematica/a.liss", "munus summa reddit ni { redde °I; }"),
                ("mathematica/b.liss", "munus summa reddit ni { redde °II; }"),
            ])],
        );
        let e = check_program(&mut p).unwrap_err();
        assert!(e.message.contains("bis definita"), "got: {}", e.message);
        assert_eq!(e.file.as_deref(), Some("mathematica/b.liss"));
    }

    #[test]
    fn importing_an_unknown_package_is_rejected() {
        let mut p = pprog(
            &[("a.liss", "affer ignota;\nmunus initium reddit ni { redde nullus; }")], &[]);
        let e = check_program(&mut p).unwrap_err();
        assert!(e.message.contains("non inventus"), "got: {}", e.message);
        assert_eq!(e.file.as_deref(), Some("a.liss"));
        assert_eq!(e.line, Some(1));
    }

    #[test]
    fn qualified_call_without_import_is_rejected() {
        let mut p = pprog(
            &[("a.liss", "munus initium reddit ni { redde mathematica.summa(°I, °II); }")],
            &[("mathematica", &[("mathematica/s.liss",
                "munus summa capit (ni a, ni b) reddit ni { redde a + b; }")])],
        );
        let e = check_program(&mut p).unwrap_err();
        assert!(e.message.contains("non allatus"), "got: {}", e.message);
    }

    #[test]
    fn unknown_function_in_a_real_package_is_rejected() {
        let mut p = pprog(
            &[("a.liss", "affer mathematica;\nmunus initium reddit ni { redde mathematica.ignota(); }")],
            &[("mathematica", &[("mathematica/s.liss",
                "munus summa capit (ni a, ni b) reddit ni { redde a + b; }")])],
        );
        let e = check_program(&mut p).unwrap_err();
        assert!(e.message.contains("in fasciculo"), "got: {}", e.message);
    }

    #[test]
    fn import_cycles_are_allowed() {
        let mut p = pprog(
            &[("a.liss", "affer par;\nmunus initium reddit ni { redde par.aequum(°IV); }")],
            &[
                ("par", &[("par/p.liss",
                    "affer impar;\nmunus aequum capit (ni n) reddit ni { si (n par_est nullus) { redde °I; } redde impar.imparum(n - °I); }")]),
                ("impar", &[("impar/i.liss",
                    "affer par;\nmunus imparum capit (ni n) reddit ni { si (n par_est nullus) { redde nullus; } redde par.aequum(n - °I); }")]),
            ],
        );
        assert!(check_program(&mut p).is_ok());
    }

    #[test]
    fn local_and_imported_functions_may_share_a_name() {
        let mut p = pprog(
            &[("a.liss",
               "affer mathematica;\nmunus summa reddit voc { redde \"summa\"; }\nmunus initium reddit ni { scribe(summa()); redde mathematica.summa(°I, °II); }")],
            &[("mathematica", &[("mathematica/s.liss",
                "munus summa capit (ni a, ni b) reddit ni { redde a + b; }")])],
        );
        assert!(check_program(&mut p).is_ok());
    }

    #[test]
    fn initium_in_a_package_is_an_ordinary_function() {
        let mut p = pprog(
            &[("a.liss", "affer alia;\nmunus initium reddit ni { redde alia.initium(°V); }")],
            &[("alia", &[("alia/x.liss",
                "munus initium capit (ni n) reddit ni { redde n; }")])],
        );
        assert!(check_program(&mut p).is_ok());
    }
}
