use syn::fold::Fold;
use syn::TraitItemMethod;

/// Utility for stripping all attributes from input before it is emitted
pub struct StripInput;

impl Fold for StripInput {
    fn fold_trait_item_method(&mut self, i: TraitItemMethod) -> TraitItemMethod {
        let attrs = i
            .attrs
            .into_iter()
            .filter(|attr| !attr.path.is_ident("msg"));

        TraitItemMethod {
            attrs: attrs.collect(),
            ..i
        }
    }
}
