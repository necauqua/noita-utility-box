use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{quote, quote_spanned};
use syn::{Data, Ident, spanned::Spanned};

/// Destroys an item it's attached to.
///
/// Used by the derive macro to trick other derives into running from it.
#[doc(hidden)]
#[proc_macro_attribute]
pub fn __derive_stub(_attr: TokenStream, _item: TokenStream) -> TokenStream {
    <_>::default()
}

#[proc_macro_derive(PtrReadable)]
pub fn derive_macro(item: TokenStream) -> TokenStream {
    let derive_input = syn::parse_macro_input!(item as syn::DeriveInput);
    let Data::Struct(_) = derive_input.data else {
        return quote_spanned!(derive_input.span() => compile_error!("Only structs are supported"))
            .into();
    };
    let ident = &derive_input.ident;
    let generics = &derive_input.generics;

    let generic_defs = generics.params.iter().map(|param| match param {
        syn::GenericParam::Type(t) => {
            if t.bounds.is_empty() {
                quote!(#t: 'static)
            } else {
                quote!(#t + 'static)
            }
        }
        syn::GenericParam::Lifetime(l) => quote!(#l),
        syn::GenericParam::Const(c) => quote!(#c),
    });

    let macro_crate = Ident::new(env!("CARGO_CRATE_NAME"), Span::mixed_site());
    quote! {
        #[derive(::zerocopy::FromBytes, ::zerocopy::IntoBytes)]
        #[::#macro_crate::__derive_stub]
        #derive_input

        impl <#(#generic_defs),*> crate::memory::PtrReadable for #ident #generics {}
    }
    .into()
}

#[proc_macro_derive(ComponentName)]
pub fn derive_component_name(item: TokenStream) -> TokenStream {
    let derive_input = syn::parse_macro_input!(item as syn::DeriveInput);
    let ident = &derive_input.ident;
    let name = ident.to_string();
    let name = name.trim_start_matches("r#");

    quote! {
        impl crate::noita::types::components::ComponentName for #ident {
            const NAME: &'static str = #name;
        }
    }
    .into()
}
