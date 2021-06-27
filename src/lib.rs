mod tower;

use {
    crate::tower::MethodData,
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
    let request_calls = methods
        .iter()
        .map(|method| method.request_match_arm(self_type));

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

fn build_service_methods(methods: &[MethodData]) -> TokenStream {
    let service_methods = methods.iter().map(MethodData::service_method);

    quote! {
        impl Service {
            #( #service_methods )*
        }
    }
}
