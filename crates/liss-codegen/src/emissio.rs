//! Cranelift: the typed IR in, one relocatable ELF object out.
//!
//! # What this walking skeleton covers, and what it does not
//!
//! `Const(Text)`, `Print` and `Return` of an integer constant — exactly the
//! program `conformance/currunt/exemplum-salve` is, and nothing more. Scalars
//! and slots are Task 12, control flow Task 13. Everything outside that is an
//! `unimplemented!` naming the task that fills it in, **not** a [`Defectus`]:
//! a Latin sentence saying "this compiler cannot do that yet" would be a claim
//! about the user's program, and it is not one. Nothing user-facing can reach
//! these panics, because no CLI calls [`compila`] until Task 15.
//!
//! # The one rule a codegen author is most likely to break
//!
//! **`Signature::call_conv` always comes from `module.isa().default_call_conv()`**
//! (spec §9.3). There is one place in this file that builds a `Signature`, and
//! that is why: a literal `CallConv::SystemV` works on both targets today and
//! quietly makes the Windows port a rewrite rather than a recompile.
//!
//! # Ownership
//!
//! §5.4's rule — every operation consumes its owned arguments and returns one
//! new owned value — is why there is no temporary set here and never will be.
//! `liss_print_text` consumes the text `liss_text_new` produced, so `scribe`
//! is two calls and no bookkeeping; the `ni` that `Return` builds is handed to
//! the caller, and `main` frees it.

use cranelift_codegen::ir::{AbiParam, InstBuilder, Signature, UserFuncName, types};
use cranelift_codegen::settings::{self, Configurable};
use cranelift_codegen::{Context, isa};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{DataDescription, FuncId, Linkage, Module, default_libcall_names};
use cranelift_object::object::write::Symbol;
use cranelift_object::{ObjectBuilder, ObjectModule};
use liss_ir::{Block, Const, Expr, ExprKind, Func, Programma, Stmt, StmtKind, Ty};
use std::collections::HashMap;

use crate::Defectus;
use crate::target::Target;

/// The symbol the runtime leaves undefined and compiled code supplies (spec
/// §6).
///
/// It is *not* the entry function's mangled name — `initium` mangles to
/// `liss_0__7_initium` like any other root function, because the user may call
/// it recursively and needs a name to call. So the entry point carries **two**
/// symbols at one address: its own, and this one. See [`Emissor::finge`].
const INGRESSUS: &str = "liss_initium";

/// `KIND_INT` from `liss_abi` (spec §5.3, `base: … 2 Int …`).
///
/// **Copied, not imported.** `liss-abi` is cross-compiled for musl and is never
/// linked into the compiler (spec §11), so codegen cannot depend on it and the
/// two copies of this number are held together by nothing but this comment. It
/// is the only kind code this task needs; Tasks 12–13 will need the rest, and
/// whoever adds them should decide then whether one shared no-std crate of
/// constants is worth its own entry in the dependency table.
const GENUS_NI: i64 = 2;

/// Compile a whole program to one relocatable ELF object for `machina`.
///
/// The bytes are `programma.o` in [`crate::target::Runtime::artefacta`]'s link
/// line. Nothing is written anywhere.
pub fn compila(prog: &Programma, machina: &Target) -> Result<Vec<u8>, Defectus> {
    let mut emissor = Emissor::novus(*machina)?;

    // Two passes, because a call may name a function declared later in the
    // program — and because `Func::symbol` is the *only* thing tying an IR
    // `FuncId` to a linker symbol, so every function must be declared before
    // any body is emitted.
    for func in &prog.funcs {
        emissor.declara(func)?;
    }
    for func in &prog.funcs {
        emissor.defini(func)?;
    }
    emissor.finge()
}

/// The ISA for a target, with the four settings spec §8.3 and §9.3 make
/// load-bearing.
fn machina_isa(machina: Target) -> Result<isa::OwnedTargetIsa, Defectus> {
    let internus = || Defectus::novus("defectus internus: machina institui non potuit");

    let mut fb = settings::builder();
    fb.set("opt_level", "speed").map_err(|_| internus())?;
    // Static, non-PIE (spec §2). With `is_pic` on, every constant reference
    // would go through a GOT that a `-static --no-pie` link has no dynamic
    // loader to fill in.
    fb.set("is_pic", "false").map_err(|_| internus())?;
    // Spec §8.3, and both halves fail *silently* if omitted. A function whose
    // frame is larger than one page can move the stack pointer past the guard
    // page in a single `sub`, touch valid memory below it, and never fault —
    // so the SIGSEGV handler of §8 never runs and deep recursion goes back to
    // being memory corruption instead of `acervus vocationum exhaustus est`.
    fb.set("enable_probestack", "true").map_err(|_| internus())?;
    // `inline` rather than the default `outline`: an outline probe is a call to
    // a `__cranelift_probestack` libcall that nothing in the embedded runtime
    // defines, so the link would fail — and it would fail only for programs
    // that happen to have a large frame.
    fb.set("probestack_strategy", "inline").map_err(|_| internus())?;

    let vexilla = settings::Flags::new(fb);
    let triplex: target_lexicon::Triple = machina.triple().parse().map_err(|_| internus())?;
    isa::lookup(triplex).map_err(|_| internus())?.finish(vexilla).map_err(|_| internus())
}

/// A signature with the target's own calling convention.
///
/// **The only place in this crate that builds a [`Signature`]**, which is what
/// makes spec §9.3's rule checkable rather than a habit: `call_conv` comes from
/// `module.isa().default_call_conv()` and never from a literal. A free function
/// rather than a method because both [`Emissor`] and [`Corpus`] need it while
/// holding different halves of the same borrow.
fn signum(module: &ObjectModule, params: &[types::Type], ret: Option<types::Type>) -> Signature {
    let mut sig = Signature::new(module.isa().default_call_conv());
    sig.params.extend(params.iter().map(|t| AbiParam::new(*t)));
    sig.returns.extend(ret.map(AbiParam::new));
    sig
}

/// Declare one ABI entry point, once per object.
///
/// `externa` is the memo, so an object names only the runtime functions its
/// program actually calls — an undefined symbol nothing references still costs
/// nothing at link time, but it makes the object's symbol table a lie about
/// what the program does.
fn externum(
    module: &mut ObjectModule,
    externa: &mut HashMap<&'static str, FuncId>,
    nomen: &'static str,
    params: &[types::Type],
    ret: Option<types::Type>,
) -> Result<FuncId, Defectus> {
    if let Some(id) = externa.get(nomen) {
        return Ok(*id);
    }
    let sig = signum(module, params, ret);
    let id = module
        .declare_function(nomen, Linkage::Import, &sig)
        .map_err(|_| Defectus::novus("defectus internus: munus declarari non potuit"))?;
    externa.insert(nomen, id);
    Ok(id)
}

/// One compilation.
struct Emissor {
    module: ObjectModule,
    ctx: Context,
    fbctx: FunctionBuilderContext,
    /// IR symbol → the `FuncId` it was declared under.
    nostra: HashMap<String, FuncId>,
    /// ABI entry point → its `FuncId`. Declared on first use, so an object only
    /// names the runtime functions its program actually calls.
    externa: HashMap<&'static str, FuncId>,
    /// The entry function's mangled symbol, for the [`INGRESSUS`] trampoline.
    principium: Option<String>,
}

impl Emissor {
    fn novus(machina: Target) -> Result<Emissor, Defectus> {
        let isa = machina_isa(machina)?;
        let mut aedificator =
            ObjectBuilder::new(isa, "programma", default_libcall_names())
                .map_err(|_| Defectus::novus("defectus internus: objectum institui non potuit"))?;
        // `liss_panic` never unwinds (spec §6): it flushes and exits, and
        // `redde`/`abrumpe`/`continua` are syntactic rather than exceptional.
        // So there is no unwinder to feed, and `.eh_frame` would be dead weight
        // in every compiled binary.
        aedificator.unwind_info(false);
        Ok(Emissor {
            module: ObjectModule::new(aedificator),
            ctx: Context::new(),
            fbctx: FunctionBuilderContext::new(),
            nostra: HashMap::new(),
            externa: HashMap::new(),
            principium: None,
        })
    }

    /// The machine type one value of `ty` occupies (spec §5.1): one word,
    /// always, and the two unowned scalars are narrower than a pointer.
    fn genus(&self, ty: &Ty) -> types::Type {
        match ty {
            Ty::Bool => types::I8,
            Ty::Char => types::I32,
            Ty::Int | Ty::Frac(_) | Ty::Text | Ty::List(_) => {
                self.module.target_config().pointer_type()
            }
        }
    }

    fn declara(&mut self, func: &Func) -> Result<(), Defectus> {
        let params: Vec<_> = func.slots[..func.params as usize]
            .iter()
            .map(|t| self.genus(t))
            .collect();
        let sig = signum(&self.module, &params, Some(self.genus(&func.ret)));
        let id = self
            .module
            .declare_function(&func.symbol, Linkage::Export, &sig)
            .map_err(|_| Defectus::novus("defectus internus: munus declarari non potuit"))?;
        self.nostra.insert(func.symbol.clone(), id);
        if func.is_entry {
            self.principium = Some(func.symbol.clone());
        }
        Ok(())
    }

    fn defini(&mut self, func: &Func) -> Result<(), Defectus> {
        assert!(
            func.slots.is_empty(),
            "Task 12: slots. `{}` has {} of them, and every owned one must be \
             zero-initialised at entry and freed from its `Block::declares` \
             (spec §7.3)",
            func.symbol,
            func.slots.len(),
        );

        let id = self.nostra[&func.symbol];
        self.ctx.clear();
        self.ctx.func.signature = self.module.declarations().get_function_decl(id).signature.clone();
        self.ctx.func.name = UserFuncName::user(0, id.as_u32());

        let data = {
            let mut fb = FunctionBuilder::new(&mut self.ctx.func, &mut self.fbctx);
            let ingressus = fb.create_block();
            fb.append_block_params_for_function_params(ingressus);
            fb.switch_to_block(ingressus);
            // Spec §7.4: seal the moment the complete predecessor set is known.
            // For the entry block that is at creation — it has none.
            fb.seal_block(ingressus);

            let mut corpus = Corpus {
                module: &mut self.module,
                externa: &mut self.externa,
                fb,
                data: Vec::new(),
            };
            corpus.blocus(&func.body)?;

            // The checker proves every function returns on every path, and
            // lowering appends `Return(zero)` to root `initium` (spec §4.4), so
            // an unterminated block here is a codegen bug rather than a program
            // that can fall off its end. A trap says so at the one moment it
            // could matter instead of letting Cranelift's verifier say it in
            // its own words.
            if !corpus.terminatus() {
                corpus.fb.ins().trap(cranelift_codegen::ir::TrapCode::unwrap_user(1));
            }
            let data = std::mem::take(&mut corpus.data);
            let config = corpus.module.target_config();
            corpus.fb.finalize(config);
            data
        };
        // The blobs are declared inside the body walk but can only be *defined*
        // once the builder has let go of the module.
        for (id, octeti) in data {
            let mut descriptio = DataDescription::new();
            descriptio.define(octeti.into_boxed_slice());
            // Bytes, not words: both a UTF-8 literal and a little-endian digit
            // string are read one `u8` at a time on the far side.
            descriptio.set_align(1);
            self.module
                .define_data(id, &descriptio)
                .map_err(|_| Defectus::novus("defectus internus: datum definiri non potuit"))?;
        }

        self.module
            .define_function(id, &mut self.ctx)
            .map_err(|_| Defectus::novus("defectus internus: munus componi non potuit"))?;
        Ok(())
    }

    /// Serialise the object, giving the entry point its second name on the way
    /// out.
    ///
    /// # The alias, and why it is not a trampoline
    ///
    /// An ELF alias is not an assembler `.set`: it is a second symbol-table
    /// entry pointing at the same `(section, offset)` with the same size and
    /// kind. `cranelift-module` has no concept of one — a `FuncId` is a
    /// declaration and a declaration is a name — but `cranelift-object` hands
    /// back the [`object::write::Object`] it built, and `object` has supported
    /// exactly this all along. So the entry point is emitted **once** and named
    /// twice, which is what an alias should cost: nothing.
    ///
    /// `Symbol` is not `Clone`, hence the field-by-field copy. Every field
    /// carried is `Copy`, and every one of them must be carried: a `value` or
    /// `section` that drifted from the original would put `liss_initium`
    /// somewhere that is not `initium`, and the link would still succeed.
    fn finge(self) -> Result<Vec<u8>, Defectus> {
        let interior = {
            let Some(symbolum) = self.principium.as_deref() else {
                return Err(Defectus::novus(
                    "defectus internus: munus 'initium' in programmate non inventum",
                ));
            };
            self.nostra[symbolum]
        };

        let mut productum = self.module.finish();
        let principale = productum.object.symbol(productum.function_symbol(interior));
        // Read out before the mutable borrow below. All `Copy`.
        let (value, size, kind, scope, weak, section, flags) = (
            principale.value,
            principale.size,
            principale.kind,
            principale.scope,
            principale.weak,
            principale.section,
            principale.flags,
        );
        productum.object.add_symbol(Symbol {
            name: INGRESSUS.as_bytes().to_vec(),
            value,
            size,
            kind,
            scope,
            weak,
            section,
            flags,
        });

        productum
            .emit()
            .map_err(|_| Defectus::novus("defectus internus: objectum scribi non potuit"))
    }
}

/// One function body under construction.
struct Corpus<'a> {
    module: &'a mut ObjectModule,
    externa: &'a mut HashMap<&'static str, FuncId>,
    fb: FunctionBuilder<'a>,
    /// Constants declared while walking the body, to be defined once the
    /// builder has released the module.
    data: Vec<(cranelift_module::DataId, Vec<u8>)>,
}

impl Corpus<'_> {
    fn ptr(&self) -> types::Type {
        self.module.target_config().pointer_type()
    }

    /// Whether the block being built already ends in a terminator.
    ///
    /// Spec §7.4 says to assert `is_filled()` before every `switch_to_block`,
    /// and that is still the rule — but `FunctionBuilder::is_filled` is
    /// **private** as of `cranelift-frontend` 0.134, so the question has to be
    /// put to the layout instead. The answer is the same one: the last
    /// instruction of the current block, if any, is a terminator.
    fn terminatus(&self) -> bool {
        self.fb
            .current_block()
            .and_then(|b| self.fb.func.layout.last_inst(b))
            .is_some_and(|i| self.fb.func.dfg.insts[i].opcode().is_terminator())
    }

    /// Call one ABI entry point.
    fn voca(
        &mut self,
        nomen: &'static str,
        params: &[types::Type],
        ret: Option<types::Type>,
        args: &[cranelift_codegen::ir::Value],
    ) -> Result<Vec<cranelift_codegen::ir::Value>, Defectus> {
        let id = externum(self.module, self.externa, nomen, params, ret)?;
        let refr = self.module.declare_func_in_func(id, self.fb.func);
        let inst = self.fb.ins().call(refr, args);
        Ok(self.fb.inst_results(inst).to_vec())
    }

    /// A pointer to a fresh read-only blob, and its length — the `(ptr, len)`
    /// pair every `liss_*_new` takes.
    ///
    /// **Empty is a null pointer and no blob at all.** Both constructors on the
    /// far side document `len == 0` as not reading the pointer —
    /// `liss_text_new` for `""` and `liss_int_new` for `nullus` — and a
    /// zero-length data object is not something `cranelift-object` will give a
    /// symbol to, so there is nothing to point at even if it did.
    ///
    /// Every constant is anonymous and local: a literal is not a symbol anyone
    /// can name, and two identical literals in one program are two blobs.
    /// Interning them is a size optimisation with no ownership question
    /// attached — both constructors copy — but it is not a walking skeleton's.
    fn constans(
        &mut self,
        octeti: &[u8],
    ) -> Result<(cranelift_codegen::ir::Value, usize), Defectus> {
        let ptr = self.ptr();
        if octeti.is_empty() {
            let nullum = self.fb.ins().iconst(ptr, 0);
            return Ok((nullum, 0));
        }
        // Declaring and defining are two failures and get two sentences, as
        // `munus declarari`/`munus componi` already do for functions. One
        // sentence for both would make the catalogue Task 19 builds report a
        // single site for two unrelated ways of being broken.
        let id = self
            .module
            .declare_anonymous_data(false, false)
            .map_err(|_| Defectus::novus("defectus internus: datum declarari non potuit"))?;
        let gv = self.module.declare_data_in_func(id, self.fb.func);
        self.data.push((id, octeti.to_vec()));
        Ok((self.fb.ins().symbol_value(ptr, gv), octeti.len()))
    }

    fn blocus(&mut self, block: &Block) -> Result<(), Defectus> {
        assert!(
            block.declares.is_empty(),
            "Task 12: `Block::declares` is the ownership machinery (spec §7.3) \
             and nothing here frees anything yet",
        );
        for stmt in &block.stmts {
            self.sententia(stmt)?;
        }
        Ok(())
    }

    fn sententia(&mut self, stmt: &Stmt) -> Result<(), Defectus> {
        match &stmt.kind {
            StmtKind::Print(e) => {
                let v = self.expressio(e)?;
                match &e.ty {
                    // Consumes `v` (spec §5.4), so the text built one line
                    // above is freed inside the call and nothing is left to
                    // tidy up. That is the whole of §7.3.1 for this statement.
                    Ty::Text => {
                        let ptr = self.ptr();
                        self.voca("liss_print_text", &[ptr], None, &[v])?;
                    }
                    other => unimplemented!(
                        "Task 12: `scribe` of {other:?}. The entry points exist \
                         — liss_print_bool, _char, _int, _frac, _list — and \
                         `liss_print_frac` additionally takes the grain",
                    ),
                }
                Ok(())
            }
            StmtKind::Return(e) => {
                // Spec §7.3.4 frees every slot of every enclosing block here,
                // innermost first, and never the returned temporary. There are
                // no slots in this task's scope, so the walk is empty — the
                // assertion in `blocus` is what keeps that true.
                let v = self.expressio(e)?;
                self.fb.ins().return_(&[v]);
                Ok(())
            }
            other => unimplemented!("Tasks 12–13: {other:?}"),
        }
    }

    fn expressio(&mut self, e: &Expr) -> Result<cranelift_codegen::ir::Value, Defectus> {
        match &e.kind {
            ExprKind::Const(Const::Text(s)) => {
                let ptr = self.ptr();
                // UTF-8 bytes, validated once inside `liss_text_new`. The
                // length is a byte count, not a character count.
                let (bytes, n) = self.constans(s.as_bytes())?;
                let len = self.fb.ins().iconst(types::I64, n as i64);
                let r = self.voca("liss_text_new", &[ptr, types::I64], Some(ptr), &[bytes, len])?;
                Ok(r[0])
            }
            ExprKind::Const(Const::Int(r)) => {
                let ptr = self.ptr();
                // `Roman`'s own representation, byte for byte: little-endian
                // digits `0..=9`, canonical, the empty payload the unique zero
                // (spec §5.2). Emitting it as static data is why `liss_int_new`
                // takes a blob rather than an `i64` — the language's integers
                // are unbounded and no immediate could carry one.
                let (blob, n) = self.constans(r.digits())?;
                let genus = self.fb.ins().iconst(types::I32, GENUS_NI);
                let neg = self.fb.ins().iconst(types::I64, i64::from(r.negative()));
                let len = self.fb.ins().iconst(types::I64, n as i64);
                let out = self.voca(
                    "liss_int_new",
                    &[types::I32, types::I64, ptr, types::I64],
                    Some(ptr),
                    &[genus, neg, blob, len],
                )?;
                Ok(out[0])
            }
            other => unimplemented!("Tasks 12–13: {other:?}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use object::{Object, ObjectSymbol};

    /// A `Programma` with one `initium` that prints `nuntius` and returns zero
    /// — the shape of `conformance/currunt/exemplum-salve`, built by hand so
    /// this module's tests do not depend on the front end.
    fn salve(nuntius: &str) -> Programma {
        Programma {
            funcs: vec![Func {
                symbol: liss_ir::symbol(None, "initium"),
                file: None,
                is_entry: true,
                params: 0,
                slots: Vec::new(),
                ret: Ty::Int,
                body: Block {
                    stmts: vec![
                        Stmt {
                            line: 2,
                            kind: StmtKind::Print(Expr {
                                ty: Ty::Text,
                                kind: ExprKind::Const(Const::Text(nuntius.to_owned())),
                            }),
                        },
                        Stmt {
                            line: 3,
                            kind: StmtKind::Return(Expr {
                                ty: Ty::Int,
                                kind: ExprKind::Const(Const::Int(liss_runtime::Roman::zero())),
                            }),
                        },
                    ],
                    declares: Vec::new(),
                },
            }],
        }
    }

    /// The object defines what the link needs and asks for exactly the three
    /// ABI entry points this program uses — no more, so an object never drags
    /// in a runtime function the program does not call.
    #[test]
    fn the_object_defines_the_entry_point_and_calls_only_what_it_needs() {
        for machina in Target::OMNES {
            let octeti = compila(&salve("Salve, munde!"), &machina).unwrap();
            let fasciculus = object::File::parse(&octeti[..]).unwrap();

            let definita: Vec<_> = fasciculus
                .symbols()
                .filter(|s| s.is_global() && s.is_definition())
                .collect();
            let nomina: Vec<_> = definita.iter().filter_map(|s| s.name().ok()).collect();
            assert!(
                nomina.contains(&INGRESSUS),
                "{}: the object must define {INGRESSUS} — it is the one symbol \
                 the prelinked runtime leaves undefined",
                machina.triple(),
            );
            assert!(
                nomina.contains(&"liss_0__7_initium"),
                "{}: root `initium` must also keep its own mangled symbol, or a \
                 recursive call to it has nothing to name",
                machina.triple(),
            );

            // **The alias is an alias.** Two names, one definition: same
            // section, same offset, same size. A `liss_initium` that drifted to
            // a different offset would still link, and would call whatever
            // happened to be there.
            let signum = |nomen: &str| {
                definita
                    .iter()
                    .find(|s| s.name() == Ok(nomen))
                    .map(|s| (s.section_index(), s.address(), s.size()))
                    .expect("the symbol is defined")
            };
            assert_eq!(
                signum(INGRESSUS),
                signum("liss_0__7_initium"),
                "{}: {INGRESSUS} must be a second name for `initium`, not a \
                 second copy of it",
                machina.triple(),
            );

            let mut petita: Vec<_> = fasciculus
                .symbols()
                .filter(|s| s.is_undefined())
                .filter_map(|s| s.name().ok())
                .filter(|n| !n.is_empty())
                .collect();
            petita.sort_unstable();
            assert_eq!(
                petita,
                ["liss_int_new", "liss_print_text", "liss_text_new"],
                "{}",
                machina.triple(),
            );
        }
    }

    /// The literal is in the object, verbatim, as its own read-only blob.
    #[test]
    fn a_text_constant_is_static_data() {
        let octeti = compila(&salve("Salve, munde!"), &Target::X86_64).unwrap();
        assert!(
            octeti.windows(13).any(|w| w == b"Salve, munde!"),
            "the literal never reached the object",
        );
    }

    /// The two settings of spec §8.3 that fail silently: without them a
    /// function with a frame larger than a page can step over the guard page,
    /// and deep recursion goes back to being memory corruption.
    #[test]
    fn the_isa_probes_the_stack_inline_and_is_not_pic() {
        for machina in Target::OMNES {
            let isa = machina_isa(machina).unwrap();
            let f = isa.flags();
            assert!(f.enable_probestack(), "{}", machina.triple());
            assert_eq!(
                f.probestack_strategy(),
                settings::ProbestackStrategy::Inline,
                "{}: an outline probe calls a libcall the embedded runtime does \
                 not define",
                machina.triple(),
            );
            assert!(!f.is_pic(), "{}: a static non-PIE link has no GOT", machina.triple());
        }
    }

    /// The call convention is the target's own, never a literal (spec §9.3).
    #[test]
    fn each_target_uses_its_own_default_call_convention() {
        for machina in Target::OMNES {
            let isa = machina_isa(machina).unwrap();
            assert_eq!(
                isa.default_call_conv(),
                cranelift_codegen::isa::CallConv::triple_default(isa.triple()),
                "{}",
                machina.triple(),
            );
        }
    }

    /// A program with no entry point is a diagnostic, not a panic.
    #[test]
    fn a_program_without_initium_is_refused() {
        let prog = Programma { funcs: Vec::new() };
        let e = compila(&prog, &Target::X86_64).unwrap_err();
        assert_eq!(
            e.nuntius,
            "defectus internus: munus 'initium' in programmate non inventum"
        );
    }
}
