use {
    super::{method_data::MethodData, result_data::ResultData},
    proc_macro2::TokenStream,
};

/// Representation of the RPC response type.
#[derive(Clone)]
pub struct ResponseData {
    result: ResultData,
}

impl ResponseData {
    /// Create a new [`ResponseData`] from the list of RPC methods.
    pub fn new(methods: &[MethodData]) -> Self {
        let result = methods[0].result().clone();

        ResponseData { result }
    }

    /// Return the [`Ok`][Result::Ok] type that's expected from the RPC call.
    pub fn ok_type(&self) -> TokenStream {
        self.result.ok_type()
    }

    /// Return the [`Err`][Result::Err] type that's expected from the RPC call.
    pub fn err_type(&self) -> TokenStream {
        self.result.err_type()
    }
}
