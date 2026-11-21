//! schematize connectors: catálogo de conectores MCP com prévia e anúncios da casa.
//!
//! FUNDAÇÃO (U1): tipos, catálogo embutido e regras de anúncio — lógica PURA, sem rede.
//! O cliente MCP (`mcp`) e a CLI (`cli`) vêm depois. O segredo é só referenciado por NOME de
//! variável de ambiente; anúncio nunca vai para prompt/contexto de agente (ver `anuncio`).

pub mod anuncio;
pub mod catalogo;
mod catalogo_asender;
pub mod cli;
pub mod conector;
pub mod erro;
pub mod mcp;
pub mod segredo;

#[cfg(test)]
mod anuncio_testes;

pub use anuncio::{
    anuncios, escolher, optout_de_env, render_texto, rotulo, Anuncio, Contexto, Data, Idioma, Slot,
};
pub use catalogo::{buscar, catalogo};
pub use conector::{Auth, Conector, FerramentaPrevia, Status, Transporte};
pub use erro::{ErroConector, ErroSegredo};
pub use segredo::{mascarar, resolver_env, Segredo};
