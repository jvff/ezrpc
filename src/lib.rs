use {
    heck::CamelCase,
    proc_macro::TokenStream as RawTokenStream,
    proc_macro2::TokenStream,
    quote::quote,
    syn::{
        parse_macro_input, FnArg, GenericArgument, Ident, ImplItem, ImplItemMethod, ItemImpl, Path,
        PathArguments, ReturnType, Type,
    },
};

#[proc_macro_attribute]
pub fn tower(_attribute: RawTokenStream, item_tokens: RawTokenStream) -> RawTokenStream {
    let item = parse_macro_input!(item_tokens as ItemImpl);
    let request = build_request(&item);
    let service = build_service(&item);

    RawTokenStream::from(quote! {
        #request
        #item
        #service
    })
}

fn build_request(item: &ItemImpl) -> TokenStream {
    let variants = item.items.iter().filter_map(|item| match item {
        ImplItem::Method(method) => Some(build_request_variant(method)),
        _ => None,
    });

    quote! {
        pub enum Request {
            #( #variants ),*
        }
    }
}

fn build_request_variant(method: &ImplItemMethod) -> TokenStream {
    let name_string = method.sig.ident.to_string().to_camel_case();
    let name = Ident::new(&name_string, method.sig.ident.span());

    if has_parameters(method) {
        let parameters = build_request_variant_parameters(method);

        quote! {
            #name {
                #( #parameters ),*
            }
        }
    } else {
        quote! { #name }
    }
}

fn has_parameters(method: &ImplItemMethod) -> bool {
    let inputs = &method.sig.inputs;

    if inputs.is_empty() {
        false
    } else if inputs.len() > 1 {
        true
    } else {
        !matches!(inputs.first(), Some(FnArg::Receiver(_)))
    }
}

fn build_request_variant_parameters(
    method: &ImplItemMethod,
) -> impl Iterator<Item = TokenStream> + '_ {
    method
        .sig
        .inputs
        .iter()
        .filter_map(|argument| match argument {
            FnArg::Receiver(_) => None,
            FnArg::Typed(argument) => Some(argument),
        })
        .map(|argument| {
            let pattern = &argument.pat;
            let argument_type = &argument.ty;

            quote! { #pattern: #argument_type }
        })
}

fn build_service(item: &ItemImpl) -> TokenStream {
    let service_impl = build_service_impl(item);

    quote! {
        pub struct Service;

        #service_impl
    }
}

fn build_service_impl(item: &ItemImpl) -> TokenStream {
    let response = build_service_response(item);
    let error = build_service_error(item);
    let request_calls = build_service_request_calls(item);

    quote! {
        impl tower::Service<Request> for Service {
            type Response = #response;
            type Error = #error;
            type Future = std::pin::Pin<Box<
                dyn std::future::Future<Output = Result<Self::Response, Self::Error>>
            >>;

            fn poll_ready(
                &mut self,
                context: &mut std::task::Context<'_>,
            ) -> std::task::Poll<Result<(), Self::Error>> {
                std::task::Poll::Ready(Ok(()))
            }

            fn call(&mut self, request: Request) -> Self::Future {
                match request {
                    #( #request_calls ),*
                }
            }
        }
    }
}

fn build_service_response(item: &ItemImpl) -> TokenStream {
    item.items
        .iter()
        .filter_map(|item| match item {
            ImplItem::Method(method) => Some(build_service_response_from_method(method)),
            _ => None,
        })
        .next()
        .expect("No methods in `impl` item")
}

fn build_service_response_from_method(method: &ImplItemMethod) -> TokenStream {
    match &method.sig.output {
        ReturnType::Default => quote! { () },
        ReturnType::Type(_, return_type) => build_service_response_from_return_type(&return_type),
    }
}

fn build_service_response_from_return_type(return_type: &Type) -> TokenStream {
    match return_type {
        Type::Path(path_type) if path_type.qself.is_none() => {
            extract_result_ok_type(&path_type.path).unwrap_or_else(|| quote! { #path_type })
        }
        other => quote! { #other },
    }
}

fn extract_result_ok_type(path: &Path) -> Option<TokenStream> {
    let mut segments = path.segments.iter();
    let first_segment = segments.next()?;

    let type_arguments = if first_segment.ident == "Result" && path.leading_colon.is_none() {
        &first_segment.arguments
    } else {
        let second_segment = segments.next()?;
        let third_segment = segments.next()?;

        if first_segment.ident != "std"
            || second_segment.ident != "result"
            || third_segment.ident != "Result"
        {
            return None;
        }

        &third_segment.arguments
    };

    match type_arguments {
        PathArguments::AngleBracketed(arguments) => arguments
            .args
            .iter()
            .map(|argument| match argument {
                GenericArgument::Type(ok_type) => quote! { #ok_type },
                _ => panic!("Unexpected generic argument in Result type"),
            })
            .next(),
        _ => None,
    }
}

fn build_service_error(item: &ItemImpl) -> TokenStream {
    item.items
        .iter()
        .filter_map(|item| match item {
            ImplItem::Method(method) => build_service_error_from_method(method),
            _ => None,
        })
        .next()
        .unwrap_or_else(|| quote! { () })
}

fn build_service_error_from_method(method: &ImplItemMethod) -> Option<TokenStream> {
    match &method.sig.output {
        ReturnType::Default => None,
        ReturnType::Type(_, return_type) => build_service_error_from_return_type(&return_type),
    }
}

fn build_service_error_from_return_type(return_type: &Type) -> Option<TokenStream> {
    match return_type {
        Type::Path(path_type) if path_type.qself.is_none() => {
            extract_result_error_type(&path_type.path)
        }
        _ => None,
    }
}

fn extract_result_error_type(path: &Path) -> Option<TokenStream> {
    let mut segments = path.segments.iter();
    let first_segment = segments.next()?;

    let type_arguments = if first_segment.ident == "Result" && path.leading_colon.is_none() {
        &first_segment.arguments
    } else {
        let second_segment = segments.next()?;
        let third_segment = segments.next()?;

        if first_segment.ident != "std"
            || second_segment.ident != "result"
            || third_segment.ident != "Result"
        {
            return None;
        }

        &third_segment.arguments
    };

    match type_arguments {
        PathArguments::AngleBracketed(arguments) => arguments
            .args
            .iter()
            .map(|argument| match argument {
                GenericArgument::Type(ok_type) => quote! { #ok_type },
                _ => panic!("Unexpected generic argument in Result type"),
            })
            .skip(1)
            .next(),
        _ => None,
    }
}

fn build_service_request_calls(item: &ItemImpl) -> impl Iterator<Item = TokenStream> + '_ {
    let self_type = &item.self_ty;

    item.items
        .iter()
        .filter_map(|item| match item {
            ImplItem::Method(method) => Some(method),
            _ => None,
        })
        .map(move |method| {
            let request = build_service_request_match_pattern(method);
            let body = build_service_request_match_arm(self_type, method);

            quote! { #request => #body }
        })
}

fn build_service_request_match_pattern(method: &ImplItemMethod) -> TokenStream {
    let name_string = method.sig.ident.to_string().to_camel_case();
    let name = Ident::new(&name_string, method.sig.ident.span());

    if has_parameters(method) {
        let parameters = build_request_match_bindings(method);

        quote! {
            Request::#name {
                #( #parameters ),*
            }
        }
    } else {
        quote! { Request::#name }
    }
}

fn build_request_match_bindings(method: &ImplItemMethod) -> impl Iterator<Item = TokenStream> + '_ {
    method
        .sig
        .inputs
        .iter()
        .filter_map(|argument| match argument {
            FnArg::Receiver(_) => None,
            FnArg::Typed(argument) => Some(argument),
        })
        .map(|argument| {
            let pattern = &argument.pat;

            quote! { #pattern }
        })
}

fn build_service_request_match_arm(self_type: &Type, method: &ImplItemMethod) -> TokenStream {
    let bindings = build_request_match_bindings(method);
    let method_name = &method.sig.ident;

    quote! {
        futures::FutureExt::boxed(#self_type::#method_name( #( #bindings ),* ))
    }
}
