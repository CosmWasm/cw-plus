use syn::visit::Visit;
use syn::Ident;

pub struct CheckGenerics<'g> {
    generics: &'g [&'g Ident],
    used: Vec<&'g Ident>,
}

impl<'g> CheckGenerics<'g> {
    pub fn new(generics: &'g [&'g Ident]) -> Self {
        Self {
            generics,
            used: vec![],
        }
    }

    pub fn used(self) -> Vec<&'g Ident> {
        self.used
    }

    /// Returns split between used and unused generics
    pub fn used_unused(self) -> (Vec<&'g Ident>, Vec<&'g Ident>) {
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
    fn visit_path(&mut self, p: &'ast syn::Path) {
        if let Some(p) = p.get_ident() {
            if let Some(gen) = self.generics.iter().find(|gen| p == **gen) {
                if !self.used.contains(&gen) {
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
