use {
    super::{method_data::MethodData, result_data::ResultData},
    proc_macro2::TokenStream,
    proc_macro_error::abort,
};

/// Representation of the RPC response type.
#[derive(Clone)]
pub struct ResponseData {
    result: ResultData,
}

impl ResponseData {
    /// Create a new [`ResponseData`] from the list of RPC methods.
    pub fn new(methods: &[MethodData]) -> Self {
        let result = methods
            .iter()
            .map(MethodData::result)
            .reduce(Self::merge_method_results)
            .cloned()
            .expect("Empty slice of methods in `ResponseData::new`");

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

    /// Merge the [`ResultData`] from two methods.
    fn merge_method_results<'r>(
        first_result: &'r ResultData,
        second_result: &'r ResultData,
    ) -> &'r ResultData {
        match (&first_result, second_result) {
            (ResultData::Result { ok_type, .. }, ResultData::NotResult(second))
                if ok_type == second =>
            {
                first_result
            }
            (ResultData::NotResult(first), ResultData::Result { ok_type, .. })
                if ok_type == first =>
            {
                second_result
            }
            (first, second) if *first == second => first_result,
            _ => abort!(second_result, "Incompatible method return type"),
        }
    }
}
