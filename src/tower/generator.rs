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
}
