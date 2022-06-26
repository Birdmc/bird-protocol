use syn::{DataEnum, Fields};

pub fn is_c_enum(data: &DataEnum) -> bool {
    data.variants
        .iter()
        .all(|variant| match variant.fields {
            Fields::Unit => true,
            _ => false,
        })
}