use {
    proc_macro::TokenStream,
    quote::quote,
    syn::{parse_macro_input, ItemImpl},
};

#[proc_macro_attribute]
pub fn tower(_attribute: TokenStream, item_tokens: TokenStream) -> TokenStream {
    let item = parse_macro_input!(item_tokens as ItemImpl);

    TokenStream::from(quote! { #item })
}
