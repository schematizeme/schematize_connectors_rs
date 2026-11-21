//! Erros do catálogo de conectores.

use std::fmt;

/// Falha de validação de um [`crate::Conector`] ou de um anúncio.
///
/// O quê: enumera por que um endpoint/link/entrada foi recusado (sem carregar segredo).
/// Onde: devolvido por `Conector::validar` e `Anuncio::validar`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErroConector {
    /// URL não começa com `https://`.
    NaoHttps,
    /// URL com caractere proibido (controle, espaço, barra invertida, não-ASCII).
    CaractereInvalido,
    /// URL com credencial embutida (`user@host`).
    Credencial,
    /// Porta diferente de 443.
    Porta,
    /// Query ou fragmento na URL.
    QueryOuFragmento,
    /// Host malformado (vazio, rótulo vazio, ponto final).
    HostMalformado,
    /// Host fora do domínio do conector (lookalike / SSRF).
    HostForaDoDominio,
    /// Combinação incoerente de status/endpoint/auth.
    Incoerente(&'static str),
}

impl fmt::Display for ErroConector {
    /// O quê: mensagem humana do erro. Onde: logs e CLI.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NaoHttps => f.write_str("a URL precisa começar com https://"),
            Self::CaractereInvalido => f.write_str("a URL tem caractere proibido"),
            Self::Credencial => f.write_str("a URL não pode carregar credencial"),
            Self::Porta => f.write_str("só a porta 443 é aceita"),
            Self::QueryOuFragmento => f.write_str("a URL não pode ter query nem fragmento"),
            Self::HostMalformado => f.write_str("host malformado"),
            Self::HostForaDoDominio => f.write_str("o host não pertence ao domínio do conector"),
            Self::Incoerente(m) => write!(f, "entrada incoerente: {m}"),
        }
    }
}

impl std::error::Error for ErroConector {}

/// Falha ao obter um segredo do ambiente. Nunca carrega o valor.
///
/// O quê: diz qual variável faltou/estava inválida (só o NOME).
/// Onde: devolvido por `resolver_env`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErroSegredo {
    /// Variável não definida.
    Ausente(String),
    /// Variável definida mas vazia.
    Vazio(String),
    /// Valor não é Unicode válido.
    NaoUnicode(String),
}

impl fmt::Display for ErroSegredo {
    /// O quê: mensagem acionável, citando só o nome da variável. Onde: CLI e logs.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ausente(v) => write!(f, "defina a variável de ambiente {v} com a sua chave"),
            Self::Vazio(v) => write!(f, "a variável de ambiente {v} está vazia"),
            Self::NaoUnicode(v) => write!(f, "a variável de ambiente {v} não é texto válido"),
        }
    }
}

impl std::error::Error for ErroSegredo {}
