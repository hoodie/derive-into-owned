#![allow(unused_imports, dead_code)]
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, DeriveInput, Field};

fn field_is_container(container_name: &str) -> impl Fn(&Field) -> bool {
    let container_name = container_name.to_owned();
    move |field: &Field| -> bool {
        if let syn::Type::Path(ref path) = field.ty {
            path.path.segments.last().unwrap();
            path.path.segments.last().unwrap().ident == container_name
        } else {
            false
        }
    }
}

#[proc_macro_derive(IntoOwned)]
pub fn derive(input: TokenStream) -> TokenStream {
    // destructure basic struct
    let DeriveInput {
        ident: ref struct_ident,
        data: ref struct_data,
        ref generics,
        ..
    } = parse_macro_input!(input as DeriveInput);

    let (struct_impl_generics, struct_type_generics, ..) = generics.split_for_impl();

    // compile error, this is to complicated for me
    if number_of_type_params(generics) > 1 {
        panic!(
            "this type has too many type parameters, only a single lifetime parameter is supported"
        )
    }

    let struct_fields = if let syn::Data::Struct(syn::DataStruct {
        fields: syn::Fields::Named(ref fields),
        ..
    }) = struct_data
    {
        &fields.named
    } else {
        unimplemented!()
    };

    // helpers
    let field_is_container = |container_name: &str| {
        let container_name = container_name.to_owned();
        move |field: &Field| -> bool {
            if let syn::Type::Path(ref path) = field.ty {
                path.path.segments.last().unwrap();
                path.path.segments.last().unwrap().ident == container_name
            } else {
                false
            }
        }
    };

    let field_is_cow = field_is_container("Cow");

    // prepared quotes

    // thing: Cow::Owned(self.thing.into_owned())
    let fields_from_self = struct_fields.iter().map(|field @ Field { ident, .. }| {
        if field_is_cow(field) {
            quote! { #ident: ::std::borrow::Cow::Owned(self.#ident.into_owned()) }
        } else {
            quote! { #ident:  self.#ident }
        }
    });

    let expanded = quote! {
        impl#struct_impl_generics #struct_ident #struct_type_generics {
            fn to_owned(self) -> #struct_ident <'a> {
                #struct_ident {
                #(#fields_from_self),*
                }
            }
        }
    };

    TokenStream::from(expanded)
}

fn number_of_type_params(generics: &syn::Generics) -> usize {
    generics.params.iter().count()
}
