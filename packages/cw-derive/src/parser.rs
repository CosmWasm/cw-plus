use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::parse::{Error, Parse, ParseBuffer, ParseStream};
use syn::punctuated::Punctuated;
use syn::{parenthesized, Ident, Result, Token};

/// Parsed arguments for `interface` macro
pub struct InterfaceArgs {
    /// Module name wrapping generated messages, by default no additional module is created
    pub module: Option<Ident>,
    /// Name of generated exec message enum, `ExecMsg` by default
    pub exec: Ident,
    /// Name of generated query message enum, `QueryMsg` by default
    pub query: Ident,
}

/// Parser arguments for `contract` macro
pub struct ContractArgs {
    /// Module name wrapping generated messages, by default no additional module is created
    pub module: Option<Ident>,
}

struct Mapping {
    index: Ident,
    value: Ident,
}

impl Parse for InterfaceArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut module = None;
        let mut exec = Ident::new("ExecMsg", Span::mixed_site());
        let mut query = Ident::new("QueryMsg", Span::mixed_site());

        let attrs: Punctuated<Mapping, Token![,]> = input.parse_terminated(Mapping::parse)?;

        for attr in attrs {
            if attr.index == "module" {
                module = Some(attr.value);
            } else if attr.index == "exec" {
                exec = attr.value;
            } else if attr.index == "query" {
                query = attr.value;
            } else {
                return Err(Error::new(
                    attr.index.span(),
                    "expected `module`, `exec` or `query`",
                ));
            }
        }

        Ok(InterfaceArgs {
            module,
            exec,
            query,
        })
    }
}

impl Parse for ContractArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut module = None;

        let attrs: Punctuated<Mapping, Token![,]> = input.parse_terminated(Mapping::parse)?;

        for attr in attrs {
            if attr.index == "module" {
                module = Some(attr.value);
            } else {
                return Err(Error::new(
                    attr.index.span(),
                    "expected `module`, `exec` or `query`",
                ));
            }
        }

        Ok(ContractArgs { module })
    }
}

impl Parse for Mapping {
    fn parse(input: ParseStream) -> Result<Self> {
        let index = input.parse()?;
        input.parse::<Token![=]>()?;
        let value = input.parse()?;
        Ok(Mapping { index, value })
    }
}

/// Type of message to be generated
#[derive(PartialEq, Debug, Clone, Copy)]
pub enum MsgType {
    Exec,
    Query,
    Instantiate,
}

/// `#[msg(...)]` attribute for `interface` macro
pub enum MsgAttr {
    Exec,
    Query,
    Instantiate { name: Ident },
}

impl MsgType {
    pub fn emit_ctx_type(self) -> TokenStream {
        use MsgType::*;

        match self {
            Exec | Instantiate => quote! {
                (cosmwasm_std::DepsMut, cosmwasm_std::Env, cosmwasm_std::MessageInfo)
            },
            Query => quote! {
                (cosmwasm_std::Deps, cosmwasm_std::Env)
            },
        }
    }

    /// Emits type which should be returned by dispatch function for this kind of message
    pub fn emit_result_type(self) -> TokenStream {
        use MsgType::*;

        match self {
            Exec | Instantiate => quote! {
                std::result::Result<cosmwasm_std::Response, C::Error>
            },
            Query => quote! {
                std::result::Result<cosmwasm_std::Binary, C::Error>
            },
        }
    }
}

impl Parse for MsgType {
    fn parse(input: ParseStream) -> Result<Self> {
        use MsgType::*;

        let item: Ident = input.parse()?;
        if item == "exec" {
            Ok(Exec)
        } else if item == "query" {
            Ok(Query)
        } else if item == "instantiate" {
            Ok(Instantiate)
        } else {
            Err(Error::new(
                item.span(),
                "Invalid message type, expected one of: `exec`, `query`",
            ))
        }
    }
}

impl PartialEq<MsgType> for MsgAttr {
    fn eq(&self, other: &MsgType) -> bool {
        self.msg_type() == *other
    }
}

impl MsgAttr {
    fn parse_instantiate(content: ParseBuffer) -> Result<Self> {
        let mut name = Ident::new("InstantiateMsg", content.span());
        let p: Option<Token![,]> = content.parse()?;

        if p.is_some() {
            let attrs: Punctuated<Mapping, Token![,]> = content.parse_terminated(Mapping::parse)?;
            for attr in attrs {
                if attr.index == "name" {
                    name = attr.value;
                }
            }
        }

        Ok(Self::Instantiate { name })
    }

    pub fn msg_type(&self) -> MsgType {
        use MsgAttr::*;

        match self {
            Exec => MsgType::Exec,
            Query => MsgType::Query,
            Instantiate { .. } => MsgType::Instantiate,
        }
    }
}

impl Parse for MsgAttr {
    fn parse(input: ParseStream) -> Result<Self> {
        let content;
        parenthesized!(content in input);

        let ty: Ident = content.parse()?;
        if ty == "exec" {
            Ok(Self::Exec)
        } else if ty == "query" {
            Ok(Self::Query)
        } else if ty == "instantiate" {
            Self::parse_instantiate(content)
        } else {
            Err(Error::new(
                ty.span(),
                "Invalid message type, expected one of: `exec`, `query`, `instantiate`",
            ))
        }
    }
}
