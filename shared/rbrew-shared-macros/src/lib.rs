use proc_macro::TokenStream;

mod iotype;

#[proc_macro]
pub fn iotype(ts: TokenStream) -> TokenStream {
    iotype::iotype2(ts)
}
