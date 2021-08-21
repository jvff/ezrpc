use {
    super::{method_data::MethodData, result_data::ResultData},
    proc_macro2::TokenStream,
    proc_macro_error::abort,
    quote::{quote, ToTokens},
};

/// Representation of the RPC response type.
#[derive(Clone)]
pub struct ResponseData {
    result: ResultData,
}

impl ResponseData {
    /// Create a new [`ResponseData`] from the list of RPC methods.
    pub fn new(methods: &[MethodData]) -> Self {
        let method_results = methods.iter().map(MethodData::result);

        match Self::common_shared_result(method_results) {
            Ok(result) => ResponseData { result },
            Err(incompatible_type) => abort!(incompatible_type, "Incompatible method return type"),
        }
    }

    /// Generate the code for declaring the [`Response`] type, if necessary.
    pub fn response_type_declaration(&self) -> TokenStream {
        quote! {}
    }

    /// Generate the conversion of a method's return type into the response type.
    ///
    /// Wraps the provided `expression` that results in the return type of the `method` into the
    /// shared response type represented by this [`ResponseData`].
    pub fn conversion_to_response(
        &self,
        method: &MethodData,
        expression: TokenStream,
    ) -> TokenStream {
        method.result().conversion_to_result(expression)
    }

    /// Return the [`Ok`][Result::Ok] type that's expected from the RPC call.
    pub fn ok_type(&self) -> TokenStream {
        self.result.ok_type().to_token_stream()
    }

    /// Return the [`Err`][Result::Err] type that's expected from the RPC call.
    pub fn err_type(&self) -> TokenStream {
        self.result
            .err_type()
            .map(ToTokens::to_token_stream)
            .unwrap_or_else(|| quote! { () })
    }

    /// Figure out if all methods share a common `Result` type.
    fn common_shared_result<'r>(
        mut result_data: impl Iterator<Item = &'r ResultData>,
    ) -> Result<ResultData, ResultData> {
        let first_result_data = result_data
            .next()
            .expect("Empty list of `ResultData` used to determine shared result");

        result_data
            .try_fold(
                first_result_data,
                |current_result_data, next_result_data| {
                    if current_result_data == next_result_data {
                        Ok(current_result_data)
                    } else if current_result_data.ok_type() != next_result_data.ok_type() {
                        Err(next_result_data)
                    } else if current_result_data.err_type().is_none() {
                        Ok(next_result_data)
                    } else if next_result_data.err_type().is_none() {
                        Ok(current_result_data)
                    } else {
                        Err(next_result_data)
                    }
                },
            )
            .map(|ok| ok.clone())
            .map_err(|err| err.clone())
    }
}
