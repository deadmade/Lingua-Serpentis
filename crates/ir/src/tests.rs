use crate::*;
use runtime::{BinOp, Roman};

#[test]
fn symbols_are_injective_over_unicode_identifiers() {
    assert_eq!(symbol(None, "initium"), "liss_0__7_initium");
    assert_eq!(symbol(Some("mathematica"), "duplum"), "liss_11_mathematica_6_duplum");
    assert_ne!(symbol(None, "aê"), symbol(None, "a_u00ea_"));
    // a digit after `liss_`, where every runtime entry point has a letter
    assert!(symbol(None, "x")[5..6].chars().all(|c| c.is_ascii_digit()));
}

#[test]
fn a_package_function_and_a_root_function_mangle_differently() {
    assert_ne!(symbol(None, "duplum"), symbol(Some("mathematica"), "duplum"));
    // `None` and `Some("")` are the same zero-length namespace, and no real package is `""`
    assert_eq!(symbol(None, "x"), symbol(Some(""), "x"));
}

#[test]
fn escape_collision_is_prevented_by_length_prefixing() {
    let a = symbol(None, "aê");
    let b = symbol(None, "a_u00ea_");
    assert_eq!(a, "liss_0__6_a_uea_");
    assert_eq!(b, "liss_0__8_a_u00ea_");
    assert_ne!(a, b);
}

#[test]
fn hand_built_two_function_programma_round_trips() {
    // munus duplica(ni n) redde { redde n * °II; }
    let duplica_symbol = symbol(None, "duplica");
    let duplica = Func {
        symbol: duplica_symbol.clone(),
        file: Some("programma/duplica.liss".to_string()),
        is_entry: false,
        params: 1,
        slots: vec![Ty::Int],
        ret: Ty::Int,
        body: Block {
            stmts: vec![Stmt {
                line: 3,
                kind: StmtKind::Return(Expr {
                    ty: Ty::Int,
                    kind: ExprKind::Binary {
                        op: BinOp::Mul,
                        operand_ty: Ty::Int,
                        lhs: Box::new(Expr { ty: Ty::Int, kind: ExprKind::Load(SlotId(0)) }),
                        rhs: Box::new(Expr {
                            ty: Ty::Int,
                            kind: ExprKind::Const(Const::Int(Roman::from_i64(2))),
                        }),
                    },
                }),
            }],
            declares: vec![],
        },
    };

    // munus initium() redde ni { redde duplica(°V); }
    let initium = Func {
        symbol: symbol(None, "initium"),
        file: Some("programma/initium.liss".to_string()),
        is_entry: true,
        params: 0,
        slots: vec![],
        ret: Ty::Int,
        body: Block {
            stmts: vec![Stmt {
                line: 2,
                kind: StmtKind::Return(Expr {
                    ty: Ty::Int,
                    kind: ExprKind::Call {
                        func: FuncId(1),
                        args: vec![Expr {
                            ty: Ty::Int,
                            kind: ExprKind::Const(Const::Int(Roman::from_i64(5))),
                        }],
                    },
                }),
            }],
            declares: vec![],
        },
    };

    let programma = Programma { funcs: vec![initium.clone(), duplica.clone()] };

    assert_eq!(programma.funcs.len(), 2);
    assert_eq!(programma.funcs[0], initium);
    assert_eq!(programma.funcs[1], duplica);
    assert!(programma.funcs[0].is_entry);
    assert!(!programma.funcs[1].is_entry);
    assert_eq!(programma.funcs[0].symbol, "liss_0__7_initium");
    assert_eq!(programma.funcs[1].symbol, duplica_symbol);

    match &programma.funcs[0].body.stmts[0].kind {
        StmtKind::Return(Expr { kind: ExprKind::Call { func, args }, .. }) => {
            assert_eq!(programma.funcs[func.0 as usize].symbol, duplica_symbol);
            assert_eq!(args.len(), 1);
        }
        other => panic!("expected a Return of a Call, got {other:?}"),
    }

    match &programma.funcs[1].body.stmts[0].kind {
        StmtKind::Return(Expr {
            ty,
            kind: ExprKind::Binary { op, operand_ty, lhs, rhs },
        }) => {
            assert_eq!(*ty, Ty::Int);
            assert_eq!(*op, BinOp::Mul);
            assert_eq!(*operand_ty, Ty::Int);
            assert_eq!(lhs.ty, Ty::Int);
            assert!(matches!(lhs.kind, ExprKind::Load(SlotId(0))));
            assert_eq!(rhs.ty, Ty::Int);
            assert!(matches!(&rhs.kind, ExprKind::Const(Const::Int(n)) if *n == Roman::from_i64(2)));
        }
        other => panic!("expected a Return of a Binary, got {other:?}"),
    }
}

#[test]
fn func_file_is_optional_for_single_file_mode() {
    let f = Func {
        symbol: symbol(None, "initium"),
        file: None,
        is_entry: true,
        params: 0,
        slots: vec![],
        ret: Ty::Int,
        body: Block { stmts: vec![], declares: vec![] },
    };
    assert_eq!(f.file, None);
}

#[test]
fn grain_distinguishes_uncia_from_scripulum() {
    assert_ne!(Ty::Frac(Grain::Uncia), Ty::Frac(Grain::Scripulum));
}

#[test]
fn block_declares_carries_slot_ids_not_names() {
    let block = Block { stmts: vec![], declares: vec![SlotId(0), SlotId(1)] };
    assert_eq!(block.declares, vec![SlotId(0), SlotId(1)]);
}
