use {
    heck::CamelCase,
    proc_macro::TokenStream as RawTokenStream,
    proc_macro2::TokenStream,
    quote::quote,
    syn::{parse_macro_input, FnArg, Ident, ImplItem, ImplItemMethod, ItemImpl},
};

#[proc_macro_attribute]
pub fn tower(_attribute: RawTokenStream, item_tokens: RawTokenStream) -> RawTokenStream {
    let item = parse_macro_input!(item_tokens as ItemImpl);
    let request = build_request(&item);

    RawTokenStream::from(quote! {
        #request
        #item
    })
}

fn build_request(item: &ItemImpl) -> TokenStream {
    let variants = item.items.iter().filter_map(|item| match item {
        ImplItem::Method(method) => Some(build_request_variant(method)),
        _ => None,
    });

    quote! {
        pub enum Request {
            #( #variants ),*
        }
    }
}

fn build_request_variant(method: &ImplItemMethod) -> TokenStream {
    let name_string = method.sig.ident.to_string().to_camel_case();
    let name = Ident::new(&name_string, method.sig.ident.span());

    if has_parameters(method) {
        let parameters = build_request_variant_parameters(method);

        quote! {
            #name {
                #( #parameters ),*
            }
        }
    } else {
        quote! { #name }
    }
}

fn has_parameters(method: &ImplItemMethod) -> bool {
    let inputs = &method.sig.inputs;

    if inputs.is_empty() {
        false
    } else if inputs.len() > 1 {
        true
    } else {
        !matches!(inputs.first(), Some(FnArg::Receiver(_)))
    }
}

fn build_request_variant_parameters(
    method: &ImplItemMethod,
) -> impl Iterator<Item = TokenStream> + '_ {
    method
        .sig
        .inputs
        .iter()
        .filter_map(|argument| match argument {
            FnArg::Receiver(_) => None,
            FnArg::Typed(argument) => Some(argument),
        })
        .map(|argument| {
            let pattern = &argument.pat;
            let argument_type = &argument.ty;

            quote! { #pattern: #argument_type }
        })
}
