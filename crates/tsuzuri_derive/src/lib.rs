use command::DeriveCommand;
use event::DeriveEvent;

mod command;
mod event;

extern crate proc_macro2;
extern crate quote;
extern crate syn;

/// Used to implement traits for an aggregate command enum.
#[proc_macro_derive(Command)]
pub fn command(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    syn::parse_macro_input!(input as DeriveCommand).expand().into()
}

/// Used to implement traits for an aggregate event enum.
#[proc_macro_derive(Event)]
pub fn event(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    syn::parse_macro_input!(input as DeriveEvent).expand().into()
}
