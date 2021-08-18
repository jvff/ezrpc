use {
    proc_macro2::TokenStream,
    quote::{quote, ToTokens},
    syn::{parse_quote, GenericArgument, Path, PathArguments, ReturnType, Type},
};

/// Representation of a function's return type as a result.
#[derive(Clone, Eq, PartialEq)]
pub enum ResultData {
    /// The return type is not a [`Result`].
    NotResult(Box<Type>),

    /// The return type is a [`Result`].
    ///
    /// The [`Ok`][Result::Ok] and [`Err`][Result::Err] types are extracted and stored separately.
    Result {
        ok_type: Box<Type>,
        err_type: Box<Type>,
    },
}

impl ResultData {
    /// Create a [`ResultData`] from a function's [`ReturnType`].
    ///
    /// Parses the function's return type to try to extract either a [`Result`] type or a non-result
    /// type. If a [`Result`] type is found (either named `Result` or `std::result::Result`), the
    /// [`Ok`][Result::Ok] and [`Err`][Result::Err] types are extracted.
    ///
    /// For function's that have no return type, the type is set to [`()`].
    pub fn new(return_type: &ReturnType) -> Self {
        match return_type {
            ReturnType::Default => ResultData::NotResult(Box::new(parse_quote! { () })),
            ReturnType::Type(_, actual_return_type) => {
                Self::parse_actual_return_type(actual_return_type)
            }
        }
    }

    /// Creates the [`ResultData`] from the extracted [`Type`].
    fn parse_actual_return_type(return_type: &Type) -> Self {
        match return_type {
            Type::Path(path_type) if path_type.qself.is_none() => {
                Self::extract_result_type(&path_type.path)
                    .unwrap_or_else(|| ResultData::NotResult(Box::new(return_type.clone())))
            }
            other => ResultData::NotResult(Box::new(other.clone())),
        }
    }

    /// Attempts to create the [`ResultData`] from the extracted type's [`Path`].
    fn extract_result_type(path: &Path) -> Option<Self> {
        let type_arguments = Self::extract_result_type_arguments(path)?;

        let generic_types = match type_arguments {
            PathArguments::AngleBracketed(arguments) => &arguments.args,
            _ => return None,
        };

        if generic_types.len() != 2 {
            return None;
        }

        let ok_type = match &generic_types[0] {
            GenericArgument::Type(ok_type) => ok_type.clone(),
            _ => return None,
        };

        let err_type = match &generic_types[1] {
            GenericArgument::Type(err_type) => err_type.clone(),
            _ => return None,
        };

        Some(ResultData::Result {
            ok_type: Box::new(ok_type),
            err_type: Box::new(err_type),
        })
    }

    /// Attempts to extract the type arguments inside a [`Path`] that is either `Result` or
    /// `std::result::Result`.
    fn extract_result_type_arguments(path: &Path) -> Option<&PathArguments> {
        let mut segments = path.segments.iter();
        let first_segment = segments.next()?;

        if first_segment.ident == "Result" && path.leading_colon.is_none() {
            Some(&first_segment.arguments)
        } else {
            let second_segment = segments.next()?;
            let third_segment = segments.next()?;

            if first_segment.ident == "std"
                && second_segment.ident == "result"
                && third_segment.ident == "Result"
            {
                Some(&third_segment.arguments)
            } else {
                None
            }
        }
    }

    /// Returns the [`Ok`][Result::Ok] type, or the bare return type if it's not a [`Result`] type.
    pub fn ok_type(&self) -> &Box<Type> {
        match self {
            ResultData::NotResult(return_type) => return_type,
            ResultData::Result { ok_type, .. } => ok_type,
        }
    }

    /// Returns the [`Err`][Result::Err] type if the return type is a [`Result`] type.
    pub fn err_type(&self) -> Option<&Box<Type>> {
        match self {
            ResultData::NotResult(_) => None,
            ResultData::Result { err_type, .. } => Some(err_type),
        }
    }

    /// Returns the code to convert an expression that results in an instance of this
    /// [`ResultData`] type into a [`Result`].
    ///
    /// The conversion is either simple the expression or the expression wrapped inside an
    /// [`Ok`][Result::Ok] variant.
    pub fn conversion_to_result(&self, expression: TokenStream) -> TokenStream {
        match self {
            ResultData::NotResult(_) => quote! { Ok(#expression) },
            ResultData::Result { .. } => expression,
        }
    }

    /// Returns the code to convert a [`Result`] into this [`ResultData`] type.
    ///
    /// If this [`ResultData`] is a [`ResultData::NotResult`], then the generated code `unwrap`s
    /// the [`Result`], so it may panic.
    pub fn conversion_from_result(&self) -> TokenStream {
        match self {
            ResultData::NotResult(_) => quote! { .expect("Result data never fails") },
            ResultData::Result { .. } => quote! {},
        }
    }
}

impl ToTokens for ResultData {
    fn to_tokens(&self, token_stream: &mut TokenStream) {
        match self {
            ResultData::NotResult(return_type) => return_type.to_tokens(token_stream),
            ResultData::Result { ok_type, err_type } => {
                let result = quote! { ::std::result::Result<#ok_type, #err_type> };

                result.to_tokens(token_stream)
            }
        }
    }
}
