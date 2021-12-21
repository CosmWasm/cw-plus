use syn::visit::Visit;
use syn::Ident;

pub struct CheckGenerics<'g> {
    generics: &'g [Ident],
    used: Vec<&'g Ident>,
}

impl<'g> CheckGenerics<'g> {
    pub fn new(generics: &'g [Ident]) -> Self {
        Self {
            generics,
            used: vec![],
        }
    }

    pub fn used(self) -> Vec<&'g Ident> {
        self.used
    }
}

impl<'ast, 'g> Visit<'ast> for CheckGenerics<'g> {
    fn visit_path(&mut self, p: &'ast syn::Path) {
        if let Some(p) = p.get_ident() {
            if let Some(gen) = self.generics.iter().find(|gen| p == *gen) {
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
