mod tower;

use {
    crate::tower::{MethodData, ParameterData, ResultData},
    heck::CamelCase,
    proc_macro::TokenStream as RawTokenStream,
    proc_macro2::TokenStream,
    quote::quote,
    syn::{parse_macro_input, FnArg, Ident, ImplItem, ImplItemMethod, ItemImpl, Type},
};

#[proc_macro_attribute]
pub fn tower(_attribute: RawTokenStream, item_tokens: RawTokenStream) -> RawTokenStream {
    let item = parse_macro_input!(item_tokens as ItemImpl);
    let methods = extract_method_data(&item);
    let request = build_request(&methods);
    let service = build_service(&item, &methods);

    RawTokenStream::from(quote! {
        #request
        #item
        #service
    })
}

fn extract_method_data(item: &ItemImpl) -> Vec<MethodData> {
    item.items
        .iter()
        .filter_map(|item| match item {
            ImplItem::Method(method) => Some(MethodData::new(&method)),
            _ => None,
        })
        .collect()
}

fn build_request(methods: &[MethodData]) -> TokenStream {
    let variants = methods.iter().map(build_request_variant);

    quote! {
        pub enum Request {
            #( #variants ),*
        }
    }
}

fn build_request_variant(method: &MethodData) -> TokenStream {
    let name_string = method.name().to_string().to_camel_case();
    let name = Ident::new(&name_string, method.name().span());
    let parameters = method.parameters();

    if !parameters.is_empty() {
        let fields = parameters.iter().map(ParameterData::declaration);

        quote! {
            #name {
                #( #fields ),*
            }
        }
    } else {
        quote! { #name }
    }
}

fn extract_parameter_data(method: &ImplItemMethod) -> Vec<ParameterData> {
    method
        .sig
        .inputs
        .iter()
        .filter_map(|argument| match argument {
            FnArg::Receiver(_) => None,
            FnArg::Typed(argument) => Some(ParameterData::new(&argument)),
        })
        .collect()
}

fn build_service(item: &ItemImpl, methods: &[MethodData]) -> TokenStream {
    let service_impl = build_service_impl(item);
    let service_methods = build_service_methods(methods);

    quote! {
        pub struct Service;

        #service_impl
        #service_methods
    }
}

fn build_service_impl(item: &ItemImpl) -> TokenStream {
    let result_data = extract_result_data(item);
    let response = result_data.ok_type();
    let error = result_data.err_type();
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
    let parameters = extract_parameter_data(method);

    if !parameters.is_empty() {
        let bindings = parameters.iter().map(ParameterData::binding);

        quote! {
            Request::#name {
                #( #bindings ),*
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

fn build_service_methods(methods: &[MethodData]) -> TokenStream {
    let service_methods = methods.iter().map(build_service_method);

    quote! {
        impl Service {
            #( #service_methods )*
        }
    }
}

fn build_service_method(method: &MethodData) -> TokenStream {
    let method_name = method.name();
    let parameters = method.parameters().iter().map(ParameterData::declaration);
    let result = method.result();
    let request = build_service_method_request(method);

    quote! {
        pub async fn #method_name(&mut self, #( #parameters ),*) #result {
            use tower::{Service as _, ServiceExt as _};

            self.ready().await?.call(#request).await
        }
    }
}

fn build_service_method_request(method: &MethodData) -> TokenStream {
    let name_string = method.name().to_string().to_camel_case();
    let name = Ident::new(&name_string, method.name().span());
    let parameters = method.parameters();

    if !parameters.is_empty() {
        let bindings = parameters.iter().map(ParameterData::binding);

        quote! {
            Request::#name {
                #( #bindings ),*
            }
        }
    } else {
        quote! { Request::#name }
    }
}
