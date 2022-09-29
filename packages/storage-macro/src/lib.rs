use proc_macro::TokenStream;
use syn::{
    Ident,
    __private::{quote::quote, Span},
    parse_macro_input, ItemStruct,
};

/// Auto generate an `IndexList` impl for your indexes struct.
///
/// # Example
///
/// ```rust
/// #[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
/// struct TestStruct {
///     id: u64,
///     id2: u32,
///     addr: Addr,
/// }
///
/// #[index_list(TestStruct)] // <- Add this line right here.
/// struct TestIndexes<'a> {
///     id: MultiIndex<'a, u32, TestStruct, u64>,
///     addr: UniqueIndex<'a, Addr, TestStruct>,
/// }
/// ```
///
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
