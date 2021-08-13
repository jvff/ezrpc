use {proc_macro_error::abort, syn::FnArg};

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
}
