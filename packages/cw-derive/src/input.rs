use proc_macro2::TokenStream;
use quote::quote;
use syn::{GenericParam, Ident, ItemTrait};

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
