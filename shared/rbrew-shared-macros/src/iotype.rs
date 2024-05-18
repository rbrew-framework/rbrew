use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{format_ident, quote};
use syn::{parse::Parse, punctuated::Punctuated, Attribute, Ident, Visibility};

#[derive(Clone, Copy)]
enum Primitive {
    U8,
    U16,
    U32,
    U64,
}

impl Primitive {
    fn as_ty(self) -> proc_macro2::TokenStream {
        match self {
            Primitive::U8 => quote!(u8),
            Primitive::U16 => quote!(u16),
            Primitive::U32 => quote!(u32),
            Primitive::U64 => quote!(u64),
        }
    }

    fn align(self) -> u64 {
        match self {
            Primitive::U8 => 1,
            Primitive::U16 => 2,
            Primitive::U32 => 4,
            Primitive::U64 => 8,
        }
    }
}

struct IoField {
    ident: Ident,
    writable: bool,
    primitive: Primitive,
    offset: u64,
}

impl Parse for IoField {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let ident = input.parse()?;
        input.parse::<syn::Token![:]>()?;
        let writable = if input.parse::<syn::Token![mut]>().is_ok() {
            true
        } else if input.parse::<syn::Token![const]>().is_ok() {
            false
        } else {
            return Err(input.error("expected either 'mut' or 'const' before a field type."));
        };
        let primitive = match input.parse::<Ident>()?.to_string().as_str() {
            "u8" => Primitive::U8,
            "u16" => Primitive::U16,
            "u32" => Primitive::U32,
            "u64" => Primitive::U64,
            ty => {
                return Err(input.error(format!(
                    "expected either 'u8', 'u16', 'u32' or 'u64'. Got '{ty}'"
                )))
            }
        };
        input.parse::<syn::Token![=]>()?;
        let offset: syn::LitInt = input.parse()?;
        Ok(Self {
            ident,
            writable,
            primitive,
            offset: offset
                .base10_parse()
                .expect("unable to cast offset to a u64"),
        })
    }
}

struct IoTypeItem {
    attrs: Vec<Attribute>,
    vis: Visibility,
    ident: Ident,
    base_adr: u64,
    len: u64,
    body: Punctuated<IoField, syn::Token![,]>,
}

impl Parse for IoTypeItem {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let attrs = Attribute::parse_outer(input)?;
        let vis = input.parse()?;
        input.parse::<syn::Token![type]>()?;
        let ident = input.parse()?;
        input.parse::<syn::Token![:]>()?;
        let base_adr: syn::LitInt = input.parse()?;
        input.parse::<syn::Token![,]>()?;
        let len: syn::LitInt = input.parse()?;
        let content;
        syn::braced!(content in input);
        let body = Punctuated::parse_terminated(&content)?;
        Ok(Self {
            attrs,
            vis,
            ident,
            base_adr: base_adr
                .base10_parse()
                .expect("failed to convert base address to u64"),
            len: len.base10_parse().expect("failed to convert length to u64"),
            body,
        })
    }
}

pub fn iotype2(ts: TokenStream) -> TokenStream {
    let IoTypeItem {
        attrs,
        vis,
        ident,
        base_adr,
        len,
        body,
    }: IoTypeItem = syn::parse(ts).expect("failed to parse IO item");

    let base_adr_lit = syn::LitInt::new(&base_adr.to_string(), Span::mixed_site());
    let len_lit = syn::LitInt::new(&len.to_string(), Span::mixed_site());

    let fns = body.iter().map(
        |IoField {
             ident,
             writable,
             offset,
             primitive,
         }| {
            let offset_lit = syn::LitInt::new(&offset.to_string(), Span::mixed_site());
            let ty = primitive.as_ty();

            let align = primitive.align();
            assert!(
                (base_adr + offset) & (align - 1) == 0,
                "unaligned IO register"
            );

            let write_fn = if *writable {
                let write_ident = format_ident!("{}_write", ident);
                quote! {
                    #[inline(always)]
                    pub unsafe fn #write_ident(value: #ty) {
                        Self::#ident().write_volatile(value)
                    }
                }
            } else {
                quote!()
            };

            let read_fn = {
                let read_ident = format_ident!("{}_read", ident);
                quote! {
                    #[inline(always)]
                    pub unsafe fn #read_ident() -> #ty {
                        Self::#ident().read_volatile()
                    }
                }
            };

            let ptr_fn = {
                let ptr_ty = if *writable {
                    quote!(*mut #ty)
                } else {
                    quote!(*const #ty)
                };
                let ptr_ident = format_ident!("{}_ptr", ident);
                quote! {
                    #[inline(always)]
                    pub fn #ptr_ident() -> #ptr_ty {
                        (#base_adr_lit + #offset_lit) as *mut _
                    }
                }
            };

            quote!(
                #ptr_fn
                #read_fn
                #write_fn
            )
        },
    );

    quote! {
        #(#attrs)*
        #vis enum #ident {}

        impl #ident {
            pub const BASE: usize = #base_adr_lit;
            pub const LEN: usize = #len_lit;

            pub fn ptr() -> *mut () {
                Self::BASE as *mut _
            }

            #(#fns)*
        }
    }
    .into()
}
