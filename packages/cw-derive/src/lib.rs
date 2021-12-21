use convert_case::{Case, Casing};
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::fold::Fold;
use syn::parse::{Parse, Parser};
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{
    parse_macro_input, Error, FnArg, GenericParam, Ident, ItemTrait, Pat, PatType, TraitItem,
    TraitItemMethod,
};

mod check_generics;
mod parser;
mod strip_input;

use check_generics::CheckGenerics;
use strip_input::StripInput;

/// Macro generating messages from contract trait.
///
/// ## Example usage
/// ```ignore
/// # use cosmwasm_std::Response;
///
/// # struct Ctx;
/// # struct Error;
///
/// # #[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, schemars::JsonSchema)]
/// # struct Member;
///
/// #[cw_derive::interface(module=msg, exec=Execute, query=Query)]
/// trait Cw4 {
///     #[msg(exec)]
///     fn update_admin(&self, ctx: Ctx, admin: Option<String>) -> Result<Response, Error>;
///
///     #[msg(exec)]
///     fn update_members(&self, ctx: Ctx, remove: Vec<String>, add: Vec<Member>)
///         -> Result<Response, Error>;
///
///     #[msg(query)]
///     fn admin(&self, ctx: Ctx) -> Result<Response, Error>;
///
///     #[msg(query)]
///     fn member(&self, ctx: Ctx, addr: String, at_height: Option<u64>) -> Result<Response, Error>;
/// }
/// ```
///
/// This would generate output like:
///
/// ```ignore
/// pub mod msg {
///     # #[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, schemars::JsonSchema)]
///     # struct Member;
///
///     #[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, schemars::JsonSchema)]
///     #[serde(rename_all = "snake_case")]
///     pub enum Execute {
///         UpdateAdmin { admin: Option<String> },
///         UpdateMembers {
///             remove: Vec<String>,
///             add: Vec<Member>,
///         },
///         AddHook { addr: String },
///         RemoveHook { addr: String },
///     }
/// }
/// ```
///
/// ## Parameters
///
/// `interface` attribute takes optional parameters:
/// * `module` - defines module name, where all generated messages would be encapsulated; no
/// additional module would be created if not provided
/// * `exec` - sets name for execution messages type, `ExecMsg` by default
/// * `query` - sets name for query messages type, `QueryMsg` by default
///
/// ## Attributes
///
/// Messages structures are generated basing on interface trait method. Some hints for generator
/// may be provided by additional attributes.
///
/// * `msg(msg_type)` - Hints, that this function is a message variant of specific type. Methods
/// which are not marked with this attribute are ignored by generator. `msg_type` is one of:
///   * `exec` - this is execute message variant
///   * `query` - this is query message variant
#[proc_macro_attribute]
pub fn interface(attr: TokenStream, item: TokenStream) -> TokenStream {
    let item = item.clone();
    let attrs = parse_macro_input!(attr as parser::InterfaceArgs);
    let input = parse_macro_input!(item as ItemTrait);

    let generics = trait_generics(&input);

    let exec = build_msg(
        &attrs.exec,
        &input,
        parser::InterfaceMsgAttr::Exec,
        &generics,
    );
    let query = build_msg(
        &attrs.query,
        &input,
        parser::InterfaceMsgAttr::Query,
        &generics,
    );

    let input = StripInput.fold_item_trait(input);

    let expanded = if let Some(module) = attrs.module {
        quote! {
            pub mod #module {
                use super::*;
                #exec

                #query
            }

            #input
        }
    } else {
        quote! {
            #exec

            #query

            #input
        }
    };

    TokenStream::from(expanded)
}

fn trait_generics(source: &ItemTrait) -> Vec<Ident> {
    source
        .generics
        .params
        .iter()
        .filter_map(|gp| match gp {
            GenericParam::Type(tp) => Some(tp.ident.clone()),
            _ => None,
        })
        .collect()
}

/// Builds message basing on input trait
fn build_msg(
    name: &Ident,
    source: &ItemTrait,
    ty: parser::InterfaceMsgAttr,
    generics: &[Ident],
) -> TokenStream2 {
    let mut generics_checker = CheckGenerics::new(generics);

    let variants: Vec<_> = source
        .items
        .iter()
        .filter_map(|item| match item {
            TraitItem::Method(method) => {
                let msg_attr = method.attrs.iter().find(|attr| attr.path.is_ident("msg"))?;
                let attr = match parser::InterfaceMsgAttr::parse.parse2(msg_attr.tokens.clone()) {
                    Ok(attr) => attr,
                    Err(err) => return Some(msg_variant_err(&method, err)),
                };

                if attr == ty {
                    let variant = msg_variant(&method, &mut generics_checker);
                    Some(variant)
                } else {
                    None
                }
            }
            _ => None,
        })
        .collect();

    let generics = generics_checker.used();
    quote! {
        #[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, schemars::JsonSchema)]
        #[serde(rename_all="snake_case")]
        pub enum #name <#(#generics,)*> where #(#generics: Clone + std::fmt::Debug + PartialEq + schemars::JsonSchema,)*{
            #(#variants,)*
        }
    }
}

/// Builds message variant error
fn msg_variant_err(method: &TraitItemMethod, err: Error) -> TokenStream2 {
    let variant = &method.sig.ident;
    let name = variant.to_string().to_case(Case::Camel);
    let variant = Ident::new(&name, variant.span());
    let err = err.into_compile_error();

    quote! {
        #variant(#err)
    }
}

/// Builds single message variant from method definition
fn msg_variant(method: &TraitItemMethod, generics_checker: &mut CheckGenerics) -> TokenStream2 {
    let name = &method.sig.ident;
    let name = Ident::new(&name.to_string().to_case(Case::UpperCamel), name.span());

    let fields = method
        .sig
        .inputs
        .iter()
        .skip(2)
        .enumerate()
        .map(|(idx, arg)| match arg {
            FnArg::Receiver(item) => {
                let err =
                    Error::new(item.span(), "Unexpected `self` argument").into_compile_error();

                quote! {
                    _self: #err
                }
            }

            FnArg::Typed(item) => msg_field(item, idx, generics_checker),
        });

    let fields: Vec<_> = fields.collect();

    let variant = quote! {
        #name {
            #(#fields,)*
        }
    };

    variant
}

fn msg_field<'g>(item: &PatType, idx: usize, generics_checker: &mut CheckGenerics) -> TokenStream2 {
    let name = match &*item.pat {
        Pat::Ident(p) => &p.ident,
        pat => {
            // TODO: Support pattern arguments, when decorated with argument with item
            // name
            //
            // Eg.
            //
            // ```
            // fn exec_foo(&self, ctx: Ctx, #[msg(name=metadata)] SomeData { addr, sender }: SomeData);
            // ```
            //
            // should expand to enum variant:
            //
            // ```
            // ExecFoo {
            //   metadata: SomeDaa
            // }
            // ```
            let err = Error::new(pat.span(), "Expected argument name, pattern occurred")
                .into_compile_error();
            let name = format!("_invalid_{}", idx);
            return quote! {
                #name: #err
            };
        }
    };

    let name = Ident::new(&name.to_string().to_case(Case::Snake), name.span());
    let ty = &item.ty;

    generics_checker.visit_type(ty);

    quote! {
        #name: #ty
    }
}
