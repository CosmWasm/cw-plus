use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::parse::{Error, Nothing, Parse, ParseBuffer, ParseStream};
use syn::punctuated::Punctuated;
use syn::{parenthesized, parse2, Ident, Result, Token, Type};

/// Parsed arguments for `interface` macro
pub struct InterfaceArgs {
    /// Module name wrapping generated messages, by default no additional module is created
    pub module: Option<Ident>,
    /// Name of generated exec message enum, `ExecMsg` by default
    pub exec: Ident,
    /// Name of generated query message enum, `QueryMsg` by default
    pub query: Ident,
    /// The type being a parameter of `CosmosMsg` for blockchain it is intendet to be used; can be
    /// set to any of generic parameters to create interface being generic over blockchains; If not
    /// provided, cosmos messages would be unparametrized (so default would be used)
    pub msg_type: Option<Type>,
}

/// Parser arguments for `contract` macro
pub struct ContractArgs {
    /// Module name wrapping generated messages, by default no additional module is created
    pub module: Option<Ident>,
}

struct Mapping {
    index: Ident,
    value: TokenStream,
}

impl Parse for InterfaceArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut module = None;
        let mut exec = Ident::new("ExecMsg", Span::mixed_site());
        let mut query = Ident::new("QueryMsg", Span::mixed_site());
        let mut msg_type = None;

        while !input.is_empty() {
            let attr: Ident = input.parse()?;
            let _: Token![=] = input.parse()?;

            if attr == "module" {
                module = Some(input.parse()?);
            } else if attr == "exec" {
                exec = input.parse()?;
            } else if attr == "query" {
                query = input.parse()?;
            } else if attr == "msg_type" {
                msg_type = Some(input.parse()?);
            } else {
                return Err(Error::new(
                    attr.span(),
                    "expected `module`, `exec`, `query`, or `msg_type`",
                ));
            }

            if input.peek(Token![,]) {
                let _: Token![,] = input.parse()?;
            } else if !input.is_empty() {
                return Err(input.error("Unexpected token, comma expected"));
            }
        }

        let _: Nothing = input.parse()?;
        let attrs: Punctuated<Mapping, Token![,]> = input.parse_terminated(Mapping::parse)?;

        for attr in attrs {
            if attr.index == "module" {
                module = Some(parse2(attr.value)?);
            } else if attr.index == "exec" {
                exec = parse2(attr.value)?;
            } else if attr.index == "query" {
                query = parse2(attr.value)?;
            } else if attr.index == "msg_type" {
                msg_type = Some(parse2(attr.value)?);
            } else {
                return Err(Error::new(
                    attr.index.span(),
                    "expected `module`, `exec`, `query`, or `msg_type`",
                ));
            }
        }

        Ok(InterfaceArgs {
            module,
            exec,
            query,
            msg_type,
        })
    }
}

impl Parse for ContractArgs {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut module = None;

        let attrs: Punctuated<Mapping, Token![,]> = input.parse_terminated(Mapping::parse)?;

        for attr in attrs {
            if attr.index == "module" {
                module = Some(parse2(attr.value)?);
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
    pub fn emit_result_type(self, msg_type: &Option<Type>) -> TokenStream {
        use MsgType::*;

        match (self, msg_type) {
            (Exec, Some(msg_type)) | (Instantiate, Some(msg_type)) => quote! {
                std::result::Result<cosmwasm_std::Response<#msg_type>, C::Error>
            },
            (Exec, None) | (Instantiate, None) => quote! {
                std::result::Result<cosmwasm_std::Response, C::Error>
            },

            (Query, _) => quote! {
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
                    name = parse2(attr.value)?;
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
