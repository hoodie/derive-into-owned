pub fn has_lifetime_arguments(segments: &[syn::PathSegment]) -> bool {
    if let Some(&syn::PathArguments::AngleBracketed(ref args)) =
        segments.last().map(|x| &x.arguments)
    {
        !args
            .args
            .into_iter()
            .any(|f| matches!(f, syn::GenericArgument::Lifetime(_)))
    } else {
        false
    }
}

pub fn number_of_type_arguments(segments: &[syn::PathSegment]) -> usize {
    if let Some(&syn::PathArguments::AngleBracketed(ref args)) =
        segments.last().map(|x| &x.arguments)
    {
        args.args
            .into_iter()
            .filter(|f| matches!(f, syn::GenericArgument::Type(_)))
            .count()
    } else {
        0
    }
}

pub fn has_binding_arguments(segments: &[syn::PathSegment]) -> bool {
    if let Some(&syn::PathArguments::AngleBracketed(ref args)) =
        segments.last().map(|x| &x.arguments)
    {
        !args
            .args
            .into_iter()
            .any(|f| matches!(f, syn::GenericArgument::Binding(_)))
    } else {
        false
    }
}

pub fn is_cow_alike(segments: &[syn::PathSegment]) -> bool {
    if let Some(&syn::PathArguments::AngleBracketed(ref args)) =
        segments.last().map(|x| &x.arguments)
    {
        !args
            .args
            .into_iter()
            .any(|f| matches!(f, syn::GenericArgument::Lifetime(_)))
    } else {
        false
    }
}

pub fn last_ident(ty: &syn::Type) -> syn::Ident {
    if let syn::Type::Path(ref path) = ty {
        path.path.segments.last().as_ref().unwrap().ident.clone()
    } else {
        unimplemented!()
    }
}

pub fn type_inside_param(ty: &syn::Type) -> syn::Ident {
    if let syn::Type::Path(ref path) = ty {
        if let syn::PathArguments::AngleBracketed(syn::AngleBracketedGenericArguments {
            ref args,
            ..
        }) = path.path.segments.last().unwrap().arguments
        {
            if let syn::GenericArgument::Type(ref ty) = args[0] {
                last_ident(ty)
            } else {
                unimplemented!()
            }
        } else {
            unimplemented!()
        }
    } else {
        unimplemented!()
    }
}
