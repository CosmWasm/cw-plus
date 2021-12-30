use proc_macro2::TokenStream;
use proc_macro_error::emit_error;
use quote::quote;
use syn::{GenericParam, Ident, ItemTrait, TraitItem};

use crate::message::Message;
use crate::parser::{InterfaceArgs, InterfaceMsgAttr};

/// Preprocessed macro input
pub struct TraitInput<'a> {
    attributes: &'a InterfaceArgs,
    item: &'a ItemTrait,
    generics: Vec<&'a Ident>,
}

impl<'a> TraitInput<'a> {
    pub fn new(attributes: &'a InterfaceArgs, item: &'a ItemTrait) -> Self {
        let generics = item
            .generics
            .params
            .iter()
            .filter_map(|gp| match gp {
                GenericParam::Type(tp) => Some(&tp.ident),
                _ => None,
            })
            .collect();

        if item
            .items
            .iter()
            .find(|item| match item {
                TraitItem::Type(ty) if ty.ident == Ident::new("Error", ty.ident.span()) => true,
                _ => false,
            })
            .is_none()
        {
            emit_error!(
                item.ident.span(), "Missing `Error` type defined for trait.";
                note = "Error is an error type returned by generated types dispatch function. Messages handling function have to return an error type convertible to this Error type.";
                note = "A trait error type should be bound to implement `From<cosmwasm_std::StdError>`.";
            );
        }

        Self {
            attributes,
            item,
            generics,
        }
    }

    pub fn process(&self) -> TokenStream {
        let messages = self.emit_messages();

        if let Some(module) = &self.attributes.module {
            quote! {
                pub mod #module {
                    use super::*;

                    #messages
                }
            }
        } else {
            messages
        }
    }

    fn emit_messages(&self) -> TokenStream {
        let exec = self.emit_msg(&self.attributes.exec, InterfaceMsgAttr::Exec);
        let query = self.emit_msg(&self.attributes.query, InterfaceMsgAttr::Query);

        quote! {
            #exec

            #query
        }
    }

    fn emit_msg(&self, name: &Ident, msg_attr: InterfaceMsgAttr) -> TokenStream {
        Message::new(name, &self.item, msg_attr, &self.generics).emit()
    }
}
