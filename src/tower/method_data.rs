use {
    super::{parameter_data::ParameterData, result_data::ResultData},
    heck::CamelCase,
    syn::{FnArg, Ident, ImplItemMethod},
};

/// Representation of a method's metadata.
pub struct MethodData {
    /// The name of the method.
    name: Ident,

    /// The name of the generated `Request` variant.
    ///
    /// This is equivalent to the name of the method converted to CamelCase.
    request_name: Ident,

    /// The parameters of the method.
    ///
    /// This does not include the receiver parameter (i.e., `&mut self`).
    parameters: Vec<ParameterData>,

    /// The resulting output of the method.
    result: ResultData,
}

impl MethodData {
    /// Create a new [`MethodData`] by parsing an [`ImplItemMethod`] syntax tree.
    pub fn new(method: &ImplItemMethod) -> Self {
        let name = method.sig.ident.clone();
        let request_name_string = name.to_string().to_camel_case();
        let request_name = Ident::new(&request_name_string, name.span());

        let parameters = method
            .sig
            .inputs
            .iter()
            .filter_map(|argument| match argument {
                FnArg::Receiver(_) => None,
                FnArg::Typed(parameter) => Some(parameter),
            })
            .map(ParameterData::new)
            .collect();

        let result = ResultData::new(&method.sig.output);

        MethodData {
            name,
            request_name,
            parameters,
            result,
        }
    }

    /// Retrieve the name of this method.
    pub fn name(&self) -> &Ident {
        &self.name
    }

    /// Retrieve the name of the generated `Request` variant respective to this method.
    ///
    /// This is equivalent to the name of the method converted to CamelCase.
    pub fn request_name(&self) -> &Ident {
        &self.request_name
    }

    /// Retrieve the list of [`ParameterData`] for this method's parameters.
    ///
    /// Does not include the receiver type (e.g. `&mut self`).
    pub fn parameters(&self) -> &Vec<ParameterData> {
        &self.parameters
    }

    /// Retrieve the [`ResultData`] of this method.
    pub fn result(&self) -> &ResultData {
        &self.result
    }
}
