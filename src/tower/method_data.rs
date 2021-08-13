use {
    super::{parameter_data::ParameterData, result_data::ResultData},
    heck::CamelCase,
    proc_macro2::TokenStream,
    quote::quote,
    syn::{FnArg, Ident, ImplItemMethod, Type},
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

    /// Retrieve the [`ResultData`] of this method.
    pub fn result(&self) -> &ResultData {
        &self.result
    }

    /// Generate the declaration of the `Request` enum variant related to this method.
    pub fn request_enum_variant(&self) -> TokenStream {
        let name = &self.request_name;

        if self.parameters.is_empty() {
            quote! { #name }
        } else {
            let parameters = self.parameters.iter().map(ParameterData::declaration);

            quote! {
                #name {
                    #( #parameters ),*
                }
            }
        }
    }

    /// Generate the dispatching code for this method.
    ///
    /// Consists af the match arm on the `Request` enum variant for this method, and a call to the
    /// proper implementation method.
    pub fn request_match_arm(&self, self_type: &Type) -> TokenStream {
        let request_name = &self.request_name;
        let method_call = self.method_call(self_type);

        if self.parameters.is_empty() {
            quote! {
                Request::#request_name => #method_call.boxed()
            }
        } else {
            let bindings = self.parameters.iter().map(ParameterData::binding);

            quote! {
                Request::#request_name { #( #bindings ),* } => #method_call.boxed()
            }
        }
    }

    /// Generate the code that calls this method.
    fn method_call(&self, self_type: &Type) -> TokenStream {
        let method_name = &self.name;
        let output_conversion = self.result.future_output_conversion();

        if self.parameters.is_empty() {
            quote! { #self_type::#method_name() #output_conversion }
        } else {
            let arguments = self.parameters.iter().map(ParameterData::binding);

            quote! { #self_type::#method_name( #( #arguments ),* ) #output_conversion }
        }
    }

    /// Generate a helper method to create and send the `Request` to call this method's
    /// implementation.
    pub fn service_method(&self) -> TokenStream {
        let method_name = &self.name;
        let parameters = self.parameters.iter().map(ParameterData::declaration);
        let result = &self.result;
        let request = self.request_construction();
        let response_conversion = self.result.conversion_from_result();

        quote! {
            pub async fn #method_name(&mut self, #( #parameters ),*) -> #result {
                use tower::{Service as _, ServiceExt as _};

                let service = self.ready().await.expect("Generated service is always ready");

                service.call(#request).await #response_conversion
            }
        }
    }

    /// Generate the code to create the `Request` variant for this method.
    fn request_construction(&self) -> TokenStream {
        let name = &self.request_name;

        if self.parameters.is_empty() {
            quote! { Request::#name }
        } else {
            let parameters = self.parameters.iter().map(ParameterData::binding);

            quote! {
                Request::#name {
                    #( #parameters ),*
                }
            }
        }
    }
}
