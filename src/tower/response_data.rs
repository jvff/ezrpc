use {
    super::{method_data::MethodData, result_data::ResultData},
    either::Either,
    proc_macro2::TokenStream,
    quote::{quote, ToTokens},
    syn::{parse_quote, Ident, Type},
};

/// Representation of the RPC response type.
#[derive(Clone)]
pub enum ResponseData {
    Shared(ResultData),

    DisjointWithSharedError {
        outputs: Vec<(Ident, Box<Type>)>,
        error: Box<Type>,
    },

    FullyDisjoint(Vec<(Ident, ResultData)>),
}

impl ResponseData {
    /// Create a new [`ResponseData`] from the list of RPC methods.
    pub fn new(methods: &[MethodData]) -> Self {
        let method_results = methods.iter().map(MethodData::result);
        let response_names = methods.iter().map(MethodData::request_name).cloned();
        let method_ok_types = method_results.clone().map(ResultData::ok_type).cloned();
        let method_err_types = method_results.clone().filter_map(ResultData::err_type);

        if let Some(error) = Self::common_shared_error(method_err_types) {
            if let Some(result) = Self::common_shared_result(method_results) {
                ResponseData::Shared(result)
            } else {
                let outputs = response_names.zip(method_ok_types).collect();

                ResponseData::DisjointWithSharedError { outputs, error }
            }
        } else {
            let results = response_names.zip(method_results.cloned()).collect();

            ResponseData::FullyDisjoint(results)
        }
    }

    /// Generate the code for declaring the [`Response`] type, if necessary.
    pub fn response_type_declaration(&self) -> TokenStream {
        let variants = match self {
            ResponseData::Shared(_) => return quote! {},
            ResponseData::DisjointWithSharedError { outputs, .. } => {
                Either::Left(outputs.iter().map(|(variant_name, output_type)| {
                    quote! { #variant_name ( #output_type ) }
                }))
            }
            ResponseData::FullyDisjoint(results) => {
                Either::Right(results.iter().map(|(variant_name, result_type)| {
                    quote! { #variant_name ( #result_type ) }
                }))
            }
        };

        quote! {
            pub enum Response {
                #( #variants ),*
            }
        }
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
        match self {
            ResponseData::Shared(_) => method.result().conversion_to_result(expression),
            ResponseData::DisjointWithSharedError { .. } => {
                let variant = method.request_name();
                let expression_result = method.result().conversion_to_result(expression);

                quote! { #expression_result.map(Response::#variant) }
            }
            ResponseData::FullyDisjoint(_) => {
                let variant = method.request_name();

                quote! { Ok(Response::#variant(#expression)) }
            }
        }
    }

    /// Return the [`Ok`][Result::Ok] type that's expected from the RPC call.
    pub fn ok_type(&self) -> TokenStream {
        match self {
            ResponseData::Shared(result_data) => result_data.ok_type().to_token_stream(),
            ResponseData::DisjointWithSharedError { .. } | ResponseData::FullyDisjoint(_) => {
                quote! { Response }
            }
        }
    }

    /// Return the [`Err`][Result::Err] type that's expected from the RPC call.
    pub fn err_type(&self) -> TokenStream {
        match self {
            ResponseData::Shared(result_data) => result_data
                .err_type()
                .map(ToTokens::to_token_stream)
                .unwrap_or_else(|| quote! { () }),
            ResponseData::DisjointWithSharedError { error, .. } => quote! { #error },
            ResponseData::FullyDisjoint { .. } => quote! { () },
        }
    }

    /// Figure out if all methods share a common error type.
    fn common_shared_error<'e>(
        mut error_types: impl Iterator<Item = &'e Box<Type>>,
    ) -> Option<Box<Type>> {
        let first_error_type = match error_types.next() {
            Some(error_type) => error_type,
            None => return Some(Box::new(parse_quote! { () })),
        };

        error_types
            .all(|error_type| error_type == first_error_type)
            .then(|| first_error_type.clone())
    }

    /// Figure out if all methods share a common `Result` type.
    fn common_shared_result<'r>(
        mut result_data: impl Iterator<Item = &'r ResultData>,
    ) -> Option<ResultData> {
        let first_result_data = result_data
            .next()
            .expect("Empty list of `ResultData` used to determine shared result");

        result_data
            .try_fold(
                first_result_data,
                |current_result_data, next_result_data| {
                    if current_result_data == next_result_data {
                        Some(current_result_data)
                    } else if current_result_data.ok_type() != next_result_data.ok_type() {
                        None
                    } else if current_result_data.err_type().is_none() {
                        Some(next_result_data)
                    } else if next_result_data.err_type().is_none() {
                        Some(current_result_data)
                    } else {
                        None
                    }
                },
            )
            .cloned()
    }
}
