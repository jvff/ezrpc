use {
    super::{parameter_data::ParameterData, result_data::ResultData},
    syn::{FnArg, Ident, ImplItemMethod},
};

/// Representation of a method's metadata.
pub struct MethodData {
    /// The name of the method.
    name: Ident,

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
            parameters,
            result,
        }
    }

    /// Retrieve the name of this method.
    pub fn name(&self) -> &Ident {
        &self.name
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
