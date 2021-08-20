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
        let mut method_results = methods.iter().map(MethodData::result);

        let starting_result_data = method_results
            .next()
            .cloned()
            .expect("Empty slice of methods in `ResponseData::new`");
        let starting_response_data = ResponseData {
            result: starting_result_data,
        };

        match method_results.fold(Ok(starting_response_data), Self::fold_method_results) {
            Ok(response_data) => response_data,
            Err(incompatible_type) => abort!(incompatible_type, "Incompatible method return type"),
        }
    }

    /// Generate the code for declaring the [`Response`] type, if necessary.
    pub fn response_type_declaration(&self) -> TokenStream {
        quote! {}
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

    /// Fold the [`ResultData`] from a method into a [`ResponseData`].
    fn fold_method_results(
        current_response: Result<ResponseData, ResultData>,
        method_result: &ResultData,
    ) -> Result<ResponseData, ResultData> {
        let ResponseData { result } = current_response?;

        match (&result, method_result) {
            (ResultData::Result { ok_type, .. }, ResultData::NotResult(second))
                if ok_type == second =>
            {
                Ok(ResponseData { result })
            }
            (ResultData::NotResult(first), ResultData::Result { ok_type, .. })
                if ok_type == first =>
            {
                Ok(ResponseData {
                    result: method_result.clone(),
                })
            }
            (first, second) if first == second => Ok(ResponseData { result }),
            _ => Err(method_result.clone()),
        }
    }
}
