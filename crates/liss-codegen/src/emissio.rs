//! Cranelift: the typed IR in, one relocatable ELF object out.

use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{
    AbiParam, InstBuilder, MemFlagsData, Signature, UserFuncName, Value, types,
};
// One machine-level basic block; `liss_ir::Block` is a *lexical* one.
use cranelift_codegen::ir::Block as Nodus;
use cranelift_codegen::settings::{self, Configurable};
use cranelift_codegen::{Context, isa};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext, Variable};
use cranelift_module::{DataDescription, DataId, FuncId, Linkage, Module, default_libcall_names};
use cranelift_object::object::write::Symbol;
use cranelift_object::{ObjectBuilder, ObjectModule};
use liss_ir::{Block, Const, Expr, ExprKind, Func, Grain, Programma, SlotId, Stmt, StmtKind, Ty};
use liss_runtime::dispositio::{ONUS, VERBUM};
use liss_runtime::genus::{
    GRADUS, KIND_BOOL, KIND_CHAR, KIND_FRAC_SCRIPULUM, KIND_FRAC_UNCIA, KIND_INT, KIND_TEXT,
};
use liss_runtime::{BinOp, Roman, UnOp};
use std::collections::HashMap;

use crate::Defectus;
use crate::target::Target;

/// The symbol the runtime leaves undefined and compiled code supplies (spec §6).
const INGRESSUS: &str = "liss_initium";

/// The kind code stamped into one value's header (spec §5.3).
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

/// The grain denominators, read out of the shared table rather than written again.
const GRANUM_UNCIAE: i64 = match liss_runtime::genus::granum(KIND_FRAC_UNCIA) {
    Some(g) => g as i64,
    None => panic!("KIND_FRAC_UNCIA is a fractional kind and has a grain"),
};
const GRANUM_SCRIPULI: i64 = match liss_runtime::genus::granum(KIND_FRAC_SCRIPULUM) {
    Some(g) => g as i64,
    None => panic!("KIND_FRAC_SCRIPULUM is a fractional kind and has a grain"),
};

/// XII for `mnr`, CCLXXXVIII for `nr` (spec §5.1); an argument, never in the value.
const fn granum(g: Grain) -> i64 {
    match g {
        Grain::Uncia => GRANUM_UNCIAE,
        Grain::Scripulum => GRANUM_SCRIPULI,
    }
}

/// A slot index outside the function's `slots` table.
fn sedes_ignota() -> Defectus {
    Defectus::novus("defectus internus: sedes ignota")
}

/// `abrumpe` or `continua` with no loop to name, or a scope depth out of range.
fn saltus_extra_ansam() -> Defectus {
    Defectus::novus("defectus internus: saltus extra ansam")
}

/// Whether a type owns heap memory, and which entry points manage it (spec §5.1).
fn possessio(ty: &Ty) -> Option<(&'static str, &'static str)> {
    match ty {
        Ty::Bool | Ty::Char => None,
        Ty::Int | Ty::Frac(_) => Some(("liss_int_free", "liss_int_copy")),
        Ty::Text => Some(("liss_text_free", "liss_text_copy")),
        Ty::List(_) => Some(("liss_list_free", "liss_list_copy")),
    }
}

fn possidet(ty: &Ty) -> bool {
    possessio(ty).is_some()
}

/// Compile a whole program to one relocatable ELF object. Nothing is written anywhere.
pub fn compila(prog: &Programma, machina: &Target) -> Result<Vec<u8>, Defectus> {
    let mut emissor = Emissor::novus(*machina)?;

    // Two passes: a call may name a function declared later in the program.
    for func in &prog.funcs {
        emissor.declara(func)?;
    }
    for (index, func) in prog.funcs.iter().enumerate() {
        emissor.defini(index, func)?;
    }
    emissor.finge()
}

/// The ISA for a target, with the four settings spec §8.3 and §9.3 make load-bearing.
fn machina_isa(machina: Target) -> Result<isa::OwnedTargetIsa, Defectus> {
    let internus = || Defectus::novus("defectus internus: machina institui non potuit");

    let mut fb = settings::builder();
    fb.set("opt_level", "speed").map_err(|_| internus())?;
    fb.set("is_pic", "false").map_err(|_| internus())?;
    fb.set("enable_probestack", "true").map_err(|_| internus())?;
    fb.set("probestack_strategy", "inline").map_err(|_| internus())?;

    let vexilla = settings::Flags::new(fb);
    let triplex: target_lexicon::Triple = machina.triple().parse().map_err(|_| internus())?;
    isa::lookup(triplex).map_err(|_| internus())?.finish(vexilla).map_err(|_| internus())
}

/// The target's own calling convention, and every sub-register parameter
/// zero-extended. The only place in this crate that builds a [`Signature`].
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

/// The machine type one value of `ty` occupies (spec §5.1).
fn genus(ptr: types::Type, ty: &Ty) -> types::Type {
    match ty {
        Ty::Bool => types::I8,
        Ty::Char => types::I32,
        Ty::Int | Ty::Frac(_) | Ty::Text | Ty::List(_) => ptr,
    }
}


/// Declare one ABI entry point, once per object.
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
        .map_err(|_| Defectus::novus("defectus internus: munus declarari non potuit: externum"))?;
    externa.insert(nomen, id);
    Ok(id)
}

/// One compilation.
struct Emissor {
    module: ObjectModule,
    ctx: Context,
    fbctx: FunctionBuilderContext,
    /// IR `FuncId` → the Cranelift `FuncId` it was declared under, indexed directly.
    ordo: Vec<FuncId>,
    /// ABI entry point → its `FuncId`, declared on first use.
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
        // `liss_panic` never unwinds (spec §6), so there is no unwinder to feed.
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
        // `get(..n)` rather than an index: a `params` beyond the slot table is
        // the checker and codegen disagreeing about a function's frame.
        let Some(parametri) = func.slots.get(..func.params as usize) else {
            return Err(sedes_ignota());
        };
        let params: Vec<_> = parametri.iter().map(|t| genus(ptr, t)).collect();
        let sig = signum(&self.module, &params, Some(genus(ptr, &func.ret)));
        let id = self
            .module
            .declare_function(&func.symbol, Linkage::Export, &sig)
            .map_err(|_| {
                Defectus::novus("defectus internus: munus declarari non potuit: proprium")
            })?;
        self.ordo.push(id);
        if func.is_entry {
            self.principium = Some(id);
        }
        Ok(())
    }

    fn defini(&mut self, index: usize, func: &Func) -> Result<(), Defectus> {
        let Some(id) = self.ordo.get(index).copied() else {
            return Err(Defectus::novus(
                "defectus internus: munus vocatum non inventum: definitio",
            ));
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
            // Spec §7.4: the entry block's predecessor set is complete at creation.
            fb.seal_block(ingressus);

            let variabiles: Vec<Variable> =
                func.slots.iter().map(|t| fb.declare_var(genus(ptr, t))).collect();

            // Parameters are slots `0..params` (spec §4.2), and they arrive owned.
            let argumenta = fb.block_params(ingressus).to_vec();
            for (var, arg) in variabiles.iter().zip(argumenta) {
                fb.def_var(*var, arg);
            }
            // Every other slot is zero-initialised (spec §5.4): a `redde` before a
            // later `Let` frees a slot whose binding never ran, and freeing null is a no-op.
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
                ansae: Vec::new(),
            };
            corpus.blocus(&func.body)?;

            // The checker proves every function returns on every path (spec
            // §4.4), so an unterminated block here is a codegen bug.
            if !corpus.terminatus() {
                corpus.fb.ins().trap(cranelift_codegen::ir::TrapCode::unwrap_user(1));
            }
            let data = std::mem::take(&mut corpus.data);
            let config = corpus.module.target_config();
            corpus.fb.finalize(config);
            data
        };
        // Declarable inside the body walk, definable only once the builder let go.
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

    /// Serialise the object, giving the entry point its second name: one
    /// definition, two symbol-table entries.
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
    /// Constants declared while walking the body, defined once the module is free.
    data: Vec<(DataId, Vec<u8>)>,
    /// This function's slot types, indexed by [`SlotId`].
    sedes: &'a [Ty],
    /// The `Variable` each slot was declared as, indexed the same way.
    variabiles: Vec<Variable>,
    /// The source file every raise in this function names — `None` in
    /// single-file mode (spec §4.1). Emitted as a C string, once, on first use.
    scida: Option<&'a str>,
    scidae_datum: Option<DataId>,
    /// The line of the statement being emitted.
    linea: usize,
    /// The blocks currently open, outermost first: what a `redde` frees (spec
    /// §7.3.4). Every owned slot is in exactly one entry while it is live.
    ambitus: Vec<Vec<SlotId>>,
    /// The loops currently open, outermost first; both keywords bind to the innermost.
    ansae: Vec<Ansa>,
}

/// One enclosing loop, as `abrumpe` and `continua` see it (spec §7.3.5–6).
#[derive(Clone, Copy)]
struct Ansa {
    /// `abrumpe`'s target: the block after the loop.
    exitus: Nodus,
    /// `continua`'s target: `dum`'s step block, `indem`'s condition block.
    /// `conformance/currunt/dum-continua-currit` pins the `dum` case.
    gradus: Nodus,
    /// The index in [`Corpus::ambitus`] of the loop **body**'s scope. A `dum`
    /// header sits at `profunditas - 1` and is deliberately not in that range.
    profunditas: usize,
}

impl<'a> Corpus<'a> {
    fn ptr(&self) -> types::Type {
        self.module.target_config().pointer_type()
    }

    /// The `Variable` and declared type of one slot.
    fn sedem(&self, slot: SlotId) -> Result<(Variable, &'a Ty), Defectus> {
        let sedes: &'a [Ty] = self.sedes;
        match (self.variabiles.get(slot.0 as usize), sedes.get(slot.0 as usize)) {
            (Some(var), Some(ty)) => Ok((*var, ty)),
            _ => Err(sedes_ignota()),
        }
    }

    /// Whether the block being built already ends in a terminator.
    fn terminatus(&self) -> bool {
        self.fb
            .current_block()
            .and_then(|b| self.fb.func.layout.last_inst(b))
            .is_some_and(|i| self.fb.func.dfg.insts[i].opcode().is_terminator())
    }

    /// Move the cursor to `nodus`, having first satisfied `switch_to_block`'s
    /// precondition — which is a disjunction: none, pristine, or filled.
    fn transi(&mut self, nodus: Nodus) {
        debug_assert!(
            match self.fb.current_block() {
                None => true,
                Some(b) => match self.fb.func.layout.last_inst(b) {
                    None => true,
                    Some(i) => self.fb.func.dfg.insts[i].opcode().is_terminator(),
                },
            },
            "a block must be filled or pristine before the cursor leaves it",
        );
        self.fb.switch_to_block(nodus);
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

    /// Call one ABI entry point that returns exactly one value (spec §5.4).
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
    /// pair every `liss_*_new` takes. Empty is a null pointer and no blob at all.
    fn constans(&mut self, octeti: &[u8]) -> Result<(Value, usize), Defectus> {
        let ptr = self.ptr();
        if octeti.is_empty() {
            let nullum = self.fb.ins().iconst(ptr, 0);
            return Ok((nullum, 0));
        }
        let id = self
            .module
            .declare_anonymous_data(false, false)
            .map_err(|_| {
                Defectus::novus("defectus internus: datum declarari non potuit: constans")
            })?;
        let gv = self.module.declare_data_in_func(id, self.fb.func);
        self.data.push((id, octeti.to_vec()));
        Ok((self.fb.ins().symbol_value(ptr, gv), octeti.len()))
    }

    /// The `(file, line)` pair every fallible entry point takes (spec §5.4).
    /// Not an ambient locus: it travels as two arguments.
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
                            Defectus::novus("defectus internus: datum declarari non potuit: scida")
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

    /// Free one value, if its type owns anything (spec §7.3). Freeing null is a no-op.
    fn liber(&mut self, ty: &Ty, v: Value) -> Result<(), Defectus> {
        let ptr = self.ptr();
        let Some((nomen, _)) = possessio(ty) else { return Ok(()) };
        self.voca(nomen, &[ptr], &[v])
    }

    /// A private copy, if the type owns anything: `Load` is the only copy site (§7.2).
    fn exemplar(&mut self, ty: &Ty, v: Value) -> Result<Value, Defectus> {
        let ptr = self.ptr();
        let Some((_, nomen)) = possessio(ty) else { return Ok(v) };
        self.voca_unum(nomen, &[ptr], ptr, &[v])
    }

    /// Free a scope's slots in reverse declaration order (spec §7.3.2), leaving
    /// each one null — which is what makes the free discipline idempotent.
    /// `conformance/currunt/memoria-sedes-post-continua` is the program that
    /// fails without that store.
    fn libera(&mut self, declares: &[SlotId]) -> Result<(), Defectus> {
        let ptr = self.ptr();
        for slot in declares.iter().rev() {
            let (var, ty) = self.sedem(*slot)?;
            if !possidet(ty) {
                continue;
            }
            let v = self.fb.use_var(var);
            self.liber(ty, v)?;
            let nihil = self.fb.ins().iconst(genus(ptr, ty), 0);
            self.fb.def_var(var, nihil);
        }
        Ok(())
    }

    /// Tear down every scope from `profunditas` to the innermost (spec §7.3.4–6).
    fn libera_usque(&mut self, profunditas: usize) -> Result<(), Defectus> {
        let ambitus = self.ambitus.clone();
        let Some(aperti) = ambitus.get(profunditas..) else {
            return Err(saltus_extra_ansam());
        };
        for scopus in aperti.iter().rev() {
            self.libera(scopus)?;
        }
        Ok(())
    }

    /// One lexical block: its statements, then its scope teardown. Not the `dum` header.
    fn blocus(&mut self, block: &Block) -> Result<(), Defectus> {
        // Pushed and popped on every path, the error one included.
        self.ambitus.push(block.declares.clone());
        let r = self.blocus_intus(block);
        self.ambitus.pop();
        r
    }

    fn blocus_intus(&mut self, block: &Block) -> Result<(), Defectus> {
        for stmt in &block.stmts {
            // Stop at the first terminator: the checker admits a statement after one.
            // `sententia_post_abrumpe_neglegitur` covers `abrumpe` and `continua` too.
            if self.terminatus() {
                break;
            }
            self.sententia(stmt)?;
        }
        // Scope exit (spec §7.3.2) — unless a `redde`, `abrumpe` or `continua`
        // already freed this scope on its way out.
        if !self.terminatus() {
            self.libera(&block.declares)?;
        }
        Ok(())
    }

    fn sententia(&mut self, stmt: &Stmt) -> Result<(), Defectus> {
        self.linea = stmt.line;
        match &stmt.kind {
            StmtKind::Let { slot, value } => {
                // A move, not a copy (spec §7.2).
                let v = self.expressio(value)?;
                let (var, _) = self.sedem(*slot)?;
                self.fb.def_var(var, v);
                Ok(())
            }
            StmtKind::Assign { slot, value } => {
                // Evaluate → free old → store (spec §7.3.3); the order is load-bearing.
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
                // Every one of these consumes `v` (spec §5.4).
                match &e.ty {
                    Ty::Bool => self.voca("liss_print_bool", &[types::I8], &[v]),
                    Ty::Char => self.voca("liss_print_char", &[types::I32], &[v]),
                    Ty::Int => self.voca("liss_print_int", &[ptr], &[v]),
                    Ty::Frac(g) => {
                        // The one printer that needs the grain.
                        let gr = self.fb.ins().iconst(types::I64, granum(*g));
                        self.voca("liss_print_frac", &[ptr, types::I64], &[v, gr])
                    }
                    Ty::Text => self.voca("liss_print_text", &[ptr], &[v]),
                    // Recurses on the child's own header, so nesting needs nothing here.
                    Ty::List(_) => self.voca("liss_print_list", &[ptr], &[v]),
                }
            }
            StmtKind::ExprStmt(e) => {
                // The one statement whose root nothing consumes (spec §7.3.1).
                let v = self.expressio(e)?;
                self.liber(&e.ty, v)
            }
            StmtKind::Return(e) => {
                // Spec §7.3.4: evaluate, then free every enclosing block, innermost
                // first. The parameter frame *is* the body block — no `0..params` sweep.
                let v = self.expressio(e)?;
                self.libera_usque(0)?;
                self.fb.ins().return_(&[v]);
                Ok(())
            }
            StmtKind::If { cond, then, else_ } => self.si(cond, then, else_.as_ref()),
            StmtKind::While { cond, body } => self.indem(stmt.line, cond, body),
            StmtKind::For { header, cond, step, body } => {
                self.dum(stmt.line, header, cond, step, body)
            }
            // The two jumps differ only in where they land (spec §7.3.5–6): both free
            // up to and including the loop body, and neither frees a `dum` header.
            StmtKind::Break | StmtKind::Continue => {
                let Some(ansa) = self.ansae.last().copied() else {
                    return Err(saltus_extra_ansam());
                };
                self.libera_usque(ansa.profunditas)?;
                let meta = if matches!(stmt.kind, StmtKind::Break) {
                    ansa.exitus
                } else {
                    ansa.gradus
                };
                self.fb.ins().jump(meta, &[]);
                Ok(())
            }
            // `xs[i] = v` — the one statement that mutates through a slot. The list
            // is borrowed: `liss_list_put` is a non-consuming entry point.
            StmtKind::IndexAssign { slot, index, value } => {
                let ptr = self.ptr();
                // Index, then value. Pinned by `series.rs`'s
                // `ordo_evaluationis_a_fonte_legitur`.
                let i = self.expressio(index)?;
                let v = self.expressio(value)?;
                let verbum = self.verbum(v);
                // Read after both operands: observed at the moment of the write.
                let (var, _) = self.sedem(*slot)?;
                let p = self.fb.use_var(var);
                let (scida, linea) = self.locus()?;
                self.voca(
                    "liss_list_put",
                    &[ptr, ptr, types::I64, ptr, types::I64],
                    &[p, i, verbum, scida, linea],
                )
            }
        }
    }

    /// `si` / `aut` — spec §4.4's flattened form, so there is no else-if arm.
    fn si(
        &mut self,
        cond: &Expr,
        then: &Block,
        else_: Option<&Block>,
    ) -> Result<(), Defectus> {
        let c = self.expressio(cond)?;
        let tunc = self.fb.create_block();
        let aliud = self.fb.create_block();
        let exitus = self.fb.create_block();
        self.fb.ins().brif(c, tunc, &[], aliud, &[]);
        self.fb.seal_block(tunc);
        self.fb.seal_block(aliud);

        let mut pervius = false;
        self.transi(tunc);
        self.blocus(then)?;
        if !self.terminatus() {
            self.fb.ins().jump(exitus, &[]);
            pervius = true;
        }

        self.transi(aliud);
        if let Some(aliter) = else_ {
            self.blocus(aliter)?;
        }
        if !self.terminatus() {
            self.fb.ins().jump(exitus, &[]);
            pervius = true;
        }

        // Sealed only after both arms have had their chance to jump to it, and left
        // pristine when neither falls through.
        self.fb.seal_block(exitus);
        if pervius {
            self.transi(exitus);
        }
        Ok(())
    }

    /// `indem` — a condition-only loop.
    fn indem(&mut self, linea: usize, cond: &Expr, body: &Block) -> Result<(), Defectus> {
        let caput = self.fb.create_block();
        let corpus = self.fb.create_block();
        let exitus = self.fb.create_block();

        self.fb.ins().jump(caput, &[]);
        self.transi(caput);
        // Re-evaluated every iteration; a raise inside it names the `indem`'s own line.
        self.linea = linea;
        let c = self.expressio(cond)?;
        self.fb.ins().brif(c, corpus, &[], exitus, &[]);
        self.fb.seal_block(corpus);

        self.transi(corpus);
        self.ansae.push(Ansa { exitus, gradus: caput, profunditas: self.ambitus.len() });
        let r = self.blocus(body);
        self.ansae.pop();
        r?;
        if !self.terminatus() {
            self.fb.ins().jump(caput, &[]);
        }

        // `caput` is sealed last, after the body's back edge exists.
        self.fb.seal_block(caput);
        self.fb.seal_block(exitus);
        self.transi(exitus);
        Ok(())
    }

    /// `dum` — init, condition, step, and the one loop with a scope of its own.
    fn dum(
        &mut self,
        linea: usize,
        header: &Block,
        cond: &Expr,
        step: &Stmt,
        body: &Block,
    ) -> Result<(), Defectus> {
        // The header's scope opens *before* its init statement, so a `redde` below
        // finds the loop variable. Popped on the error path too.
        self.ambitus.push(header.declares.clone());
        let r = self.dum_intus(linea, header, cond, step, body);
        self.ambitus.pop();
        r
    }

    fn dum_intus(
        &mut self,
        linea: usize,
        header: &Block,
        cond: &Expr,
        step: &Stmt,
        body: &Block,
    ) -> Result<(), Defectus> {
        // A second statement walk, owing the same terminator guard as `blocus`;
        // breaking is not enough, this must return — the `jump caput` below is what
        // would panic. D-26: undecided, not settled, so no fixture may bless it.
        // `regimen.rs`'s `caput_dum_terminatum_componitur` pins it instead, and
        // `generative.rs`'s `caput_dum_terminatum` keeps the generator off the shape.
        for stmt in &header.stmts {
            if self.terminatus() {
                break;
            }
            self.sententia(stmt)?;
        }
        if self.terminatus() {
            return Ok(());
        }

        let caput = self.fb.create_block();
        let corpus = self.fb.create_block();
        let gradus = self.fb.create_block();
        let exitus = self.fb.create_block();

        self.fb.ins().jump(caput, &[]);
        self.transi(caput);
        // Restored after the init clause, which moved it.
        self.linea = linea;
        let c = self.expressio(cond)?;
        self.fb.ins().brif(c, corpus, &[], exitus, &[]);
        self.fb.seal_block(corpus);

        self.transi(corpus);
        self.ansae.push(Ansa { exitus, gradus, profunditas: self.ambitus.len() });
        let r = self.blocus(body);
        self.ansae.pop();
        r?;
        if !self.terminatus() {
            self.fb.ins().jump(gradus, &[]);
        }

        self.fb.seal_block(gradus);
        self.transi(gradus);
        self.sententia(step)?;
        if !self.terminatus() {
            self.fb.ins().jump(caput, &[]);
        }

        self.fb.seal_block(caput);
        self.fb.seal_block(exitus);
        self.transi(exitus);
        // Spec §7.3.5: once, here; the `redde` path never arrives and freed it itself.
        self.libera(&header.declares)
    }

    fn expressio(&mut self, e: &Expr) -> Result<Value, Defectus> {
        match &e.kind {
            ExprKind::Const(c) => self.constantem(c),
            ExprKind::Load(slot) => {
                // The only unconditional copy in the language (spec §7.2).
                let (var, ty) = self.sedem(*slot)?;
                let v = self.fb.use_var(var);
                self.exemplar(ty, v)
            }
            ExprKind::Unary { op, arg } => self.unarium(*op, arg),
            ExprKind::Binary { op, operand_ty, lhs, rhs } => {
                // Both operands, unconditionally: `et` and `vel` do not short-circuit
                // (spec §3.5), and the IR has no `And`/`Or` control-flow form.
                // `conformance/errores/{vel,et}-utrumque-latus` and `scalares.rs`'s
                // `{vel,et}_dextrum_latus_quoque_aestimat` hold the rule.
                let a = self.expressio(lhs)?;
                let b = self.expressio(rhs)?;
                self.binarium(*op, operand_ty, a, b)
            }
            ExprKind::Call { func, args } => {
                // Arguments are moved (spec §7.2): the callee consumes them.
                let mut valores = Vec::with_capacity(args.len());
                for a in args {
                    valores.push(self.expressio(a)?);
                }
                let Some(id) = self.ordo.get(func.0 as usize).copied() else {
                    return Err(Defectus::novus(
                        "defectus internus: munus vocatum non inventum: vocatio",
                    ));
                };
                let refr = self.module.declare_func_in_func(id, self.fb.func);
                let inst = self.fb.ins().call(refr, &valores);
                // Every liss function returns exactly one value.
                Ok(self.fb.inst_results(inst)[0])
            }
            ExprKind::ListLit { items } => self.series(&e.ty, items),
            // `xs[i]` — 1-based, bounds-checked, and a deep copy (spec §5.4).
            // Both operands are consumed inside the call, `base` included.
            ExprKind::Index { base, index } => {
                let ptr = self.ptr();
                let p = self.expressio(base)?;
                let i = self.expressio(index)?;
                let (scida, linea) = self.locus()?;
                let verbum = self.voca_unum(
                    "liss_list_get",
                    &[ptr, ptr, ptr, types::I64],
                    types::I64,
                    &[p, i, scida, linea],
                )?;
                // The result crosses as a `u64` (spec §5.1); `e.ty` is the element type.
                Ok(self.ex_verbo(&e.ty, verbum))
            }
            ExprKind::Len(arg) => {
                let ptr = self.ptr();
                let p = self.expressio(arg)?;
                self.voca_unum("liss_list_len", &[ptr], ptr, &[p])
            }
            // `adde(xs, v)` — a new list one longer; the old one's element words are
            // stolen, not copied (spec §5.4). `tests/series.rs` reads off the object
            // that codegen reaches this entry point and adds nothing.
            ExprKind::Append { list, item } => {
                let ptr = self.ptr();
                let p = self.expressio(list)?;
                let v = self.expressio(item)?;
                let verbum = self.verbum(v);
                self.voca_unum("liss_list_append", &[ptr, types::I64], ptr, &[p, verbum])
            }
        }
    }

    /// A list literal: one `liss_list_new`, then one direct store per element (spec
    /// §5.2). Elements first, so an element that raises leaves no live allocation.
    fn series(&mut self, ty: &Ty, items: &[Expr]) -> Result<Value, Defectus> {
        let ptr = self.ptr();
        // The element kind, off the node's own type rather than `codex(ty) - GRADUS`.
        let Ty::List(elementum) = ty else {
            unimplemented!(
                "the checker types a list literal as a series, not `{ty:?}`; \
                 codegen should never have been asked",
            )
        };

        let mut valores = Vec::with_capacity(items.len());
        for elementum in items {
            valores.push(self.expressio(elementum)?);
        }

        let ekind = self.fb.ins().iconst(types::I64, i64::from(codex(elementum)));
        let longitudo = self.fb.ins().iconst(types::I64, items.len() as i64);
        let p = self.voca_unum(
            "liss_list_new",
            &[types::I64, types::I64],
            ptr,
            &[ekind, longitudo],
        )?;
        for (i, v) in valores.into_iter().enumerate() {
            let verbum = self.verbum(v);
            self.repone(p, i, verbum);
        }
        Ok(p)
    }

    /// Store one element word at `ONUS + VERBUM*i` — the payload's whole layout.
    fn repone(&mut self, p: Value, i: usize, verbum: Value) {
        let discrimen = (i as i64).saturating_mul(VERBUM as i64).saturating_add(ONUS as i64);
        match i32::try_from(discrimen) {
            Ok(offset) => {
                self.fb.ins().store(MemFlagsData::trusted(), verbum, p, offset);
            }
            Err(_) => {
                let sedes = self.fb.ins().iadd_imm_s(p, discrimen);
                self.fb.ins().store(MemFlagsData::trusted(), verbum, sedes, 0);
            }
        }
    }

    /// One value widened to the `u64` element word, zero-extended (spec §5.1).
    fn verbum(&mut self, v: Value) -> Value {
        if self.fb.func.dfg.value_type(v).bits() < 64 {
            self.fb.ins().uextend(types::I64, v)
        } else {
            v
        }
    }

    /// The inverse: one element word back to the machine type `ty` occupies.
    fn ex_verbo(&mut self, ty: &Ty, verbum: Value) -> Value {
        let ptr = self.ptr();
        let metrum = genus(ptr, ty);
        if metrum.bits() < 64 {
            self.fb.ins().ireduce(metrum, verbum)
        } else {
            verbum
        }
    }

    /// One literal.
    fn constantem(&mut self, c: &Const) -> Result<Value, Defectus> {
        let ptr = self.ptr();
        match c {
            Const::Bool(b) => Ok(self.fb.ins().iconst(types::I8, i64::from(*b))),
            Const::Char(c) => Ok(self.fb.ins().iconst(types::I32, i64::from(u32::from(*c)))),
            Const::Int(r) => self.numerus(KIND_INT, r),
            Const::Frac(g, r) => self.numerus(codex(&Ty::Frac(*g)), r),
            Const::Text(s) => {
                // The length is a byte count, not a character count.
                let (bytes, n) = self.constans(s.as_bytes())?;
                let len = self.fb.ins().iconst(types::I64, n as i64);
                self.voca_unum("liss_text_new", &[ptr, types::I64], ptr, &[bytes, len])
            }
        }
    }

    /// One boxed number, from a static digit blob: `Roman`'s own bytes (spec §5.2).
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

    /// `non` and unary `-`. A negated literal is folded, one level only.
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
                        // The checker refuses `-` on anything else.
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

    /// One binary operator. The dispatch is on `operand_ty`, not the node's own `ty`.
    fn binarium(&mut self, op: BinOp, operand_ty: &Ty, a: Value, b: Value) -> Result<Value, Defectus> {
        let ptr = self.ptr();
        match (op, operand_ty) {
            (BinOp::Add, Ty::Int | Ty::Frac(_)) => {
                self.voca_unum("liss_int_add", &[ptr, ptr], ptr, &[a, b])
            }
            (BinOp::Sub, Ty::Int | Ty::Frac(_)) => {
                self.voca_unum("liss_int_sub", &[ptr, ptr], ptr, &[a, b])
            }
            (BinOp::Mul, Ty::Int) => self.voca_unum("liss_int_mul", &[ptr, ptr], ptr, &[a, b]),
            (BinOp::Mul, Ty::Frac(g)) => {
                let gr = self.fb.ins().iconst(types::I64, granum(*g));
                self.voca_unum("liss_frac_mul", &[ptr, ptr, types::I64], ptr, &[a, b, gr])
            }
            // The divisor is not pre-checked here, and must not be: the runtime
            // raises it, from the one site that carries the sentence. D-24
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
            (BinOp::Add, Ty::Text) => {
                self.voca_unum("liss_text_concat", &[ptr, ptr], ptr, &[a, b])
            }
            // No branch: both operands are already evaluated (spec §3.5).
            (BinOp::And, Ty::Bool) => Ok(self.fb.ins().band(a, b)),
            (BinOp::Or, Ty::Bool) => Ok(self.fb.ins().bor(a, b)),
            (BinOp::Eq | BinOp::Ne | BinOp::Gt | BinOp::Lt, _) => {
                self.comparatio(op, operand_ty, a, b)
            }
            (op, ty) => unimplemented!(
                "the checker rejects `{op:?}` on `{ty:?}`; codegen should never \
                 have been asked",
            ),
        }
    }

    /// The four comparisons. The owned types go through one three-way compare each.
    fn comparatio(&mut self, op: BinOp, operand_ty: &Ty, a: Value, b: Value) -> Result<Value, Defectus> {
        let ptr = self.ptr();
        let ordo = match operand_ty {
            // The machine's unsigned compare *is* the ordering of the unowned scalars.
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
            // Not a three-way compare: `liss_list_eq` returns 1 for equal, hence the
            // inverted codes. Only equality arrives (`errores/series-comparatio-ordinis`).
            Ty::List(_) => {
                let aequa =
                    self.voca_unum("liss_list_eq", &[ptr, ptr], types::I8, &[a, b])?;
                let cc = match op {
                    BinOp::Eq => IntCC::NotEqual,
                    BinOp::Ne => IntCC::Equal,
                    op => unimplemented!(
                        "the checker refuses `{op:?}` on a series; codegen \
                         should never have been asked",
                    ),
                };
                return Ok(self.fb.ins().icmp_imm_s(cc, aequa, 0));
            }
        };
        // `-1`, `0` or `1`, and signed: unsigned would read `minor_est`'s `-1` as 255.
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

    /// The shape of `conformance/currunt/exemplum-salve`, built by hand.
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

            // Two names, one definition: same section, same offset, same size.
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

    #[test]
    fn a_text_constant_is_static_data() {
        let octeti = compila(&salve("Salve, munde!"), &Target::X86_64).unwrap();
        assert!(
            octeti.windows(13).any(|w| w == b"Salve, munde!"),
            "the literal never reached the object",
        );
    }

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
