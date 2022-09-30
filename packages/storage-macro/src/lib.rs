/*!
Procedural macros helper for interacting with cw-storage-plus and cosmwasm-storage.

For more information on this package, please check out the
[README](https://github.com/CosmWasm/cw-plus/blob/main/packages/storage-macro/README.md).
*/

use proc_macro::TokenStream;
use syn::{
    Ident,
    __private::{quote::quote, Span},
    parse_macro_input, ItemStruct,
};

#[proc_macro_attribute]
pub fn index_list(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemStruct);

    let ty = Ident::new(&attr.to_string(), Span::call_site());
    let struct_ty = input.ident.clone();

    let names = input
        .fields
        .clone()
        .into_iter()
        .map(|e| {
            let name = e.ident.unwrap();
            quote! { &self.#name }
        })
        .collect::<Vec<_>>();

    let expanded = quote! {
        #input

        impl cw_storage_plus::IndexList<#ty> for #struct_ty<'_> {
            fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn cw_storage_plus::Index<#ty>> + '_> {
                let v: Vec<&dyn cw_storage_plus::Index<#ty>> = vec![#(#names),*];
                Box::new(v.into_iter())
            }
        }
    };

    TokenStream::from(expanded)
}
