mod tower;

use {
    crate::tower::{Generator, MethodData},
    proc_macro::TokenStream as RawTokenStream,
    proc_macro2::TokenStream,
    quote::quote,
    syn::{parse_macro_input, ItemImpl},
};

#[proc_macro_attribute]
pub fn tower(_attribute: RawTokenStream, item_tokens: RawTokenStream) -> RawTokenStream {
    let item = parse_macro_input!(item_tokens as ItemImpl);
    let generator = Generator::new(&item);
    let request = generator.request();
    let service = build_service(&generator);

    RawTokenStream::from(quote! {
        #request
        #item
        #service
    })
}

fn build_service(generator: &Generator) -> TokenStream {
    let service_impl = build_service_impl(generator);
    let service_methods = build_service_methods(generator);

    quote! {
        pub struct Service;

        #service_impl
        #service_methods
    }
}

fn build_service_impl(generator: &Generator) -> TokenStream {
    let result_data = generator.result();
    let response = result_data.ok_type();
    let error = result_data.err_type();
    let request_calls = generator
        .methods()
        .iter()
        .map(|method| method.request_match_arm(generator.self_type()));

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

fn build_service_methods(generator: &Generator) -> TokenStream {
    let service_methods = generator.methods().iter().map(MethodData::service_method);

    quote! {
        impl Service {
            #( #service_methods )*
        }
    }
}
