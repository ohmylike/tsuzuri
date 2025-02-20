use std::collections::HashMap;

use proc_macro2::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;
use syn::ItemEnum;

pub struct DeriveCommand {
    ident: syn::Ident,
    command_type: CommandType,
}

enum CommandType {
    Unnamed(HashMap<syn::Ident, syn::Path>),
    Other,
}

impl Parse for DeriveCommand {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let item_enum: ItemEnum = input.parse()?;
        let mut commands = HashMap::new();
        let mut is_unnamed = true;
        for variant in item_enum.variants {
            match variant.fields {
                syn::Fields::Named(_) => {
                    is_unnamed = false;
                    break;
                }
                syn::Fields::Unnamed(syn::FieldsUnnamed { unnamed, .. }) => {
                    let span = unnamed.span();
                    let mut iter = unnamed.into_iter();
                    let Some(field) = iter.next() else {
                        return Err(syn::Error::new(span, "command not specified"));
                    };
                    let syn::Type::Path(syn::TypePath { path, .. }) = field.ty else {
                        return Err(syn::Error::new(span, "expected path to command"));
                    };
                    if iter.next().is_some() {
                        return Err(syn::Error::new(span, "only one command can be specified"));
                    }
                    commands.insert(variant.ident, path);
                }
                syn::Fields::Unit => {
                    is_unnamed = false;
                    break;
                }
            }
        }
        let command_type = if is_unnamed {
            CommandType::Unnamed(commands)
        } else {
            CommandType::Other
        };

        Ok(DeriveCommand {
            ident: item_enum.ident,
            command_type,
        })
    }
}

impl DeriveCommand {
    pub fn expand(self) -> TokenStream {
        let handle_impl = self.expand_handle_impl();
        let from_impls = self.expand_from_impls();

        quote! {
            #handle_impl
            #from_impls
        }
    }

    fn expand_handle_impl(&self) -> TokenStream {
        let Self { ident, command_type } = self;

        match command_type {
            CommandType::Unnamed(commands) => {
                let paths: Vec<_> = commands.values().collect();
                let arms = commands.iter().map(|(name, path)| {
                    quote! {
                        #ident::#name(cmd) => {
                            <T as ::tsuzuri::aggregate::Handle<#path>>::handle(&self.0, cmd)
                                .map_err(|err|
                                    ::tsuzuri::__macro_helpers::serde_json::to_value(err)
                                        .unwrap_or_else(|err|
                                            ::tsuzuri::__macro_helpers::serde_json::Value::String(::std::string::ToString::to_string(&err))
                                        )
                                    )
                        }
                    }
                });

                quote! {
                    #[automatically_derived]
                    impl<T> ::tsuzuri::aggregate::Handle<#ident> for ::tsuzuri::aggregate::State<T>
                    where
                        T: ::tsuzuri::aggregate::Aggregate,
                        #( T: ::tsuzuri::aggregate::Handle<#paths>, )*
                        #( <T as ::tsuzuri::aggregate::Handle<#paths>>::Error: ::serde::Serialize, )*
                    {
                        type Error = ::tsuzuri::__macro_helpers::serde_json::Value;

                        fn handle(&self, cmd: #ident) -> ::std::result::Result<::std::vec::Vec<<T as ::tsuzuri::aggregate::Aggregate>::Event>, Self::Error> {
                            match cmd {
                                #( #arms, )*
                            }
                        }
                    }
                }
            }
            CommandType::Other => quote! {
                impl<T> ::tsuzuri::aggregate::Handle<#ident> for ::tsuzuri::aggregate::State<T>
                where
                    T: ::tsuzuri::aggregate::Aggregate + ::tsuzuri::aggregate::Handle<#ident>,
                {
                    type Error = <T as ::tsuzuri::aggregate::Handle<#ident>>::Error;

                    fn handle(&self, cmd: #ident) -> ::std::result::Result<::std::vec::Vec<<Self as ::tsuzuri::aggregate::Aggregate>::Event>, Self::Error> {
                        <T as ::tsuzuri::aggregate::Handle<#ident>>::handle(&self.0, cmd)
                    }
                }
            },
        }
    }

    fn expand_from_impls(&self) -> TokenStream {
        let Self { ident, command_type } = self;

        match command_type {
            CommandType::Unnamed(commands) => {
                let from_impls = commands.iter().map(|(name, path)| {
                    quote! {
                        #[automatically_derived]
                        impl ::std::convert::From<#path> for #ident {
                            fn from(cmd: #path) -> Self {
                                #ident::#name(cmd)
                            }
                        }
                    }
                });

                quote! {
                    #( #from_impls )*
                }
            }
            CommandType::Other => quote! {},
        }
    }
}
