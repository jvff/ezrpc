use {
    proc_macro2::TokenStream,
    proc_macro_error::abort,
    quote::quote,
    syn::{FnArg, Type},
};

/// The receiver type of the method.
///
/// Methods with owned receivers aren't represented because they're currently not supported.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ReceiverType {
    /// A method that has no `self` receiver.
    NoReceiver,

    /// A method that has a shared reference `&self` receiver.
    Reference,

    /// A method that has an exclusive mutable reference `&mut self` receiver.
    MutableReference,
}

impl ReceiverType {
    /// Create a new [`ReceiverType`] from a method's argument list.
    pub fn new<'a>(method_arguments: impl IntoIterator<Item = &'a FnArg>) -> Self {
        let maybe_receiver = method_arguments
            .into_iter()
            .filter_map(|argument| match argument {
                FnArg::Receiver(receiver) => Some(receiver),
                FnArg::Typed(_) => None,
            })
            .next();

        if let Some(receiver) = maybe_receiver {
            if receiver.reference.is_none() {
                abort!(receiver, "Methods that take ownership aren't supported");
            }

            match receiver.mutability {
                Some(_) => ReceiverType::MutableReference,
                None => ReceiverType::Reference,
            }
        } else {
            ReceiverType::NoReceiver
        }
    }

    /// Generate the code necessary for calling a method from the generated `Service` type.
    ///
    /// This requires to [`ReceiverType`]s. One that is more strict, for the common receiver type
    /// for the service, and one that can be more relaxed, for the method to be called. It is
    /// assumed that this method will be called on the more strict receiver type and it will
    /// receive the `method_receiver_type` as an extra parameter.
    pub fn service_method_call_prefix(
        &self,
        method_receiver_type: ReceiverType,
        self_type: &Type,
    ) -> TokenStream {
        match (self, method_receiver_type) {
            (
                ReceiverType::NoReceiver | ReceiverType::Reference | ReceiverType::MutableReference,
                ReceiverType::NoReceiver,
            ) => {
                quote! { #self_type:: }
            }

            (ReceiverType::Reference, ReceiverType::Reference) => quote! {
                inner.
            },

            (ReceiverType::MutableReference, ReceiverType::Reference) => quote! {
                inner.read().await.
            },

            (ReceiverType::MutableReference, ReceiverType::MutableReference) => quote! {
                inner.write().await.
            },

            (
                ReceiverType::NoReceiver,
                ReceiverType::Reference | ReceiverType::MutableReference,
            )
            | (ReceiverType::Reference, ReceiverType::MutableReference) => {
                unreachable!(
                    "Service receiver type should always be stricter than method receiver type"
                )
            }
        }
    }
}
