mod tower;

use {
    crate::tower::Generator,
    proc_macro::TokenStream,
    quote::quote,
    syn::{parse_macro_input, ItemImpl},
};

#[proc_macro_attribute]
pub fn tower(_attribute: TokenStream, item_tokens: TokenStream) -> TokenStream {
    let item = parse_macro_input!(item_tokens as ItemImpl);
    let generator = Generator::new(&item);
    let request = generator.request();
    let service = generator.service();

    TokenStream::from(quote! {
        #request
        #item
        #service
    })
}
