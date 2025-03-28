// File: `cache_diff_derive/src/lib.rs`

mod parse_field;
mod parse_container;
mod shared;
use proc_macro::TokenStream;
use parse_container::ParseContainer;
use parse_field::ParseField;

// Code
pub(crate) const NAMESPACE: &str = "cache_diff";
pub(crate) const MACRO_NAME: &str = "CacheDiff";

#[proc_macro_derive(CacheDiff, attributes(cache_diff))]
pub fn cache_diff(item: TokenStream)
    -> TokenStream {
    create_cache_diff(item.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

fn create_cache_diff(item: proc_macro2::TokenStream) -> syn::Result<proc_macro2::TokenStream> {
    let ParseContainer {
        ident,
        generics,
        custom,
        fields,
    } = ParseContainer::from_derive_input(&syn::parse2(item)?)?;

    let custom_diff = if let Some(ref custom_fn) = custom {
        quote::quote! {
            let custom_diff = #custom_fn(old, self);
            for diff in &custom_diff {
                differences.push(diff.to_string())
            }
        }
    } else {
        quote::quote! {}
    };

    let mut comparisons = Vec::new();
    for field in fields.iter() {
        let ParseField {
            ident,
            name,
            ignore,
            display,
        } = field;

        if ignore.is_none() {
            comparisons.push(quote::quote! {
                if self.#ident != old.#ident {
                    differences.push(
                        format!("{name} ({old} to {new})",
                            name = #name,
                            old = #display(&old.#ident),
                            new = #display(&self.#ident)
                        )
                    );
                }
            });
        }
    }
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();
    Ok(quote::quote! {
        impl #impl_generics ::cache_diff::CacheDiff for #ident #type_generics #where_clause {
            fn diff(&self, old: &Self) -> ::std::vec::Vec<String> {
                let mut differences = ::std::vec::Vec::new();
                #custom_diff
                #(#comparisons)*
                differences
            }
        }
    })
}

