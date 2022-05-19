use syn::fold::Fold;

/// Removes all generics from type assumning default values for it
pub struct StripGenerics;

impl Fold for StripGenerics {
    fn fold_path_arguments(&mut self, _: syn::PathArguments) -> syn::PathArguments {
        syn::PathArguments::None
    }
}
