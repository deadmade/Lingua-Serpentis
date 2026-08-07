//! Cranelift: the typed IR in, one relocatable ELF object out.
//!
//! # What this covers, and what it does not
//!
//! **Straight-line code over all six scalar types** (Task 12): constants, slots,
//! every binary and unary operator, the four comparisons, and the division and
//! modulus raises. Control flow — `si`, `dum`, `indem`, `abrumpe`, `continua` —
//! is Task 13; lists are Task 15. Everything outside that is an `unimplemented!`
//! naming the task that fills it in, **not** a [`Defectus`]: a Latin sentence
//! saying "this compiler cannot do that yet" would be a claim about the user's
//! program, and it is not one. Nothing user-facing can reach these panics,
//! because no CLI calls [`compila`] until Task 16.
//!
//! `ExprKind::Call` is emitted here, one task before the one that owns it,
//! because `conformance/currunt/exemplum-praecisio` — Task 12's checkpoint —
//! calls a two-parameter `munus`. What is emitted is the direct call and
//! nothing else: mangled cross-package symbols, deep recursion and the
//! stack-exhaustion fixture remain Task 14's, and the machinery a call needs
//! beyond `Module::declare_func_in_func` is exactly zero, because spec §5.4
//! abolished the shadow call stack.
//!
//! # The rule a codegen author is most likely to break, and it is silent
//!
//! **`et` and `vel` evaluate both operands** (spec §3.5). They do not
//! short-circuit: `interp.rs:131-135` evaluates both sides and *then* dispatches
//! on the operator, and the 103-fixture corpus is blessed against exactly that.
//! There is no `And`/`Or` control-flow form in the IR and there must not be one;
//! [`Corpus::expressio`]'s `Binary` arm evaluates `lhs` and then `rhs`
//! unconditionally, and `et`/`vel` are `band`/`bor` over the two results.
//! Emitting the branch a compiler writer's reflexes reach for would be a silent
//! semantic change to every program that relies on it.
//!
//! # The other rule, and it is also silent
//!
//! **`Signature::call_conv` always comes from `module.isa().default_call_conv()`**
//! (spec §9.3). There is one place in this file that builds a `Signature`, and
//! that is why: a literal `CallConv::SystemV` works on both targets today and
//! quietly makes the Windows port a rewrite rather than a recompile. The same
//! function is also the only place an [`AbiParam`] is built, which is what makes
//! the zero-extension of `veritas` and `lit` a property of the file rather than
//! a habit — see [`signum`].
//!
//! # Ownership
//!
//! §5.4's rule — every operation consumes its owned arguments and returns one
//! new owned value — is why there is no temporary set here and never will be.
//! A statement's expression tree leaves exactly **one** live temporary, its
//! root, and only [`StmtKind::ExprStmt`] fails to consume it. The three rules
//! that remain are [`Corpus::sententia`]'s `Assign` (evaluate → free old →
//! store), [`Corpus::blocus`]'s scope exit, and [`Corpus::sententia`]'s `Return`
//! (free every enclosing block, innermost first, never the returned value).
//!
//! Copies happen at exactly one place: `Load` (§7.2). Everything else moves.

use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{AbiParam, InstBuilder, Signature, UserFuncName, Value, types};
use cranelift_codegen::settings::{self, Configurable};
use cranelift_codegen::{Context, isa};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext, Variable};
use cranelift_module::{DataDescription, DataId, FuncId, Linkage, Module, default_libcall_names};
use cranelift_object::object::write::Symbol;
use cranelift_object::{ObjectBuilder, ObjectModule};
use liss_ir::{Block, Const, Expr, ExprKind, Func, Grain, Programma, SlotId, Stmt, StmtKind, Ty};
use liss_runtime::genus::{
    GRADUS, KIND_BOOL, KIND_CHAR, KIND_FRAC_SCRIPULUM, KIND_FRAC_UNCIA, KIND_INT, KIND_TEXT,
};
use liss_runtime::{BinOp, Roman, UnOp};
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

/// The kind code stamped into one value's header (spec §5.3).
///
/// **Imported, not copied.** The constants moved down into `liss_runtime::genus`
/// this task: `liss-abi` is cross-compiled for musl and never linked into the
/// compiler (spec §11), so codegen cannot depend on *it* — but `liss-runtime` is
/// already a dependency of both ends and needs no new row in spec §11's table,
/// so the writer and the reader of these numbers finally share one definition
/// rather than a comment. `liss-abi` re-exports them, so `liss_abi::KIND_INT`
/// still names this constant.
///
/// Only the three numeric codes are *written* by this task, and only by
/// [`Corpus::numerus`]: `voc`'s is stamped inside `liss_text_new`, and `veritas`
/// and `lit` are immediates with no header to stamp. The whole closed form is
/// written out anyway because it is one expression rather than five, and because
/// the container kind is the thing Task 15 reads off a `Ty`.
fn codex(ty: &Ty) -> u32 {
    match ty {
        Ty::Bool => KIND_BOOL,
        Ty::Char => KIND_CHAR,
        Ty::Int => KIND_INT,
        Ty::Frac(Grain::Uncia) => KIND_FRAC_UNCIA,
        Ty::Frac(Grain::Scripulum) => KIND_FRAC_SCRIPULUM,
        Ty::Text => KIND_TEXT,
        Ty::List(inner) => codex(inner) + GRADUS,
    }
}

/// The grain denominators, **read out of the shared table** rather than written
/// again here.
///
/// A second `Grain → 12 | 288` in this file would have been the kind-code
/// duplication one function lower down, and a worse one: the reader is
/// `liss_abi::numeri::exige_granum`, which accepts XII or CCLXXXVIII and rejects
/// everything else — so emitting `12` for an `nr` **passes that gate** and
/// silently produces wrong arithmetic (`°II:I/II * °I:I/XXIV` is 30/288 with the
/// right grain and 720/288 with the wrong one). A wrong answer, not a crash.
///
/// `liss_runtime::genus::granum` returns an `Option` because it is total over
/// every kind code, and these two are the kinds that have a grain. The `Option`
/// is discharged **at compile time**: a `const` initialiser that panics is a
/// build failure, not a runtime one, so this costs neither a branch nor a
/// `Defectus` sentence that could never be raised.
const GRANUM_UNCIAE: i64 = match liss_runtime::genus::granum(KIND_FRAC_UNCIA) {
    Some(g) => g as i64,
    None => panic!("KIND_FRAC_UNCIA is a fractional kind and has a grain"),
};
const GRANUM_SCRIPULI: i64 = match liss_runtime::genus::granum(KIND_FRAC_SCRIPULUM) {
    Some(g) => g as i64,
    None => panic!("KIND_FRAC_SCRIPULUM is a fractional kind and has a grain"),
};

/// The grain denominator a fraction counts in: XII for `mnr`, CCLXXXVIII for
/// `nr` (spec §5.1). It travels as an argument rather than in the value, because
/// the grain is a property of the *type* — which is what lets `ni`, `mnr` and
/// `nr` share `add`, `sub`, `neg` and `cmp` outright.
const fn granum(g: Grain) -> i64 {
    match g {
        Grain::Uncia => GRANUM_UNCIAE,
        Grain::Scripulum => GRANUM_SCRIPULI,
    }
}

/// A slot index outside the function's `slots` table — from a `SlotId` or from
/// `Func::params`.
///
/// One helper for two call sites, which is the pattern `liss_abi::defectus`
/// already uses and the one D-24 established: where two sites raise the same
/// sentence they share a function, so the literal stays unique in the workspace
/// and stays uniquely attributable when Task 19 puts `Defectus::novus(` in the
/// catalogue's markers.
fn sedes_ignota() -> Defectus {
    Defectus::novus("defectus internus: sedes ignota")
}

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
    for (index, func) in prog.funcs.iter().enumerate() {
        emissor.defini(index, func)?;
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

/// A signature with the target's own calling convention, and with every
/// sub-register parameter **zero-extended**.
///
/// **The only place in this crate that builds a [`Signature`]**, which is what
/// makes spec §9.3's rule checkable rather than a habit: `call_conv` comes from
/// `module.isa().default_call_conv()` and never from a literal. A free function
/// rather than a method because both [`Emissor`] and [`Corpus`] need it while
/// holding different halves of the same borrow.
///
/// # Why the extension, and why it is not optional
///
/// `Ty::Bool` and `Ty::Char` are the only values narrower than a register:
/// `i8` holding 0 or 1, and `i32` holding a Unicode scalar value (spec §5.1).
/// `ArgumentExtension::None` is self-consistent for a liss→liss call — both ends
/// agree to look only at the low bits — and **wrong the moment a runtime entry
/// point is on the other end**. Rust's `extern "C" fn(v: i8)` carries an
/// extension attribute that the callee is entitled to rely on, and Cranelift
/// will not add one for you; a `liss_print_bool` reached with garbage in the
/// upper 56 bits of the register is a call that satisfies neither side's idea of
/// the ABI.
///
/// Zero rather than sign extension, for the two values that exist: `veritas` is
/// 0 or 1 and `lit` is a scalar value below 0x110000, so both are non-negative
/// and the two extensions coincide. Where they coincide, the one that matches
/// how the word is passed *generically* — list elements and `liss_list_get`'s
/// result are `u64`, zero-extended (spec §5.1) — is the one to write, so there
/// is one rule for narrow scalars rather than two.
///
/// Returns are left unextended, and that is not an oversight: this crate never
/// reads more of a returned value than its declared type, so `liss_int_cmp`'s
/// `i8` is used as an `i8` and the bits above it are nobody's business.
fn signum(module: &ObjectModule, params: &[types::Type], ret: Option<types::Type>) -> Signature {
    let mut sig = Signature::new(module.isa().default_call_conv());
    let latitudo = u32::from(module.target_config().pointer_bits());
    sig.params.extend(params.iter().map(|t| {
        let p = AbiParam::new(*t);
        if t.bits() < latitudo { p.uext() } else { p }
    }));
    sig.returns.extend(ret.map(AbiParam::new));
    sig
}

/// The machine type one value of `ty` occupies (spec §5.1): one word, always,
/// and the two unowned scalars are narrower than a pointer.
fn genus(ptr: types::Type, ty: &Ty) -> types::Type {
    match ty {
        Ty::Bool => types::I8,
        Ty::Char => types::I32,
        Ty::Int | Ty::Frac(_) | Ty::Text | Ty::List(_) => ptr,
    }
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
    /// IR [`liss_ir::FuncId`] → the Cranelift `FuncId` it was declared under,
    /// indexed directly: `Programma::funcs[id]` *is* the function `id` names
    /// (spec §4.1), and the checker asserts it as it numbers them, so a `Vec`
    /// built in declaration order is the whole mapping a `Call` needs.
    ordo: Vec<FuncId>,
    /// ABI entry point → its `FuncId`. Declared on first use, so an object only
    /// names the runtime functions its program actually calls.
    externa: HashMap<&'static str, FuncId>,
    /// The entry function, for the [`INGRESSUS`] alias.
    principium: Option<FuncId>,
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
            ordo: Vec::new(),
            externa: HashMap::new(),
            principium: None,
        })
    }

    fn declara(&mut self, func: &Func) -> Result<(), Defectus> {
        let ptr = self.module.target_config().pointer_type();
        // `get(..n)` rather than an index: parameters are slots `0..params`
        // (spec §4.2), so a `params` beyond the slot table is the checker and
        // codegen disagreeing about a function's frame — the same defect
        // `Corpus::sedem` names, and the last slot access in this file that was
        // still a panic.
        let Some(parametri) = func.slots.get(..func.params as usize) else {
            return Err(sedes_ignota());
        };
        let params: Vec<_> = parametri.iter().map(|t| genus(ptr, t)).collect();
        let sig = signum(&self.module, &params, Some(genus(ptr, &func.ret)));
        let id = self
            .module
            .declare_function(&func.symbol, Linkage::Export, &sig)
            .map_err(|_| Defectus::novus("defectus internus: munus declarari non potuit"))?;
        self.ordo.push(id);
        if func.is_entry {
            self.principium = Some(id);
        }
        Ok(())
    }

    fn defini(&mut self, index: usize, func: &Func) -> Result<(), Defectus> {
        let Some(id) = self.ordo.get(index).copied() else {
            return Err(Defectus::novus("defectus internus: munus vocatum non inventum"));
        };
        let ptr = self.module.target_config().pointer_type();
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

            // One `Variable` per slot, declared before anything can use one.
            // Slots are allocated per function and never reused (spec §4.2), so
            // the function's locals are a flat `Vec<Ty>` — exactly the shape
            // `cranelift_frontend::Variable` wants.
            let variabiles: Vec<Variable> =
                func.slots.iter().map(|t| fb.declare_var(genus(ptr, t))).collect();

            // Parameters are slots `0..params` (spec §4.2), and they arrive
            // **owned**: `Func::params` exists for the signature and this store,
            // and is not a free list — the parameter frame *is* the body block's
            // scope, so the ordinary teardown frees them exactly once.
            let argumenta = fb.block_params(ingressus).to_vec();
            for (var, arg) in variabiles.iter().zip(argumenta) {
                fb.def_var(*var, arg);
            }
            // Every other slot is zero-initialised here. For an **owned** slot
            // that is spec §5.4's contract doing its work: `Block::declares`
            // lists every slot bound anywhere in the block regardless of
            // position, so a `redde` taken before a later `Let` frees a slot
            // whose binding never ran — and freeing a null pointer is a
            // deliberate no-op precisely so that walk needs no special case. For
            // an unowned one it is only Cranelift's requirement that a
            // `Variable` be defined on every path reaching a use.
            for (var, ty) in
                variabiles.iter().zip(&func.slots).skip(func.params as usize)
            {
                let nihil = fb.ins().iconst(genus(ptr, ty), 0);
                fb.def_var(*var, nihil);
            }

            let mut corpus = Corpus {
                module: &mut self.module,
                externa: &mut self.externa,
                ordo: &self.ordo,
                fb,
                data: Vec::new(),
                sedes: &func.slots,
                variabiles,
                scida: func.file.as_deref(),
                scidae_datum: None,
                linea: 0,
                ambitus: Vec::new(),
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
        let Some(interior) = self.principium else {
            return Err(Defectus::novus(
                "defectus internus: munus 'initium' in programmate non inventum",
            ));
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
    /// The whole program's declarations, for [`ExprKind::Call`].
    ordo: &'a [FuncId],
    fb: FunctionBuilder<'a>,
    /// Constants declared while walking the body, to be defined once the
    /// builder has released the module.
    data: Vec<(DataId, Vec<u8>)>,
    /// This function's slot types, indexed by [`SlotId`].
    sedes: &'a [Ty],
    /// The `Variable` each slot was declared as, indexed the same way.
    variabiles: Vec<Variable>,
    /// The source file every raise in this function names — `None` in
    /// single-file mode, where a diagnostic reads `Error in linea N:` (spec
    /// §4.1). Emitted as a C string, once, on first use.
    scida: Option<&'a str>,
    scidae_datum: Option<DataId>,
    /// The line of the statement being emitted. `Stmt` carries `line` and
    /// nothing finer, which is exactly the interpreter's granularity — `exec`
    /// passes `s.line` into every `eval` — so it is exactly the attribution the
    /// corpus is blessed against.
    linea: usize,
    /// The blocks currently open, outermost first: what a `redde` frees (spec
    /// §7.3.4). Straight-line code never puts more than the function body on it;
    /// the stack exists because `Return` must walk *every* enclosing scope and
    /// Task 13 is what makes that more than one entry.
    ambitus: Vec<Vec<SlotId>>,
}

impl<'a> Corpus<'a> {
    fn ptr(&self) -> types::Type {
        self.module.target_config().pointer_type()
    }

    /// The `Variable` and declared type of one slot.
    ///
    /// A `Defectus` rather than an index panic: a `SlotId` outside
    /// `Func::slots` is the checker and codegen disagreeing about a function's
    /// frame, which is a compiler defect and not a statement about the user's
    /// program — but it is still not something to discover as a Rust panic
    /// crossing into a user's terminal.
    fn sedem(&self, slot: SlotId) -> Result<(Variable, &'a Ty), Defectus> {
        let sedes: &'a [Ty] = self.sedes;
        match (self.variabiles.get(slot.0 as usize), sedes.get(slot.0 as usize)) {
            (Some(var), Some(ty)) => Ok((*var, ty)),
            _ => Err(sedes_ignota()),
        }
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

    /// Call one ABI entry point that returns nothing.
    fn voca(
        &mut self,
        nomen: &'static str,
        params: &[types::Type],
        args: &[Value],
    ) -> Result<(), Defectus> {
        let id = externum(self.module, self.externa, nomen, params, None)?;
        let refr = self.module.declare_func_in_func(id, self.fb.func);
        self.fb.ins().call(refr, args);
        Ok(())
    }

    /// Call one ABI entry point that returns exactly one value — which, by spec
    /// §5.4, is every *operation* the runtime has: each consumes its owned
    /// arguments and returns one new owned value.
    ///
    /// The index is safe by construction rather than by hope: the signature is
    /// built one line above with exactly one return, and `inst_results` yields
    /// exactly the signature's returns.
    fn voca_unum(
        &mut self,
        nomen: &'static str,
        params: &[types::Type],
        ret: types::Type,
        args: &[Value],
    ) -> Result<Value, Defectus> {
        let id = externum(self.module, self.externa, nomen, params, Some(ret))?;
        let refr = self.module.declare_func_in_func(id, self.fb.func);
        let inst = self.fb.ins().call(refr, args);
        Ok(self.fb.inst_results(inst)[0])
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
    fn constans(&mut self, octeti: &[u8]) -> Result<(Value, usize), Defectus> {
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

    /// The `(file, line)` pair every fallible entry point takes (spec §5.4).
    ///
    /// **Not an ambient locus.** A raise inside a callee must name the callee's
    /// line, and an ambient one would name whatever line the caller last set
    /// unless every call site saved and restored it — which is the shadow call
    /// stack this design abolished. So it travels as two arguments, from here.
    ///
    /// The file name is a **C string**, and null and empty both mean "no file":
    /// spec §4.1 lowers `Func::file: None` to a zero-length name, `liss_panic`
    /// accepts either, and a null costs no `.rodata` at all. One blob per
    /// function, however many raises it contains.
    fn locus(&mut self) -> Result<(Value, Value), Defectus> {
        let ptr = self.ptr();
        let scida = match self.scida {
            None => self.fb.ins().iconst(ptr, 0),
            Some(nomen) => {
                let id = match self.scidae_datum {
                    Some(id) => id,
                    None => {
                        let mut octeti = nomen.as_bytes().to_vec();
                        octeti.push(0);
                        let id = self.module.declare_anonymous_data(false, false).map_err(|_| {
                            Defectus::novus("defectus internus: datum declarari non potuit")
                        })?;
                        self.data.push((id, octeti));
                        self.scidae_datum = Some(id);
                        id
                    }
                };
                let gv = self.module.declare_data_in_func(id, self.fb.func);
                self.fb.ins().symbol_value(ptr, gv)
            }
        };
        let linea = self.fb.ins().iconst(types::I64, self.linea as i64);
        Ok((scida, linea))
    }

    /// Free one value, if its type owns anything (spec §7.3).
    ///
    /// Unconditional, and it can be: **freeing a null pointer is a deliberate
    /// no-op** on the far side (spec §5.4), which is what keeps the scope walk
    /// one uniform loop with no special case for a slot whose `Let` did not run.
    fn liber(&mut self, ty: &Ty, v: Value) -> Result<(), Defectus> {
        let ptr = self.ptr();
        let nomen = match ty {
            Ty::Bool | Ty::Char => return Ok(()),
            // `ni`, `mnr` and `nr` are all one `*mut LissInt`; the grain is a
            // property of the type and never travels in the value.
            Ty::Int | Ty::Frac(_) => "liss_int_free",
            Ty::Text => "liss_text_free",
            Ty::List(_) => "liss_list_free",
        };
        self.voca(nomen, &[ptr], &[v])
    }

    /// A private copy of one value, if its type owns anything.
    ///
    /// **`Load` is the only unconditional copy site** (spec §7.2); everything
    /// else — call arguments, `Let`, `Assign`, list elements — is a move. The
    /// `_copy` family is one of the two that **borrow** rather than consume, and
    /// it is forced: a consuming copy would empty the slot on first read and
    /// make every later `Load` of that variable a use-after-free.
    fn exemplar(&mut self, ty: &Ty, v: Value) -> Result<Value, Defectus> {
        let ptr = self.ptr();
        let nomen = match ty {
            Ty::Bool | Ty::Char => return Ok(v),
            Ty::Int | Ty::Frac(_) => "liss_int_copy",
            Ty::Text => "liss_text_copy",
            Ty::List(_) => "liss_list_copy",
        };
        self.voca_unum(nomen, &[ptr], ptr, &[v])
    }

    /// Free a scope's slots, in **reverse declaration order** (spec §7.3.2).
    fn libera(&mut self, declares: &[SlotId]) -> Result<(), Defectus> {
        for slot in declares.iter().rev() {
            let (var, ty) = self.sedem(*slot)?;
            let v = self.fb.use_var(var);
            self.liber(ty, v)?;
        }
        Ok(())
    }

    fn blocus(&mut self, block: &Block) -> Result<(), Defectus> {
        self.ambitus.push(block.declares.clone());
        for stmt in &block.stmts {
            // **Stop at the first terminator**, because the checker admits a
            // statement after one. Its all-paths analysis asks whether *any*
            // statement returns, not whether the last does (`checker.rs:128-131`
            // — "a `redde` followed by unreachable code still returns, and this
            // language has no dead-code lint to disagree with"), so
            // `redde nullus; scribe("numquam");` is a legal program that the
            // interpreter runs to an empty stdout and exit 0.
            //
            // Without this the `redde` fills the Cranelift block and the next
            // statement adds an instruction to a filled one: a panic in debug,
            // and in release a `Defectus` claiming an internal compiler failure
            // over a program that is simply valid.
            //
            // **Task 13 needs this and not only here.** `abrumpe` and
            // `continua` fill a block the same way, so every arm that walks
            // statements owes the same guard.
            if self.terminatus() {
                break;
            }
            self.sententia(stmt)?;
        }
        // Scope exit (spec §7.3.2) — unless the block already ended in a
        // `redde`, which freed this scope and every enclosing one itself.
        if !self.terminatus() {
            self.libera(&block.declares)?;
        }
        self.ambitus.pop();
        Ok(())
    }

    fn sententia(&mut self, stmt: &Stmt) -> Result<(), Defectus> {
        self.linea = stmt.line;
        match &stmt.kind {
            StmtKind::Let { slot, value } => {
                // A move, not a copy (spec §7.2): the temporary the expression
                // produced becomes the slot's, and there is nothing to free —
                // the slot was zero-initialised at entry and a `Let` is a fresh
                // binding, never a rebinding.
                let v = self.expressio(value)?;
                let (var, _) = self.sedem(*slot)?;
                self.fb.def_var(var, v);
                Ok(())
            }
            StmtKind::Assign { slot, value } => {
                // **Evaluate → free old → store** (spec §7.3.3), and the order
                // is load-bearing in both directions: `x = x + °I` must read `x`
                // before the old value dies, and if the evaluation raises, the
                // slot must still hold a valid pointer for the teardown that
                // never comes — a raise does not unwind, but a half-freed slot
                // would still be a double free if anything later walked it.
                let v = self.expressio(value)?;
                let (var, ty) = self.sedem(*slot)?;
                let vetus = self.fb.use_var(var);
                self.liber(ty, vetus)?;
                self.fb.def_var(var, v);
                Ok(())
            }
            StmtKind::Print(e) => {
                let v = self.expressio(e)?;
                let ptr = self.ptr();
                // Every one of these **consumes** `v` (spec §5.4), so the value
                // built one line above is freed inside the call and nothing is
                // left to tidy up. That is the whole of §7.3.1 for `scribe`.
                match &e.ty {
                    Ty::Bool => self.voca("liss_print_bool", &[types::I8], &[v]),
                    Ty::Char => self.voca("liss_print_char", &[types::I32], &[v]),
                    Ty::Int => self.voca("liss_print_int", &[ptr], &[v]),
                    Ty::Frac(g) => {
                        // The one printer that needs the grain, because `mnr`
                        // and `nr` are the same word and different sentences.
                        let gr = self.fb.ins().iconst(types::I64, granum(*g));
                        self.voca("liss_print_frac", &[ptr, types::I64], &[v, gr])
                    }
                    Ty::Text => self.voca("liss_print_text", &[ptr], &[v]),
                    Ty::List(_) => unimplemented!(
                        "Task 15: `scribe` of a list. `liss_print_list` exists \
                         and consumes its argument, recursing through `ekind`",
                    ),
                }
            }
            StmtKind::ExprStmt(e) => {
                // **The one statement whose root nothing consumes** (spec
                // §7.3.1). Because everything else consumes, "free temporaries
                // at end of statement" collapses to exactly this one call.
                let v = self.expressio(e)?;
                self.liber(&e.ty, v)
            }
            StmtKind::Return(e) => {
                // Spec §7.3.4: evaluate, then free every slot of every enclosing
                // block, innermost first. The returned temporary is never among
                // them — if it came from a `Load` it is a fresh copy.
                //
                // "Every enclosing block" is exhaustive and needs no supplement:
                // the parameter frame *is* the body block, so freeing
                // `0..params` again off `Func::params` would double-free every
                // heap parameter on every `redde`.
                let v = self.expressio(e)?;
                for ambitus in self.ambitus.clone().iter().rev() {
                    self.libera(ambitus)?;
                }
                self.fb.ins().return_(&[v]);
                Ok(())
            }
            StmtKind::IndexAssign { .. } => unimplemented!(
                "Task 15: `IndexAssign`. `liss_list_put` borrows the list, \
                 consumes the index and the value, and frees the element it \
                 displaces",
            ),
            other => unimplemented!(
                "Task 13: control flow — {other:?}. Blocks are sealed the moment \
                 their complete predecessor set is known (spec §7.4), and \
                 `abrumpe`/`continua` free from the current block up to the loop \
                 body but *not* a `dum` header",
            ),
        }
    }

    fn expressio(&mut self, e: &Expr) -> Result<Value, Defectus> {
        match &e.kind {
            ExprKind::Const(c) => self.constantem(c),
            ExprKind::Load(slot) => {
                // The only unconditional copy in the language (spec §7.2). The
                // slot still owns what it held.
                let (var, ty) = self.sedem(*slot)?;
                let v = self.fb.use_var(var);
                self.exemplar(ty, v)
            }
            ExprKind::Unary { op, arg } => self.unarium(*op, arg),
            ExprKind::Binary { op, operand_ty, lhs, rhs } => {
                // **Both operands, unconditionally, in source order.**
                //
                // For `et` and `vel` this is spec §3.5 and not an oversight:
                // `interp.rs:131-135` evaluates both sides and *then* dispatches
                // on the operator, so `verus vel (°V / nullus par_est °I)` is a
                // runtime error today rather than `verus`, and the 103-fixture
                // corpus is blessed against exactly that. The IR has no
                // `And`/`Or` control-flow form, only this node; keep it that
                // way. If short-circuiting is ever wanted it is a language
                // change with its own spec, not a codegen detail.
                let a = self.expressio(lhs)?;
                let b = self.expressio(rhs)?;
                self.binarium(*op, operand_ty, a, b)
            }
            ExprKind::Call { func, args } => {
                // Arguments are **moved** (spec §7.2): the callee consumes them
                // and returns one new owned value, so there is no bookkeeping
                // here at all — no `liss_enter`, no `liss_leave`, no shadow
                // stack. Cross-package symbols and deep recursion are Task 14's;
                // the direct call is here because Task 12's own checkpoint
                // fixture, `exemplum-praecisio`, contains one.
                let mut valores = Vec::with_capacity(args.len());
                for a in args {
                    valores.push(self.expressio(a)?);
                }
                let Some(id) = self.ordo.get(func.0 as usize).copied() else {
                    return Err(Defectus::novus("defectus internus: munus vocatum non inventum"));
                };
                let refr = self.module.declare_func_in_func(id, self.fb.func);
                let inst = self.fb.ins().call(refr, &valores);
                // Every liss function returns exactly one value: the checker
                // gives every `munus` a `reddit` type and `declara` builds the
                // signature from it.
                Ok(self.fb.inst_results(inst)[0])
            }
            other => unimplemented!(
                "Task 15: lists — {other:?}. The entry points exist: \
                 `liss_list_new`/`_get`/`_put`/`_append`/`_len`/`_eq`, and \
                 `codex` already gives a `Ty` its element kind",
            ),
        }
    }

    /// One literal.
    fn constantem(&mut self, c: &Const) -> Result<Value, Defectus> {
        let ptr = self.ptr();
        match c {
            // The two unowned scalars are immediates and nothing more (spec
            // §5.1): `veritas` is an `i8` that is 0 or 1, `lit` an `i32` holding
            // a Unicode scalar value.
            Const::Bool(b) => Ok(self.fb.ins().iconst(types::I8, i64::from(*b))),
            Const::Char(c) => Ok(self.fb.ins().iconst(types::I32, i64::from(u32::from(*c)))),
            Const::Int(r) => self.numerus(KIND_INT, r),
            Const::Frac(g, r) => self.numerus(codex(&Ty::Frac(*g)), r),
            Const::Text(s) => {
                // UTF-8 bytes, validated once inside `liss_text_new`. The length
                // is a byte count, not a character count.
                let (bytes, n) = self.constans(s.as_bytes())?;
                let len = self.fb.ins().iconst(types::I64, n as i64);
                self.voca_unum("liss_text_new", &[ptr, types::I64], ptr, &[bytes, len])
            }
        }
    }

    /// One boxed number — `ni`, `mnr` or `nr` — from a static digit blob.
    ///
    /// The blob is `Roman`'s own representation, byte for byte: little-endian
    /// digits `0..=9`, canonical, the empty payload the unique zero (spec §5.2).
    /// Emitting it as static data is why `liss_int_new` takes a blob rather than
    /// an `i64` — the language's integers are unbounded and no immediate could
    /// carry one — and why the sign is a separate argument.
    fn numerus(&mut self, kind: u32, r: &Roman) -> Result<Value, Defectus> {
        let ptr = self.ptr();
        let (blob, n) = self.constans(r.digits())?;
        let genus = self.fb.ins().iconst(types::I32, i64::from(kind));
        let neg = self.fb.ins().iconst(types::I64, i64::from(r.negative()));
        let len = self.fb.ins().iconst(types::I64, n as i64);
        self.voca_unum(
            "liss_int_new",
            &[types::I32, types::I64, ptr, types::I64],
            ptr,
            &[genus, neg, blob, len],
        )
    }

    /// `non` and unary `-`.
    ///
    /// # `-<literal>` is folded, and this is the first place `Roman::negative()`
    /// matters
    ///
    /// `-°V` parses to `Unary{Neg}` over a `Const` — the lexer builds every
    /// numeral through `from_u64`, so a negative `Const::Int` was unreachable
    /// until now and the `neg` argument of `liss_int_new` was a literal `0` in
    /// every object this compiler had ever emitted. Folding makes it real: the
    /// sign travels in the emitted call rather than through an allocate-negate-
    /// free round trip, and `numeri_negativi_in_constantes_conflantur` is the
    /// first test of the accessor that reads it back.
    ///
    /// **One level, deliberately.** `- -°V` still calls `liss_int_neg` once,
    /// because folding transitively is a constant folder and a constant folder
    /// is a pass, not a match arm. The case that a fold gets wrong is `-nullus`,
    /// and it is `Roman::neg`'s to get right: negating an empty magnitude must
    /// stay the canonical, non-negative zero, or structural equality stops being
    /// numeric equality on the far side.
    fn unarium(&mut self, op: UnOp, arg: &Expr) -> Result<Value, Defectus> {
        match op {
            UnOp::Not => {
                // `veritas` is 0 or 1, so complementing the low bit *is* `non`.
                let v = self.expressio(arg)?;
                Ok(self.fb.ins().bxor_imm_u(v, 1))
            }
            UnOp::Neg => {
                if let ExprKind::Const(c) = &arg.kind {
                    let conflatum = match c {
                        Const::Int(r) => Some(Const::Int(r.neg())),
                        Const::Frac(g, r) => Some(Const::Frac(*g, r.neg())),
                        // The checker refuses `-` on anything else:
                        // `negatio numerum requirit`.
                        _ => None,
                    };
                    if let Some(c) = conflatum {
                        return self.constantem(&c);
                    }
                }
                let v = self.expressio(arg)?;
                let ptr = self.ptr();
                self.voca_unum("liss_int_neg", &[ptr], ptr, &[v])
            }
        }
    }

    /// One binary operator, over two already-evaluated operands.
    ///
    /// The dispatch is on `operand_ty`, **not** on the node's own `ty`: a
    /// comparison yields `veritas` from operands that are not, which is the
    /// whole reason the IR carries the two separately (spec §4.1).
    fn binarium(&mut self, op: BinOp, operand_ty: &Ty, a: Value, b: Value) -> Result<Value, Defectus> {
        let ptr = self.ptr();
        match (op, operand_ty) {
            // `add` and `sub` are **shared** by `ni`, `mnr` and `nr`: adding two
            // counts of the same grain is adding two integers.
            (BinOp::Add, Ty::Int | Ty::Frac(_)) => {
                self.voca_unum("liss_int_add", &[ptr, ptr], ptr, &[a, b])
            }
            (BinOp::Sub, Ty::Int | Ty::Frac(_)) => {
                self.voca_unum("liss_int_sub", &[ptr, ptr], ptr, &[a, b])
            }
            // `mul` and `div` are where the grain finally shows: a fraction's
            // product rescales by XII or CCLXXXVIII, an integer's does not.
            (BinOp::Mul, Ty::Int) => self.voca_unum("liss_int_mul", &[ptr, ptr], ptr, &[a, b]),
            (BinOp::Mul, Ty::Frac(g)) => {
                let gr = self.fb.ins().iconst(types::I64, granum(*g));
                self.voca_unum("liss_frac_mul", &[ptr, ptr, types::I64], ptr, &[a, b, gr])
            }
            // **The divisor is not pre-checked here**, and must not be: the
            // runtime raises `divisio per nullum` / `modulus per nullum` itself,
            // from the one site that carries each sentence. A check emitted here
            // would duplicate the sentence and blind the catalogue's ratchet to
            // whichever copy stayed uncovered (D-24).
            (BinOp::Div, Ty::Int) => {
                let (scida, linea) = self.locus()?;
                self.voca_unum(
                    "liss_int_div",
                    &[ptr, ptr, ptr, types::I64],
                    ptr,
                    &[a, b, scida, linea],
                )
            }
            (BinOp::Div, Ty::Frac(g)) => {
                let gr = self.fb.ins().iconst(types::I64, granum(*g));
                let (scida, linea) = self.locus()?;
                self.voca_unum(
                    "liss_frac_div",
                    &[ptr, ptr, types::I64, ptr, types::I64],
                    ptr,
                    &[a, b, gr, scida, linea],
                )
            }
            (BinOp::Mod, Ty::Int) => {
                let (scida, linea) = self.locus()?;
                self.voca_unum(
                    "liss_int_mod",
                    &[ptr, ptr, ptr, types::I64],
                    ptr,
                    &[a, b, scida, linea],
                )
            }
            // Concatenation is the only arithmetic `voc` has.
            (BinOp::Add, Ty::Text) => {
                self.voca_unum("liss_text_concat", &[ptr, ptr], ptr, &[a, b])
            }
            // **No branch.** See `expressio`'s `Binary` arm and spec §3.5: both
            // operands are already evaluated, so `et` and `vel` are one
            // bitwise instruction over two `i8`s that are 0 or 1.
            (BinOp::And, Ty::Bool) => Ok(self.fb.ins().band(a, b)),
            (BinOp::Or, Ty::Bool) => Ok(self.fb.ins().bor(a, b)),
            (BinOp::Eq | BinOp::Ne | BinOp::Gt | BinOp::Lt, _) => {
                self.comparatio(op, operand_ty, a, b)
            }
            // Unreachable from a valid program: the checker's `check_binary`
            // admits no other pairing, and every combination it does admit is
            // above. Left as a panic rather than a `Defectus` for the reason the
            // module doc gives — a Latin sentence here would be a claim about
            // the user's program, and it would be a false one.
            (op, ty) => unimplemented!(
                "the checker rejects `{op:?}` on `{ty:?}`; codegen should never \
                 have been asked",
            ),
        }
    }

    /// `par_est`, `par_non_est`, `maior_est`, `minor_est`.
    ///
    /// The owned types go through **one three-way compare** each rather than
    /// four predicates: codegen compares the result against an immediate, which
    /// is one instruction, and the runtime carries one implementation of the
    /// ordering. `liss_int_cmp` and `liss_text_cmp` both consume their operands.
    fn comparatio(&mut self, op: BinOp, operand_ty: &Ty, a: Value, b: Value) -> Result<Value, Defectus> {
        let ptr = self.ptr();
        let ordo = match operand_ty {
            // The two unowned scalars are immediates, and the machine's own
            // compare *is* their ordering: `veritas` is 0 or 1 and `lit` is a
            // scalar value below 0x110000, so both are non-negative and the
            // unsigned comparison is the one `Value::cmp` gave them. There is no
            // `liss_bool_cmp` or `liss_char_cmp` and there must not be.
            Ty::Bool | Ty::Char => {
                let cc = match op {
                    BinOp::Eq => IntCC::Equal,
                    BinOp::Ne => IntCC::NotEqual,
                    BinOp::Gt => IntCC::UnsignedGreaterThan,
                    _ => IntCC::UnsignedLessThan,
                };
                return Ok(self.fb.ins().icmp(cc, a, b));
            }
            Ty::Int | Ty::Frac(_) => self.voca_unum("liss_int_cmp", &[ptr, ptr], types::I8, &[a, b])?,
            Ty::Text => self.voca_unum("liss_text_cmp", &[ptr, ptr], types::I8, &[a, b])?,
            Ty::List(_) => unimplemented!(
                "Task 15: comparing lists. `liss_list_eq` exists and consumes \
                 both; the checker already refuses `maior_est`/`minor_est` on a \
                 series, so only `par_est`/`par_non_est` reach here",
            ),
        };
        // `-1`, `0` or `1`, and **signed**: `minor_est` on `ni` is the `-1`
        // case, and an unsigned compare against zero would read it as 255.
        let cc = match op {
            BinOp::Eq => IntCC::Equal,
            BinOp::Ne => IntCC::NotEqual,
            BinOp::Gt => IntCC::SignedGreaterThan,
            _ => IntCC::SignedLessThan,
        };
        Ok(self.fb.ins().icmp_imm_s(cc, ordo, 0))
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

    /// **The two narrow scalars cross the ABI zero-extended, and a pointer does
    /// not.**
    ///
    /// `ArgumentExtension::None` on `veritas` and `lit` is self-consistent for a
    /// liss→liss call and wrong the moment a runtime entry point is on the other
    /// end — `liss_print_bool` reached with garbage in the upper 56 bits of its
    /// register satisfies neither side's idea of the ABI, and Cranelift adds no
    /// extension it was not asked for. Asserted here rather than left to a
    /// behavioural test because the wrong answer is *usually* invisible: the
    /// register is usually already clean.
    #[test]
    fn the_two_narrow_scalars_cross_the_abi_zero_extended() {
        use cranelift_codegen::ir::ArgumentExtension;

        for machina in Target::OMNES {
            let emissor = Emissor::novus(machina).unwrap();
            let ptr = emissor.module.target_config().pointer_type();
            let sig = signum(
                &emissor.module,
                &[genus(ptr, &Ty::Bool), genus(ptr, &Ty::Char), genus(ptr, &Ty::Int)],
                Some(ptr),
            );
            assert_eq!(sig.params[0].extension, ArgumentExtension::Uext, "veritas");
            assert_eq!(sig.params[1].extension, ArgumentExtension::Uext, "lit");
            assert_eq!(
                sig.params[2].extension,
                ArgumentExtension::None,
                "{}: a full-width pointer must carry no extension",
                machina.triple(),
            );
            assert_eq!(
                sig.returns[0].extension,
                ArgumentExtension::None,
                "{}: this crate never reads more of a return than its type",
                machina.triple(),
            );
        }
    }

    /// The call convention is the target's own, never a literal (spec §9.3).
    ///
    /// **Asserted on the `Signature` [`signum`] builds, not on the ISA.** The
    /// earlier form compared `isa.default_call_conv()` with
    /// `CallConv::triple_default(isa.triple())`, which is a property of
    /// Cranelift and holds no matter what this crate does — it would have passed
    /// unchanged over a `signum` that wrote `CallConv::SystemV`. What the rule
    /// is about is the signature, so that is what is inspected.
    #[test]
    fn each_signature_carries_its_target_s_own_call_convention() {
        for machina in Target::OMNES {
            let emissor = Emissor::novus(machina).unwrap();
            let ptr = emissor.module.target_config().pointer_type();
            let sig = signum(&emissor.module, &[ptr], Some(ptr));
            assert_eq!(
                sig.call_conv,
                cranelift_codegen::isa::CallConv::triple_default(
                    emissor.module.isa().triple()
                ),
                "{}: a literal call convention works on both targets today and \
                 makes the Windows port a rewrite",
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
