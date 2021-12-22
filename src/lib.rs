//! # derive_into_owned
//!
//! This crate supports deriving two different methods (not traits):
//!
//!  * `IntoOwned`
//!  * `Borrowed`
//!
//! These were first created to help out with types generated by [`quick_protobuf`] which generates
//! structs with [`Cow`] fields. It is entirely possible that this crate is not needed at all and
//! that there exists better alternatives to what this crate aims to support.
//!
//! ## Definitions, naming
//!
//! "Cow-alike" is used to mean a type that:
//!
//!  * has a lifetime specifier, like `Foo<'a>`
//!  * has an implementation for `fn into_owned(self) -> Foo<'static>`
//!
//! This is a bit of a bad name as [`Cow`] itself has a different kind of `into_owned` which
//! returns the owned version of the generic type parameter. I am open to suggestions for both
//! "Cow-alike" and `into_owned`.
//!
//! ## `IntoOwned`
//!
//! `#[derive(IntoOwned)]` implements a method `fn into_owned(self) -> Foo<'static>` for type
//! `Foo<'a>` which contains [`Cow`] or "Cow-alike" fields. The method returns a version with
//! `'static` lifetime which means the value owns all of it's data. This is useful if you are
//! for example, working with [`tokio-rs`] which currently requires types to be `'static`.
//!
//! ## `Borrowed`
//!
//! `#[derive(Borrowed)]` implements a method `fn borrowed<'b>(&'b self) -> Foo<'b>` for type
//! `Foo<'a>`. This is useful in case you need to transform the value into another type using
//! std conversions like [`From`], but you don't want to clone the data in the process. Note that
//! the all the fields that are not [`Cow`] or "Cow-alike" are just cloned, and new vectors are
//! collected, so this yields savings only when you manage to save big chunks of memory.
//!
//! ## Limitations
//!
//! Currently only the types I needed are supported and this might be a rather limited set of
//! functionality. If you find that this does not work in your case please file an issue at [project
//! repository](https://github.com/koivunej/derive-into-owned/issues).
//!
//! [`quick_protobuf`]: https://github.com/tafia/quick-protobuf/
//! [`tokio-rs`]: https://tokio.rs
//! [`Cow`]: https://doc.rust-lang.org/std/borrow/enum.Cow.html
//! [`From`]: https://doc.rust-lang.org/std/convert/trait.From.html

#[macro_use]
extern crate quote;

use core::slice::SlicePattern;

use helpers::{has_binding_arguments, has_lifetime_arguments};
use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};

mod helpers;

#[proc_macro_derive(IntoOwned)]
#[doc(hidden)]
pub fn into_owned(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);

    let expanded = impl_with_generator(&ast, IntoOwnedGen);

    TokenStream::from(expanded)
}

#[proc_macro_derive(Borrowed)]
#[doc(hidden)]
pub fn borrowed(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);

    let expanded = impl_with_generator(&ast, BorrowedGen);

    TokenStream::from(expanded)
}

fn impl_with_generator<G: BodyGenerator>(
    ast: &syn::DeriveInput,
    gen: G,
) -> proc_macro2::TokenStream {
    // this is based heavily on https://github.com/asajeffrey/deep-clone/blob/master/deep-clone-derive/lib.rs
    let name = &ast.ident;

    let borrowed_params = gen.quote_borrowed_params(ast);
    let borrowed = if borrowed_params.is_empty() {
        quote! {}
    } else {
        quote! { < #(#borrowed_params),* > }
    };

    let params = gen.quote_type_params(ast);
    let params = if params.is_empty() {
        quote! {}
    } else {
        quote! { < #(#params),* > }
    };

    let owned_params = gen.quote_rhs_params(ast);
    let owned = if owned_params.is_empty() {
        quote! {}
    } else {
        quote! { < #(#owned_params),* > }
    };

    let body = match ast.data {
        syn::Data::Struct(ref variant) => {
            let inner = gen.visit_struct(variant);
            quote! { #name #inner }
        }
        syn::Data::Enum(ref body) => {
            let cases = body.variants.iter().map(|variant| {
                let unqualified_ident = &variant.ident;
                let ident = quote! { #name::#unqualified_ident };

                gen.visit_enum_data(ident, &variant.fields)
            });
            quote! { match self { #(#cases),* } }
        }
        syn::Data::Union(_) => todo!(),
    };

    gen.combine_impl(borrowed, name, params, owned, body)
}

/// Probably not the best abstraction
trait BodyGenerator {
    fn quote_borrowed_params(&self, ast: &syn::DeriveInput) -> Vec<proc_macro2::TokenStream> {
        let borrowed_lifetime_params = ast.generics.lifetimes.iter().map(|alpha| quote! { #alpha });
        let borrowed_type_params = ast.generics.ty_params.iter().map(|ty| quote! { #ty });
        borrowed_lifetime_params
            .chain(borrowed_type_params)
            .collect::<Vec<_>>()
    }

    fn quote_type_params(&self, ast: &syn::DeriveInput) -> Vec<proc_macro2::TokenStream> {
        ast.generics
            .lifetimes
            .iter()
            .map(|alpha| quote! { #alpha })
            .chain(ast.generics.ty_params.iter().map(|ty| {
                let ident = &ty.ident;
                quote! { #ident }
            }))
            .collect::<Vec<_>>()
    }

    fn quote_rhs_params(&self, ast: &syn::DeriveInput) -> Vec<proc_macro2::TokenStream> {
        let owned_lifetime_params = ast.generics.lifetimes.iter().map(|_| quote! { 'static });
        let owned_type_params = ast.generics.ty_params.iter().map(|ty| {
            let ident = &ty.ident;
            quote! { #ident }
        });
        owned_lifetime_params
            .chain(owned_type_params)
            .collect::<Vec<_>>()
    }

    fn visit_struct(&self, data: &syn::Data) -> proc_macro2::TokenStream;
    fn visit_enum_data(
        &self,
        variant: proc_macro2::TokenStream,
        data: &syn::Data,
    ) -> proc_macro2::TokenStream;
    fn combine_impl(
        &self,
        borrows: proc_macro2::TokenStream,
        name: &syn::Ident,
        rhs_params: proc_macro2::TokenStream,
        owned: proc_macro2::TokenStream,
        body: proc_macro2::TokenStream,
    ) -> proc_macro2::TokenStream;
}

struct IntoOwnedGen;

impl BodyGenerator for IntoOwnedGen {
    fn visit_struct(&self, data: &syn::Data) -> proc_macro2::TokenStream {
        match *data {
            syn::Data::Struct(ref data) => {
                let fields = data.fields.iter().map(|field| {
                    let ident = field.ident.as_ref().unwrap();
                    let field_ref = quote! { self.#ident };
                    let code = FieldKind::resolve(field).move_or_clone_field(&field_ref);
                    quote! { #ident: #code }
                });
                quote! { { #(#fields),* } }
            }
            syn::Data::Enum(_) => todo!(),
            syn::Data::Union(_) => todo!(),
            // syn::Data::Tuple(ref body) => {
            //     let fields = body.iter().enumerate().map(|(index, field)| {
            //         let index = quote::format_ident!(index);
            //         let index = quote! { self.#index };
            //         FieldKind::resolve(field).move_or_clone_field(&index)
            //     });
            //     quote! { ( #(#fields),* ) }
            // }
        }
    }

    fn visit_enum_data(
        &self,
        ident: proc_macro2::TokenStream,
        data: &syn::Data,
    ) -> proc_macro2::TokenStream {
        match *data {
            syn::Data::Struct(ref data) => {
                let idents = data
                    .fields
                    .iter()
                    .map(|field| field.ident.as_ref().unwrap());
                let cloned = data.fields.iter().map(|field| {
                    let ident = field.ident.as_ref().unwrap();
                    let ident = quote! { #ident };
                    let code = FieldKind::resolve(field).move_or_clone_field(&ident);
                    quote! { #ident: #code }
                });
                quote! { #ident { #(#idents),* } => #ident { #(#cloned),* } }
            }
            syn::Data::Enum(_) => todo!(),
            syn::Data::Union(_) => todo!(),
            // syn::VariantData::Tuple(ref body) => {
            //     let idents = (0..body.len())
            //         .map(|index| quote::format_ident!(format!("x{}", index)))
            //         .collect::<Vec<_>>();
            //     let cloned = idents
            //         .iter()
            //         .zip(body.iter())
            //         .map(|(ident, field)| {
            //             let ident = quote! { #ident };
            //             FieldKind::resolve(field).move_or_clone_field(&ident)
            //         })
            //         .collect::<Vec<_>>();
            //     quote! { #ident ( #(#idents),* ) => #ident ( #(#cloned),* ) }
            // }
            // syn::VariantData::Unit => {
            //     quote! { #ident => #ident }
            // }
        }
    }

    fn combine_impl(
        &self,
        borrowed: proc_macro2::TokenStream,
        name: &syn::Ident,
        params: proc_macro2::TokenStream,
        owned: proc_macro2::TokenStream,
        body: proc_macro2::TokenStream,
    ) -> proc_macro2::TokenStream {
        quote! {
            impl #borrowed #name #params {
                /// Returns a version of `self` with all fields converted to owning versions.
                pub fn into_owned(self) -> #name #owned { #body }
            }
        }
    }
}

struct BorrowedGen;

impl BodyGenerator for BorrowedGen {
    fn quote_rhs_params(&self, ast: &syn::DeriveInput) -> Vec<proc_macro2::TokenStream> {
        let owned_lifetime_params = ast
            .generics
            .lifetimes
            .iter()
            .map(|_| quote! { '__borrowedgen });
        let owned_type_params = ast.generics.ty_params.iter().map(|ty| {
            let ident = &ty.ident;
            quote! { #ident }
        });
        owned_lifetime_params
            .chain(owned_type_params)
            .collect::<Vec<_>>()
    }

    fn visit_struct(&self, data: &syn::Data) -> proc_macro2::TokenStream {
        match *data {
            syn::Data::Struct(ref data) => {
                let fields = data.fields.iter().map(|field| {
                    let ident = field.ident.as_ref().unwrap();
                    let field_ref = quote! { self.#ident };
                    let code = FieldKind::resolve(field).borrow_or_clone(&field_ref);
                    quote! { #ident: #code }
                });
                quote! { { #(#fields),* } }
            }
            syn::Data::Enum(_) => todo!(),
            syn::Data::Union(_) => todo!(),
            // syn::Data::Tuple(ref body) => {
            //     let fields = body.iter().enumerate().map(|(index, field)| {
            //         let index = quote::format_ident!(index);
            //         let index = quote! { self.#index };
            //         FieldKind::resolve(field).borrow_or_clone(&index)
            //     });
            //     quote! { ( #(#fields),* ) }
            // }
        }
    }

    fn visit_enum_data(
        &self,
        ident: proc_macro2::TokenStream,
        data: &syn::Data,
    ) -> proc_macro2::TokenStream {
        match *data {
            syn::Data::Struct(ref data) => {
                let idents = data
                    .fields
                    .iter()
                    .map(|field| field.ident.as_ref().unwrap());
                let cloned = data.fields.iter().map(|field| {
                    let ident = field.ident.as_ref().unwrap();
                    let ident = quote! { #ident };
                    let code = FieldKind::resolve(field).borrow_or_clone(&ident);
                    quote! { #ident: #code }
                });
                quote! { #ident { #(ref #idents),* } => #ident { #(#cloned),* } }
            }
            syn::Data::Enum(_) => todo!(),
            syn::Data::Union(_) => todo!(),
            // syn::VariantData::Tuple(ref body) => {
            //     let idents = (0..body.len())
            //         .map(|index| quote::format_ident!("x{}", index))
            //         .collect::<Vec<_>>();
            //     let cloned = idents
            //         .iter()
            //         .zip(body.iter())
            //         .map(|(ident, field)| {
            //             let ident = quote! { #ident };
            //             FieldKind::resolve(field).borrow_or_clone(&ident)
            //         })
            //         .collect::<Vec<_>>();
            //     quote! { #ident ( #(ref #idents),* ) => #ident ( #(#cloned),* ) }
            // }
            // syn::VariantData::Unit => {
            //     quote! { #ident => #ident }
            // }
        }
    }

    fn combine_impl(
        &self,
        borrowed: proc_macro2::TokenStream,
        name: &syn::Ident,
        params: proc_macro2::TokenStream,
        owned: proc_macro2::TokenStream,
        body: proc_macro2::TokenStream,
    ) -> proc_macro2::TokenStream {
        quote! {
            impl #borrowed #name #params {
                /// Returns a clone of `self` that shares all the "Cow-alike" data with `self`.
                pub fn borrowed<'__borrowedgen>(&'__borrowedgen self) -> #name #owned { #body }
            }
        }
    }
}

enum FieldKind {
    PlainCow,
    AssumedCow,
    /// Option fields with either PlainCow or AssumedCow
    OptField(usize, Box<FieldKind>),
    IterableField(Box<FieldKind>),
    JustMoved,
}

impl FieldKind {
    fn resolve(field: &syn::Field) -> Self {
        // FIXME: remove this obserdirty
        let hack = |path: &syn::Path| path.segments.iter().cloned().collect::<Vec<_>>().as_slice();
        match field.ty {
            syn::Type::Path(syn::TypePath { ref path, .. }) => {
                if is_cow(hack(path)) {
                    FieldKind::PlainCow
                } else if is_cow_alike(hack(path)) {
                    FieldKind::AssumedCow
                } else if let Some(kind) = is_opt_cow(hack(path)) {
                    kind
                } else if let Some(kind) = is_iter_field(hack(path)) {
                    kind
                } else {
                    FieldKind::JustMoved
                }
            }
            _ => FieldKind::JustMoved,
        }
    }

    fn move_or_clone_field(&self, var: &proc_macro2::TokenStream) -> proc_macro2::TokenStream {
        use self::FieldKind::*;

        match *self {
            PlainCow => quote! { ::std::borrow::Cow::Owned(#var.into_owned()) },
            AssumedCow => quote! { #var.into_owned() },
            OptField(levels, ref inner) => {
                let next = quote::format_ident!("val");
                let next = quote! { #next };

                let mut tokens = inner.move_or_clone_field(&next);

                for _ in 0..(levels - 1) {
                    tokens = quote! { #next.map(|#next| #tokens) };
                }

                quote! { #var.map(|#next| #tokens) }
            }
            IterableField(ref inner) => {
                let next = quote::format_ident!("x");
                let next = quote! { #next };

                let tokens = inner.move_or_clone_field(&next);

                quote! { #var.into_iter().map(|x| #tokens).collect() }
            }
            JustMoved => quote! { #var },
        }
    }

    fn borrow_or_clone(&self, var: &proc_macro2::TokenStream) -> proc_macro2::TokenStream {
        use self::FieldKind::*;

        match *self {
            PlainCow => quote! { ::std::borrow::Cow::Borrowed(#var.as_ref()) },
            AssumedCow => quote! { #var.borrowed() },
            OptField(levels, ref inner) => {
                let next = quote::format_ident!("val");
                let next = quote! { #next };

                let mut tokens = inner.borrow_or_clone(&next);

                for _ in 0..(levels - 1) {
                    tokens = quote! { #next.as_ref().map(|#next| #tokens) };
                }

                quote! { #var.as_ref().map(|#next| #tokens) }
            }
            IterableField(ref inner) => {
                let next = quote::format_ident!("x");
                let next = quote! { #next };

                let tokens = inner.borrow_or_clone(&next);

                quote! { #var.iter().map(|x| #tokens).collect() }
            }
            JustMoved => quote! { #var.clone() },
        }
    }
}

fn type_hopefully_is(segments: &[syn::PathSegment], expected: &str) -> bool {
    let expected = expected
        .split("::")
        .map(|x| quote::format_ident!("{}", x))
        .collect::<Vec<_>>();
    if segments.len() > expected.len() {
        return false;
    }

    let expected = expected.iter().collect::<Vec<_>>();
    let segments = segments.iter().map(|x| &x.ident).collect::<Vec<_>>();

    for len in 0..expected.len() {
        if segments[..] == expected[expected.len() - len - 1..] {
            return true;
        }
    }

    false
}

fn is_cow(segments: &[syn::PathSegment]) -> bool {
    type_hopefully_is(segments, "std::borrow::Cow")
}

fn is_cow_alike(segments: &[syn::PathSegment]) -> bool {
    if let Some(&syn::PathArguments::AngleBracketed(ref data)) =
        segments.last().map(|x| &x.arguments)
    {
        has_lifetime_arguments(segments)
    } else {
        false
    }
}

fn is_opt_cow(mut segments: &[syn::PathSegment]) -> Option<FieldKind> {
    let mut levels = 0;
    loop {
        if type_hopefully_is(segments, "std::option::Option") {
            if let syn::PathSegment {
                parameters: syn::PathParameters::AngleBracketed(ref data),
                ..
            } = *segments.last().unwrap()
            {
                if has_lifetime_arguments(segments) || has_binding_arguments(segments) {
                    // Option<&'a ?> cannot be moved but let the compiler complain
                    // don't know about data bindings
                    break;
                }

                if data.types.len() != 1 {
                    // Option<A, B> probably means some other, movable option
                    break;
                }

                match *data.types.first().unwrap() {
                    syn::Type::Path(
                        None,
                        syn::Path {
                            segments: ref next_segments,
                            ..
                        },
                    ) => {
                        levels += 1;
                        segments = next_segments;
                        continue;
                    }
                    _ => break,
                }
            }
        } else if is_cow(segments) {
            return Some(FieldKind::OptField(levels, Box::new(FieldKind::PlainCow)));
        } else if is_cow_alike(segments) {
            return Some(FieldKind::OptField(levels, Box::new(FieldKind::AssumedCow)));
        }

        break;
    }

    None
}

fn is_iter_field(mut segments: &[syn::PathSegment]) -> Option<FieldKind> {
    loop {
        // this should be easy to do for arrays as well..
        if type_hopefully_is(segments, "std::vec::Vec") {
            if let syn::PathSegment {
                parameters: syn::PathParameters::AngleBracketed(ref data),
                ..
            } = *segments.last().unwrap()
            {
                if has_lifetime_arguments(segments) || has_binding_arguments(segments) {
                    break;
                }

                if data.types.len() != 1 {
                    // TODO: this could be something like Vec<(u32, Bar<'a>)>?
                    break;
                }

                match *data.types.first().unwrap() {
                    syn::Type::Path(
                        None,
                        syn::Path {
                            segments: ref next_segments,
                            ..
                        },
                    ) => {
                        segments = next_segments;
                        continue;
                    }
                    _ => break,
                }
            }
        } else if is_cow(segments) {
            return Some(FieldKind::IterableField(Box::new(FieldKind::PlainCow)));
        } else if is_cow_alike(segments) {
            return Some(FieldKind::IterableField(Box::new(FieldKind::AssumedCow)));
        }

        break;
    }

    None
}
