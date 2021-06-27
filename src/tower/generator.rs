use {
    super::{method_data::MethodData, result_data::ResultData},
    proc_macro2::TokenStream,
    quote::quote,
    syn::{ImplItem, ItemImpl, Type},
};

/// Code generator for a [`tower::Service`] RPC.
pub struct Generator {
    /// The type the `impl` block is for.
    self_type: Type,

    /// The meta-data for each method in the `impl` block.
    methods: Vec<MethodData>,

    /// The common result type returned by the RPC calls.
    result: ResultData,
}

impl Generator {
    /// Create a [`Generator`] after extracting the necessary meta-data from an [`ItemImpl`].
    pub fn new(item: &ItemImpl) -> Self {
        let self_type = item.self_ty.as_ref().clone();

        let methods: Vec<_> = item
            .items
            .iter()
            .filter_map(|item| match item {
                ImplItem::Method(method) => Some(MethodData::new(method)),
                _ => None,
            })
            .collect();

        let result = methods[0].result().clone();

        Generator {
            self_type,
            methods,
            result,
        }
    }

    /// Retrieve the underlying type used in the input `impl` block.
    pub fn self_type(&self) -> &Type {
        &self.self_type
    }

    /// Retrieve the list of method meta-data for all methods in the `impl` block.
    pub fn methods(&self) -> &[MethodData] {
        &self.methods
    }

    /// Retrieve the shared resulting output type.
    pub fn result(&self) -> &ResultData {
        &self.result
    }

    /// Generate the `Request` enum type for sending to the generated [`tower::Service`].
    ///
    /// Contains one variant for each method, in order to determine which method to call.
    pub fn request(&self) -> TokenStream {
        let variants = self.methods.iter().map(MethodData::request_enum_variant);

        quote! {
            pub enum Request {
                #( #variants ),*
            }
        }
    }

    /// Generate the `Service` type and its [`tower::Service`] implementation.
    ///
    /// The `Service` type receives `Request`s and dispatches them to the method implementations in
    /// the input `impl` block.
    pub fn service(&self) -> TokenStream {
        let service_impl = self.service_impl();
        let service_methods = self.methods.iter().map(MethodData::service_method);

        quote! {
            pub struct Service;

            impl Service {
                #( #service_methods )*
            }

            #service_impl
        }
    }

    /// Generate the implementation of the [`tower::Service`] trait for the generated `Service`
    /// type.
    ///
    /// The implementation is a large dispatcher, that calls the methods in the input `impl` block.
    fn service_impl(&self) -> TokenStream {
        let request_match_arms = self
            .methods
            .iter()
            .map(|method| method.request_match_arm(&self.self_type));
        let response = self.result.ok_type();
        let error = self.result.err_type();

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
                        #( #request_match_arms ),*
                    }
                }
            }
        }
    }
}
