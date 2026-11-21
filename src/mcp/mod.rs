//! Cliente MCP de PRÉVIA (somente leitura): `initialize` + `tools/list`, nunca invocação.
//!
//! O quê: contrato público em `tipos`; prévia ao vivo em `viva` sobre um transporte plugável
//! (`curl` real em `curl`, Fake nos testes); parse puro em `jsonrpc`.
//! Onde: consumido pela CLI (`src/cli/`) para mostrar as ferramentas de um conector.

mod curl;
mod http;
mod jsonrpc;
mod tipos;
mod viva;

#[cfg(test)]
mod testes_curl;
#[cfg(test)]
mod testes_jsonrpc;
#[cfg(test)]
mod testes_local;
#[cfg(test)]
mod testes_viva;

pub use curl::CurlTransporte;
pub use http::{ReqHttp, RespHttp, Transporte};
pub use jsonrpc::sanitizar;
pub use tipos::{ErroMcp, FerramentaViva, Origem, Previa, PreviaEstatica, PreviaViva, Previador};
