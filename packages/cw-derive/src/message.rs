use crate::check_generics::CheckGenerics;
use crate::parser::{MsgAttr, MsgType};
use convert_case::{Case, Casing};
use proc_macro2::{Span, TokenStream};
use proc_macro_error::emit_error;
use quote::quote;
use syn::parse::{Parse, Parser};
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{
    FnArg, GenericParam, Ident, ImplItem, ItemImpl, ItemTrait, Pat, PatType, ReturnType, Signature,
    TraitItem, TraitItemMethod, Type, WhereClause, WherePredicate,
};

fn filter_wheres<'a>(
    clause: &'a Option<WhereClause>,
    generics: &[&GenericParam],
    used_generics: &[&GenericParam],
) -> Vec<&'a WherePredicate> {
    clause
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
        .unwrap_or_default()
}

fn process_fields<'s>(
    sig: &'s Signature,
    generics_checker: &mut CheckGenerics,
) -> Vec<MsgField<'s>> {
    sig.inputs
        .iter()
        .skip(2)
        .filter_map(|arg| match arg {
            FnArg::Receiver(item) => {
                emit_error!(item.span(), "Unexpected `self` argument");
                None
            }

            FnArg::Typed(item) => MsgField::new(item, generics_checker),
        })
        .collect()
}

/// Representation of single struct message
pub struct StructMessage<'a> {
    contract_type: &'a Type,
    fields: Vec<MsgField<'a>>,
    function_name: &'a Ident,
    generics: Vec<&'a GenericParam>,
    unused_generics: Vec<&'a GenericParam>,
    wheres: Vec<&'a WherePredicate>,
    full_where: Option<&'a WhereClause>,
    result: &'a ReturnType,
    msg_attr: MsgAttr,
}

impl<'a> StructMessage<'a> {
    /// Creates new struct message of given type from impl block
    pub fn new(
        source: &'a ItemImpl,
        ty: MsgType,
        generics: &'a [&'a GenericParam],
    ) -> Option<StructMessage<'a>> {
        let mut generics_checker = CheckGenerics::new(generics);

        let contract_type = &source.self_ty;
        let mut methods = source.items.iter().filter_map(|item| match item {
            ImplItem::Method(method) => {
                let msg_attr = method.attrs.iter().find(|attr| attr.path.is_ident("msg"))?;
                let attr = match MsgAttr::parse.parse2(msg_attr.tokens.clone()) {
                    Ok(attr) => attr,
                    Err(err) => {
                        emit_error!(method.span(), err);
                        return None;
                    }
                };

                if attr == ty {
                    Some((method, attr))
                } else {
                    None
                }
            }
            _ => None,
        });

        let (method, msg_attr) = if let Some(method) = methods.next() {
            method
        } else {
            emit_error!(source.span(), "No instantiation message");
            return None;
        };

        if let Some((obsolete, _)) = methods.next() {
            emit_error!(
                obsolete.span(), "More than one instantiation message";
                note = method.span() => "Instantiation message previously defied here"
            );
        }

        let function_name = &method.sig.ident;
        let fields = process_fields(&method.sig, &mut generics_checker);
        let (used_generics, unused_generics) = generics_checker.used_unused();
        let wheres = filter_wheres(&source.generics.where_clause, generics, &used_generics);

        Some(Self {
            contract_type,
            fields,
            function_name,
            generics: used_generics,
            unused_generics,
            wheres,
            full_where: source.generics.where_clause.as_ref(),
            result: &method.sig.output,
            msg_attr,
        })
    }

    pub fn emit(&self) -> TokenStream {
        use MsgAttr::*;

        match &self.msg_attr {
            Instantiate { name } => self.emit_struct(name),
            _ => {
                emit_error!(Span::mixed_site(), "Invalid message type");
                quote! {}
            }
        }
    }

    pub fn emit_struct(&self, name: &Ident) -> TokenStream {
        let Self {
            contract_type,
            fields,
            function_name,
            generics,
            unused_generics,
            wheres,
            full_where,
            result,
            msg_attr,
        } = self;

        let where_clause = if !wheres.is_empty() {
            quote! {
                where #(#wheres,)*
            }
        } else {
            quote! {}
        };

        let ctx_type = msg_attr.msg_type().emit_ctx_type();
        let fields_names: Vec<_> = fields.iter().map(MsgField::name).collect();
        let fields = fields.iter().map(MsgField::emit);

        let generics = if generics.is_empty() {
            quote! {}
        } else {
            quote! {
                <#(#generics,)*>
            }
        };

        let unused_generics = if unused_generics.is_empty() {
            quote! {}
        } else {
            quote! {
                <#(#unused_generics,)*>
            }
        };

        quote! {
            #[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, schemars::JsonSchema)]
            #[serde(rename_all="snake_case")]
            pub struct #name #generics #where_clause {
                #(#fields,)*
            }

            impl #generics #name #generics #where_clause {
                pub fn dispatch #unused_generics(self, contract: #contract_type, ctx: #ctx_type)
                    #result #full_where
                {
                    let Self { #(#fields_names,)* } = self;
                    contract.#function_name(ctx.into(), #(#fields_names,)*).map_err(Into::into)
                }
            }
        }
    }
}

/// Representation of single enum message
pub struct EnumMessage<'a> {
    name: &'a Ident,
    trait_name: &'a Ident,
    variants: Vec<MsgVariant<'a>>,
    generics: Vec<&'a GenericParam>,
    unused_generics: Vec<&'a GenericParam>,
    all_generics: &'a [&'a GenericParam],
    wheres: Vec<&'a WherePredicate>,
    full_where: Option<&'a WhereClause>,
    msg_ty: MsgType,
}

impl<'a> EnumMessage<'a> {
    pub fn new(
        name: &'a Ident,
        source: &'a ItemTrait,
        ty: MsgType,
        generics: &'a [&'a GenericParam],
    ) -> Self {
        let trait_name = &source.ident;

        let mut generics_checker = CheckGenerics::new(generics);
        let variants: Vec<_> = source
            .items
            .iter()
            .filter_map(|item| match item {
                TraitItem::Method(method) => {
                    let msg_attr = method.attrs.iter().find(|attr| attr.path.is_ident("msg"))?;
                    let attr = match MsgAttr::parse.parse2(msg_attr.tokens.clone()) {
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

        let (used_generics, unused_generics) = generics_checker.used_unused();
        let wheres = filter_wheres(&source.generics.where_clause, generics, &used_generics);

        Self {
            name,
            trait_name,
            variants,
            generics: used_generics,
            unused_generics,
            all_generics: generics,
            wheres,
            full_where: source.generics.where_clause.as_ref(),
            msg_ty: ty,
        }
    }

    pub fn emit(&self) -> TokenStream {
        let Self {
            name,
            trait_name,
            variants,
            generics,
            unused_generics,
            all_generics,
            wheres,
            full_where,
            msg_ty,
        } = self;

        let match_arms = variants
            .iter()
            .map(|variant| variant.emit_dispatch_leg(*msg_ty));
        let variants = variants.iter().map(MsgVariant::emit);
        let where_clause = if !wheres.is_empty() {
            quote! {
                where #(#wheres,)*
            }
        } else {
            quote! {}
        };

        let ctx_type = msg_ty.emit_ctx_type();
        let dispatch_type = msg_ty.emit_result_type();

        let all_generics = if all_generics.is_empty() {
            quote! {}
        } else {
            quote! { <#(#all_generics,)*> }
        };

        let generics = if generics.is_empty() {
            quote! {}
        } else {
            quote! { <#(#generics,)*> }
        };

        quote! {
            #[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, schemars::JsonSchema)]
            #[serde(rename_all="snake_case")]
            pub enum #name #generics #where_clause {
                #(#variants,)*
            }

            impl #generics #name #generics #where_clause {
                pub fn dispatch<C: #trait_name #all_generics, #(#unused_generics,)*>(self, contract: &C, ctx: #ctx_type)
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
        let fields = process_fields(&method.sig, generics_checker);

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
    pub fn emit_dispatch_leg(&self, msg_attr: MsgType) -> TokenStream {
        use MsgType::*;

        let Self {
            name,
            fields,
            function_name,
        } = self;
        let args = fields.iter().map(|field| field.name);
        let fields = fields.iter().map(|field| field.name);

        match msg_attr {
            Exec => quote! {
                #name {
                    #(#fields,)*
                } => contract.#function_name(ctx.into(), #(#args),*).map_err(Into::into)
            },
            Query => quote! {
                #name {
                    #(#fields,)*
                } => cosmwasm_std::to_binary(&contract.#function_name(ctx.into(), #(#args),*)?).map_err(Into::into)
            },
            Instantiate => {
                emit_error!(name.span(), "Instantiation messages not supported on traits, they should be defined on contracts directly");
                quote! {}
            }
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

    pub fn name(&self) -> &'a Ident {
        self.name
    }
}
