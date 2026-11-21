//! Stub do binário. A CLI real chega na U3 (`src/cli/`).

/// O quê: ponto de entrada provisório; orienta quem rodar o binário antes da CLI existir.
/// Onde: executado por `schematize-connectors`.
fn main() {
    println!("schematize-connectors {} — CLI em construção (U3)", env!("CARGO_PKG_VERSION"));
}
