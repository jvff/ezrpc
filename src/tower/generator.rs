use {
    super::{method_data::MethodData, receiver_type::ReceiverType, response_data::ResponseData},
    proc_macro2::TokenStream,
    proc_macro_error::abort,
    quote::quote,
    syn::{ImplItem, ItemImpl, Type},
};

/// Code generator for a [`tower::Service`] RPC.
pub struct Generator {
    /// The type the `impl` block is for.
    self_type: Type,

    /// The meta-data for each method in the `impl` block.
    methods: Vec<MethodData>,

    /// The common response type sent by the RPC calls.
    response: ResponseData,

    /// The most strict method receiver type.
    receiver_type: ReceiverType,
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

        if methods.is_empty() {
            abort!(item, "`impl` item has no methods");
        }

        let response = ResponseData::new(&methods);

        let receiver_type = methods
            .iter()
            .map(MethodData::receiver_type)
            .max()
            .expect("There is at least one method");

        Generator {
            self_type,
            methods,
            response,
            receiver_type,
        }
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
        let service_data = self.service_data();
        let service_impl = self.service_impl();
        let service_methods = self.methods.iter().map(MethodData::service_method);

        quote! {
            pub struct Service #service_data;

            impl Service {
                #( #service_methods )*
            }

            #service_impl
        }
    }

    /// Generate the inner field inside the `Service` type.
    ///
    /// This contains a shared reference to the instance that implements the method behaviour. It
    /// is used for methods that require a `&self` or a `&mut self` receiver. The instance is
    /// wrapped in an `Arc`, because the `Service` type may live less than the response
    /// [`Future`][std::future::Future] it returns. If there's at least one method that uses a
    /// `&mut self` receiver, then the instance is also wrapped inside a
    /// [`RwLock`][tokio::sync::RwLock], to avoid concurrent access to it.
    fn service_data(&self) -> TokenStream {
        let self_type = &self.self_type;

        match self.receiver_type {
            ReceiverType::NoReceiver => quote! {},
            ReceiverType::Reference => quote! { (std::sync::Arc<#self_type>) },
            ReceiverType::MutableReference => {
                quote! { (std::sync::Arc<tokio::sync::RwLock<#self_type>>) }
            }
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
        let response = self.response.ok_type();
        let error = self.response.err_type();

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
                    use futures::FutureExt as _;

                    async move {
                        match request {
                            #( #request_match_arms ),*
                        }
                    }.boxed()
                }
            }
        }
    }
}
