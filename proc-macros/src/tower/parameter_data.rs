use {
    proc_macro2::TokenStream,
    quote::quote,
    syn::{Pat, PatType, Type},
};

/// Representation of a function parameter.
pub struct ParameterData {
    /// The binding pattern of the parameter.
    pattern: Pat,
    /// The type of the parameter.
    parameter_type: Type,
}

impl ParameterData {
    /// Create a new [`ParameterData`] from the [`PatType`] parameter syntax tree.
    pub fn new(parameter: &PatType) -> Self {
        ParameterData {
            pattern: parameter.pat.as_ref().clone(),
            parameter_type: parameter.ty.as_ref().clone(),
        }
    }

    /// Obtain the declaration for this parameter.
    ///
    /// Contains the binding pattern and the parameter type. This can be used when generating a
    /// matching function parameter or a type field.
    pub fn declaration(&self) -> TokenStream {
        let pattern = &self.pattern;
        let parameter_type = &self.parameter_type;

        quote! { #pattern: #parameter_type }
    }

    /// Obtain the binding used for this parameter.
    ///
    /// The binding can be used to access the parameter value.
    pub fn binding(&self) -> TokenStream {
        let pattern = &self.pattern;

        quote! { #pattern }
    }
}
