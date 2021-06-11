use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn tower(_attribute: TokenStream, item_tokens: TokenStream) -> TokenStream {
    item_tokens
}
