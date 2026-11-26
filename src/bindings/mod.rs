#[cfg(all(feature = "napi", not(target_arch = "wasm32")))]
mod node;
#[cfg(feature = "wasm")]
mod wasm;

fn diagrams(value: Option<&str>) -> Result<crate::Diagrams, String> {
    match value {
        None | Some("pelers") => Ok(crate::Diagrams::Pelers),
        Some("placeholder") => Ok(crate::Diagrams::Placeholder),
        Some(other) => Err(format!(
            "Unknown diagrams option \"{other}\"; expected \"pelers\" or \"placeholder\"."
        )),
    }
}
