extern crate proc_macro;

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::Data::Enum;
use syn::DeriveInput;

/// Implement the `std::fmt::Display` trait for the given enum. Only supports enums which have only
/// fieldless variants.
#[proc_macro_derive(EnumDisplay)]
pub fn enum_display(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).expect("unable to parse input");

    let name = &ast.ident;

    let Enum(data) = &ast.data
    else {
        panic!("EnumDisplay derive macro can only be applied to enums; {name} is not an enum")
    };

    let match_arms: Vec<_> = data
        .variants
        .iter()
        .map(|variant| {
            let variant_name = &variant.ident;
            if !variant.fields.is_empty() {
                panic!("EnumDisplay macro only supports enums with only fieldless variants; {name}::{variant_name} has fields");
            }

            let variant_name_str = variant_name.to_string();
            quote! {
                Self::#variant_name => write!(f, #variant_name_str)
            }
        })
        .collect();

    let gen = quote! {
        impl std::fmt::Display for #name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    #(#match_arms,)*
                }
            }
        }
    };

    gen.into()
}

/// Implement the `std::str::FromStr` trait for the given enum, with `FromStr::Err` set to `String`.
/// Only supports enums which have only fieldless variants. The generated implementation will be
/// case-insensitive.
#[proc_macro_derive(EnumFromStr)]
pub fn enum_from_str(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).expect("unable to parse input");

    let name = &ast.ident;

    let Enum(data) = &ast.data
    else {
        panic!("EnumFromStr derive macro can only be applied to enums; {name} is not an enum");
    };

    let match_arms: Vec<_> = data
        .variants
        .iter()
        .map(|variant| {
            let variant_name = &variant.ident;
            if !variant.fields.is_empty() {
                panic!("EnumFromStr macro only supports enums with only fieldless variants; {name}::{variant_name} has fields");
            }

            let variant_name_lowercase = variant_name.to_string().to_ascii_lowercase();
            quote! {
                #variant_name_lowercase => Ok(Self::#variant_name)
            }
        })
        .collect();

    let err_fmt_string = format!("invalid {name} string: '{{}}'");
    let gen = quote! {
        impl std::str::FromStr for #name {
            type Err = String;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                match s.to_ascii_lowercase().as_str() {
                    #(#match_arms,)*
                    _ => Err(format!(#err_fmt_string, s))
                }
            }
        }
    };

    gen.into()
}

/// Implement the `serde::Serialize` trait for the given type, serializing values as strings. This
/// requires that the type implements the `std::fmt::Display` trait.
#[proc_macro_derive(StrSerialize)]
pub fn str_serialize(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).expect("unable to parse input");

    let ident = &ast.ident;

    let gen = quote! {
        impl serde::Serialize for #ident {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                serializer.serialize_str(&self.to_string())
            }
        }
    };

    gen.into()
}

/// Implement the `serde::Deserialize` trait for the given type, deserializing values from strings.
/// This requires that the type implements the `std::str::FromStr` trait.
#[proc_macro_derive(StrDeserialize)]
pub fn str_deserialize(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).expect("unable to parse input");

    let ident = &ast.ident;

    let visitor_struct_name = format_ident!("__{}VisitorGenerated", ident);
    let expecting_fmt_string = format!("a string representing a {ident}");
    let gen = quote! {
        struct #visitor_struct_name;

        impl<'de> serde::de::Visitor<'de> for #visitor_struct_name {
            type Value = #ident;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(formatter, #expecting_fmt_string)
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                let input = v.parse().map_err(serde::de::Error::custom)?;
                Ok(input)
            }
        }

        impl<'de> serde::Deserialize<'de> for #ident {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                deserializer.deserialize_str(#visitor_struct_name)
            }
        }
    };

    gen.into()
}
