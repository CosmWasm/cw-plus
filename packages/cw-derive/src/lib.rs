use input::TraitInput;
use proc_macro::TokenStream;
use proc_macro_error::proc_macro_error;
use quote::quote;
use syn::fold::Fold;
use syn::{parse_macro_input, ItemTrait};

pub(crate) mod check_generics;
mod input;
mod message;
mod parser;
mod strip_input;

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
#[proc_macro_error]
#[proc_macro_attribute]
pub fn interface(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attrs = parse_macro_input!(attr as parser::InterfaceArgs);
    let input = parse_macro_input!(item as ItemTrait);

    let expanded = TraitInput::new(&attrs, &input).process();
    let input = StripInput.fold_item_trait(input);

    let expanded = quote! {
        #input

        #expanded
    };

    TokenStream::from(expanded)
}
