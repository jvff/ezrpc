mod tower;

use {
    crate::tower::Generator,
    proc_macro::TokenStream as RawTokenStream,
    quote::quote,
    syn::{parse_macro_input, ItemImpl},
};

#[proc_macro_attribute]
pub fn tower(_attribute: RawTokenStream, item_tokens: RawTokenStream) -> RawTokenStream {
    let item = parse_macro_input!(item_tokens as ItemImpl);
    let generator = Generator::new(&item);
    let request = generator.request();
    let service = generator.service();

    RawTokenStream::from(quote! {
        #request
        #item
        #service
    })
}
