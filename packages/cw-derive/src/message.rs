use crate::check_generics::CheckGenerics;
use crate::parser::InterfaceMsgAttr;
use convert_case::{Case, Casing};
use proc_macro2::TokenStream;
use proc_macro_error::emit_error;
use quote::quote;
use syn::parse::{Parse, Parser};
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{
    FnArg, Ident, ItemTrait, Pat, PatType, TraitItem, TraitItemMethod, Type, WhereClause,
    WherePredicate,
};

/// Representation of single message
pub struct Message<'a> {
    pub name: &'a Ident,
    pub trait_name: &'a Ident,
    pub variants: Vec<MsgVariant<'a>>,
    pub generics: Vec<&'a Ident>,
    pub unsused_generics: Vec<&'a Ident>,
    pub all_generics: &'a [&'a Ident],
    pub wheres: Vec<&'a WherePredicate>,
    pub full_where: Option<&'a WhereClause>,
    pub msg_attr: InterfaceMsgAttr,
}

impl<'a> Message<'a> {
    pub fn new(
        name: &'a Ident,
        source: &'a ItemTrait,
        ty: InterfaceMsgAttr,
        generics: &'a [&'a Ident],
    ) -> Self {
        let trait_name = &source.ident;

        let mut generics_checker = CheckGenerics::new(generics);
        let variants: Vec<_> = source
            .items
            .iter()
            .filter_map(|item| match item {
                TraitItem::Method(method) => {
                    let msg_attr = method.attrs.iter().find(|attr| attr.path.is_ident("msg"))?;
                    let attr = match InterfaceMsgAttr::parse.parse2(msg_attr.tokens.clone()) {
                        Ok(attr) => attr,
                        Err(err) => {
                            emit_error!(method.span(), err);
                            return None;
                        }
                    };

                    if attr == ty {
                        Some(MsgVariant::new(&method, &mut generics_checker))
                    } else {
                        None
                    }
                }
                _ => None,
            })
            .collect();

        let (used_generics, unsused_generics) = generics_checker.used_unused();
        let wheres = source
            .generics
            .where_clause
            .as_ref()
            .map(|clause| {
                clause
                    .predicates
                    .iter()
                    .filter(|pred| {
                        let mut generics_checker = CheckGenerics::new(generics);
                        generics_checker.visit_where_predicate(pred);
                        generics_checker
                            .used()
                            .into_iter()
                            .all(|gen| used_generics.contains(&gen))
                    })
                    .collect()
            })
            .unwrap_or_default();

        Self {
            name,
            trait_name,
            variants,
            generics: used_generics,
            unsused_generics,
            all_generics: generics,
            wheres,
            full_where: source.generics.where_clause.as_ref(),
            msg_attr: ty,
        }
    }

    pub fn emit(&self) -> TokenStream {
        self.emit_enum()
    }

    fn emit_enum(&self) -> TokenStream {
        let Self {
            name,
            trait_name,
            variants,
            generics,
            unsused_generics,
            all_generics,
            wheres,
            full_where,
            msg_attr,
        } = self;

        let match_arms = variants
            .iter()
            .map(|variant| variant.emit_dispatch_leg(*msg_attr));
        let variants = variants.iter().map(MsgVariant::emit);
        let where_clause = if !wheres.is_empty() {
            quote! {
                where #(#wheres,)*
            }
        } else {
            quote! {}
        };

        let ctx_type = msg_attr.emit_ctx_type();
        let dispatch_type = msg_attr.emit_result_type();

        quote! {
            #[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, schemars::JsonSchema)]
            #[serde(rename_all="snake_case")]
            pub enum #name <#(#generics,)*> #where_clause {
                #(#variants,)*
            }

            impl<#(#generics,)*> #name<#(#generics,)*> #where_clause {
                pub fn dispatch<C: #trait_name<#(#all_generics,)*>, #(#unsused_generics,)*>(self, contract: &C, ctx: #ctx_type)
                    -> #dispatch_type #full_where
                {
                    use #name::*;

                    match self {
                        #(#match_arms,)*
                    }
                }
            }
        }
    }
}

/// Representation of whole message variant
pub struct MsgVariant<'a> {
    name: Ident,
    function_name: &'a Ident,
    // With https://github.com/rust-lang/rust/issues/63063 this could be just an iterator over
    // `MsgField<'a>`
    fields: Vec<MsgField<'a>>,
}

impl<'a> MsgVariant<'a> {
    /// Creates new message variant from trait method
    pub fn new(
        method: &'a TraitItemMethod,
        generics_checker: &mut CheckGenerics,
    ) -> MsgVariant<'a> {
        let function_name = &method.sig.ident;
        let name = Ident::new(
            &function_name.to_string().to_case(Case::UpperCamel),
            function_name.span(),
        );

        let fields = method
            .sig
            .inputs
            .iter()
            .skip(2)
            .filter_map(|arg| match arg {
                FnArg::Receiver(item) => {
                    emit_error!(item.span(), "Unexpected `self` argument");
                    None
                }

                FnArg::Typed(item) => MsgField::new(item, generics_checker),
            })
            .collect();

        Self {
            name,
            function_name,
            fields,
        }
    }

    /// Emits message variant
    pub fn emit(&self) -> TokenStream {
        let Self { name, fields, .. } = self;
        let fields = fields.iter().map(MsgField::emit);

        quote! {
            #name {
                #(#fields,)*
            }
        }
    }

    /// Emits match leg dispatching against this variant. Assumes enum variants are imported into the
    /// scope. Dispatching is performed by calling the function this variant is build from on the
    /// `contract` variable, with `ctx` as its first argument - both of them should be in scope.
    pub fn emit_dispatch_leg(&self, msg_attr: InterfaceMsgAttr) -> TokenStream {
        let Self {
            name,
            fields,
            function_name,
        } = self;
        let args = fields.iter().map(|field| field.name);
        let fields = fields.iter().map(|field| field.name);

        match msg_attr {
            InterfaceMsgAttr::Exec => quote! {
                #name {
                    #(#fields,)*
                } => contract.#function_name(ctx.into(), #(#args),*).map_err(Into::into)
            },
            InterfaceMsgAttr::Query => quote! {
                #name {
                    #(#fields,)*
                } => cosmwasm_std::to_binary(&contract.#function_name(ctx.into(), #(#args),*)?).map_err(Into::into)
            },
        }
    }
}

/// Representation of single message variant field
pub struct MsgField<'a> {
    name: &'a Ident,
    ty: &'a Type,
}

impl<'a> MsgField<'a> {
    /// Creates new field from trait method argument
    pub fn new(item: &'a PatType, generics_checker: &mut CheckGenerics) -> Option<MsgField<'a>> {
        let name = match &*item.pat {
            Pat::Ident(p) => Some(&p.ident),
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
                emit_error!(pat.span(), "Expected argument name, pattern occurred");
                None
            }
        }?;

        let ty = &item.ty;
        generics_checker.visit_type(ty);

        Some(Self { name, ty })
    }

    /// Emits message field
    pub fn emit(&self) -> TokenStream {
        let Self { name, ty } = self;

        quote! {
            #name: #ty
        }
    }
}
