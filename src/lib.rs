mod tower;

use {
    crate::tower::{ParameterData, ResultData},
    heck::CamelCase,
    proc_macro::TokenStream as RawTokenStream,
    proc_macro2::TokenStream,
    quote::quote,
    syn::{parse_macro_input, FnArg, Ident, ImplItem, ImplItemMethod, ItemImpl, Type},
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
    let service_methods = build_service_methods(item);

    quote! {
        pub struct Service;

        #service_impl
        #service_methods
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

fn extract_result_data(item: &ItemImpl) -> ResultData {
    item.items
        .iter()
        .filter_map(|item| match item {
            ImplItem::Method(method) => Some(ResultData::new(&method.sig.output)),
            _ => None,
        })
        .next()
        .expect("No methods in `impl` item")
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
            FnArg::Typed(argument) => Some(ParameterData::new(&argument)),
        })
        .map(|parameter_data| parameter_data.binding())
}

fn build_service_request_match_arm(self_type: &Type, method: &ImplItemMethod) -> TokenStream {
    let bindings = build_request_match_bindings(method);
    let method_name = &method.sig.ident;

    quote! {
        futures::FutureExt::boxed(#self_type::#method_name( #( #bindings ),* ))
    }
}

fn build_service_methods(item: &ItemImpl) -> TokenStream {
    let service_methods = item.items.iter().filter_map(|item| match item {
        ImplItem::Method(method) => Some(build_service_method(method)),
        _ => None,
    });

    quote! {
        impl Service {
            #( #service_methods )*
        }
    }
}

fn build_service_method(method: &ImplItemMethod) -> TokenStream {
    let method_name = &method.sig.ident;
    let parameters = build_service_method_parameters(method);
    let result = &method.sig.output;
    let request = build_service_method_request(method);

    quote! {
        pub async fn #method_name(&mut self, #( #parameters ),*) #result {
            use tower::{Service as _, ServiceExt as _};

            self.ready().await?.call(#request).await
        }
    }
}

fn build_service_method_parameters(
    method: &ImplItemMethod,
) -> impl Iterator<Item = TokenStream> + '_ {
    method
        .sig
        .inputs
        .iter()
        .filter_map(|argument| match argument {
            FnArg::Receiver(_) => None,
            FnArg::Typed(argument) => Some(quote! { #argument }),
        })
}

fn build_service_method_request(method: &ImplItemMethod) -> TokenStream {
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
