use syn::visit::Visit;
use syn::GenericParam;

pub struct CheckGenerics<'g> {
    generics: &'g [&'g GenericParam],
    used: Vec<&'g GenericParam>,
}

impl<'g> CheckGenerics<'g> {
    pub fn new(generics: &'g [&'g GenericParam]) -> Self {
        Self {
            generics,
            used: vec![],
        }
    }

    pub fn used(self) -> Vec<&'g GenericParam> {
        self.used
    }

    /// Returns split between used and unused generics
    pub fn used_unused(self) -> (Vec<&'g GenericParam>, Vec<&'g GenericParam>) {
        let unused = self
            .generics
            .iter()
            .filter(|gen| !self.used.contains(*gen))
            .copied()
            .collect();

        (self.used, unused)
    }
}

impl<'ast, 'g> Visit<'ast> for CheckGenerics<'g> {
    fn visit_lifetime(&mut self, i: &'ast syn::Lifetime) {
        if let Some(gen) = self
            .generics
            .iter()
            .find(|gen| matches!(gen, GenericParam::Lifetime(lt) if lt.lifetime == *i))
        {
            if !self.used.contains(gen) {
                self.used.push(gen);
            }
        }
    }
    fn visit_path(&mut self, p: &'ast syn::Path) {
        if let Some(p) = p.get_ident() {
            if let Some(gen) = self
                .generics
                .iter()
                .find(|gen| matches!(gen, GenericParam::Type(ty) if ty.ident == *p))
            {
                if !self.used.contains(gen) {
                    self.used.push(gen);
                }
            }
        }

        // Default visit implementation - visiting path deeper
        for el in &p.segments {
            self.visit_path_segment(el);
        }
    }
}
