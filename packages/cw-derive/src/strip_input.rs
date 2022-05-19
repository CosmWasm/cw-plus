use syn::fold::{self, Fold};
use syn::{ImplItemMethod, ItemImpl, TraitItemMethod};

/// Utility for stripping all attributes from input before it is emitted
pub struct StripInput;

impl Fold for StripInput {
    fn fold_trait_item_method(&mut self, i: TraitItemMethod) -> TraitItemMethod {
        let attrs = i
            .attrs
            .into_iter()
            .filter(|attr| !attr.path.is_ident("msg"));

        fold::fold_trait_item_method(
            self,
            TraitItemMethod {
                attrs: attrs.collect(),
                ..i
            },
        )
    }

    fn fold_impl_item_method(&mut self, i: ImplItemMethod) -> ImplItemMethod {
        let attrs = i
            .attrs
            .into_iter()
            .filter(|attr| !attr.path.is_ident("msg"))
            .collect();

        fold::fold_impl_item_method(self, ImplItemMethod { attrs, ..i })
    }

    fn fold_item_impl(&mut self, i: ItemImpl) -> ItemImpl {
        let attrs = i
            .attrs
            .into_iter()
            .filter(|attr| !attr.path.is_ident("messages"))
            .collect();

        fold::fold_item_impl(self, ItemImpl { attrs, ..i })
    }
}
