//! Cliente MCP de PRÉVIA (somente leitura): `initialize` + `tools/list`, nunca `tools/call`.
//!
//! O quê: contrato público em `tipos`; prévia ao vivo em `viva` sobre um transporte plugável.
//! Onde: consumido pela CLI (`src/cli/`) para mostrar as ferramentas de um conector.

mod tipos;
mod viva;

pub use tipos::{ErroMcp, FerramentaViva, Origem, Previa, PreviaEstatica, PreviaViva, Previador};
