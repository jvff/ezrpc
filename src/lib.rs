mod tower;

use {
    crate::tower::{MethodData, ParameterData},
    proc_macro::TokenStream as RawTokenStream,
    proc_macro2::TokenStream,
    quote::quote,
    syn::{parse_macro_input, ImplItem, ItemImpl, Type},
};

#[proc_macro_attribute]
pub fn tower(_attribute: RawTokenStream, item_tokens: RawTokenStream) -> RawTokenStream {
    let item = parse_macro_input!(item_tokens as ItemImpl);
    let methods = extract_method_data(&item);
    let request = build_request(&methods);
    let service = build_service(&item.self_ty, &methods);

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
    let variants = methods.iter().map(MethodData::request_enum_variant);

    quote! {
        pub enum Request {
            #( #variants ),*
        }
    }
}

fn build_service(self_type: &Type, methods: &[MethodData]) -> TokenStream {
    let service_impl = build_service_impl(self_type, methods);
    let service_methods = build_service_methods(methods);

    quote! {
        pub struct Service;

        #service_impl
        #service_methods
    }
}

fn build_service_impl(self_type: &Type, methods: &[MethodData]) -> TokenStream {
    let result_data = methods
        .iter()
        .next()
        .expect("No methods in `impl` item")
        .result();
    let response = result_data.ok_type();
    let error = result_data.err_type();
    let request_calls = build_service_request_calls(self_type, methods);

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

fn build_service_request_calls<'r, 's: 'r, 'm: 'r>(
    self_type: &'s Type,
    methods: &'m [MethodData],
) -> impl Iterator<Item = TokenStream> + 'r {
    methods.iter().map(move |method| {
        let request = build_service_request_match_pattern(method);
        let body = build_service_request_match_arm(self_type, method);

        quote! { #request => #body }
    })
}

fn build_service_request_match_pattern(method: &MethodData) -> TokenStream {
    let name = method.request_name();
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

fn build_service_request_match_arm(self_type: &Type, method: &MethodData) -> TokenStream {
    let method_name = method.name();
    let bindings = method.parameters().iter().map(ParameterData::binding);

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
    let name = method.request_name();
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
