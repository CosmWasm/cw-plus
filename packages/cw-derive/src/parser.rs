use proc_macro2::Span;
use syn::parse::{Error, Parse, ParseStream};
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

impl Parse for Mapping {
    fn parse(input: ParseStream) -> Result<Self> {
        let index = input.parse()?;
        input.parse::<Token![=]>()?;
        let value = input.parse()?;
        Ok(Mapping { index, value })
    }
}

/// `#[msg(...)]` attribute for `interface` macro
#[derive(PartialEq)]
pub enum InterfaceMsgAttr {
    Exec,
    Query,
}

impl Parse for InterfaceMsgAttr {
    fn parse(input: ParseStream) -> Result<Self> {
        let _content;

        parenthesized!(_content in input);
        let item: Ident = input.parse()?;

        if !input.is_empty() {
            return Err(Error::new(input.span(), "Unexpected token"));
        }

        if item == "exec" {
            Ok(Self::Exec)
        } else if item == "query" {
            Ok(Self::Query)
        } else {
            Err(Error::new(
                item.span(),
                "Invalid message type, expected one of: `exec`, `query`",
            ))
        }
    }
}
