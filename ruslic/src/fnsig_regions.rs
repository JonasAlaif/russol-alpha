use rustc_ast::Mutability;
use rustc_middle::ty::{TyKind, Region, Ty, GenericArgKind, TyCtxt};
use rustc_data_structures::fx::FxHashSet;

pub fn collect_blocking_lfts<'tcx>(args: Vec<Ty<'tcx>>, ret: Ty<'tcx>, tcx: TyCtxt<'tcx>) -> (FxHashSet<Region<'tcx>>, FxHashSet<Region<'tcx>>) {
    let args_mutrefs = args.iter().flat_map(|ty| TypeMutRefWalker::new(*ty, tcx).into_iter());
    let (sources, tys): (FxHashSet<_>, Vec<_>) = args_mutrefs.unzip();
    let dests = ret.walk().chain(tys.into_iter().flat_map(Ty::walk)).filter_map(|ga|
        match ga.unpack() {
            GenericArgKind::Lifetime(r) => Some(r),
            GenericArgKind::Type(_) => None,
            GenericArgKind::Const(_) => None,
        }
    ).collect();
    (sources, dests)
}

struct TypeMutRefWalker<'tcx> {
    stack: Vec<(&'tcx TyKind<'tcx>, usize, usize)>,
    visited: FxHashSet<Ty<'tcx>>,
    tcx: TyCtxt<'tcx>,
}

impl<'tcx> TypeMutRefWalker<'tcx> {
    pub fn new(ty: Ty<'tcx>, tcx: TyCtxt<'tcx>) -> Self {
        let stack = vec![(ty.kind(), 0, 0)];
        Self { stack, visited: FxHashSet::default(), tcx }
    }
    fn push_ty(&mut self, ty: Ty<'tcx>) {
        if !self.visited.contains(&ty) {
            self.visited.insert(ty);
            self.stack.push((ty.kind(), 0, 0));
        }
    }
}
impl<'tcx> Iterator for TypeMutRefWalker<'tcx> {
    type Item = (Region<'tcx>, Ty<'tcx>);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.stack.pop()? {
                (TyKind::Bool, _, _) |
                (TyKind::Char, _, _) |
                (TyKind::Int(_), _, _) |
                (TyKind::Uint(_), _, _) |
                (TyKind::Float(_), _, _) |
                (TyKind::Param(_), _, _) |
                // Cannot go through immut ref without loosing mutability (itself or nothing inside can be blocked or blocking)
                (TyKind::Ref(_, _, Mutability::Not), _, _) |
                // Cannot go through raw pointer without unsafe
                (TyKind::RawPtr(_), _, _) => (),
                (kind@TyKind::Tuple(tys), idx, _) => if idx < tys.len() {
                    self.stack.push((kind, idx+1, 0));
                    self.push_ty(tys[idx]);
                },
                (kind@TyKind::Adt(adt, substs), vidx, fidx) => {
                    if adt.is_phantom_data() {
                        // Special rule: since PhantomData<T> owns T
                        self.push_ty(substs.type_at(0));
                        continue;
                    }
                    if let Some(v) = adt.variants().iter().skip(vidx).next() {
                        if fidx < v.fields.len() {
                            self.stack.push((kind, vidx, fidx+1));
                            self.push_ty(v.fields[fidx].ty(self.tcx, substs));
                        } else {
                            self.stack.push((kind, vidx+1, 0));
                        }
                    }
                }
                (TyKind::Ref(r, ty, Mutability::Mut), _, _) => return Some((*r, *ty)),
                (kind, _, _) => todo!("Unsupported ty {:?}", kind),
            }
        }
    }
}
