//! Tipos públicos da prévia MCP: contrato estável consumido pela CLI (U3).

use crate::conector::Conector;
use crate::segredo::Segredo;
use serde::Serialize;
use std::fmt;

/// De onde veio a prévia.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Origem {
    /// `tools/list` ao vivo no servidor.
    Viva,
    /// Lista embutida no catálogo (sem rede).
    Estatica,
}

/// Ferramenta exibida na prévia (texto já sanitizado quando vem do servidor).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FerramentaViva {
    /// Nome da ferramenta.
    pub nome: String,
    /// Descrição curta (até 300 chars, sem controle/ANSI).
    pub descricao: String,
    /// `true` se escreve/envia algo. Só é `false` com `readOnlyHint == true` (seguro por padrão).
    pub escrita: bool,
}

/// Resultado de uma prévia.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Previa {
    /// Id do conector.
    pub conector_id: String,
    /// Viva ou estática.
    pub origem: Origem,
    /// Versão de protocolo negociada (só viva).
    pub protocolo: Option<String>,
    /// `nome versão` do servidor (só viva, sanitizado).
    pub servidor: Option<String>,
    /// Ferramentas.
    pub ferramentas: Vec<FerramentaViva>,
}

/// Falha da prévia. A mensagem é acionável, sem culpa, e NUNCA contém o segredo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErroMcp {
    /// Conector sem endpoint MCP (ou inválido): nada a conectar.
    SemEndpoint,
    /// O servidor pediu chave: defina a env var.
    ExigeSegredo {
        /// Nome da variável de ambiente esperada.
        env_var: String,
    },
    /// Falha de rede/transporte.
    Rede(String),
    /// Resposta fora do protocolo.
    Protocolo(String),
    /// Resposta maior que o teto.
    RespostaGrande,
    /// Tempo esgotado.
    Timeout,
    /// O servidor recusou (chave inválida, erro JSON-RPC).
    Recusado(String),
}

impl fmt::Display for ErroMcp {
    /// O quê: mensagem humana. Onde: CLI e logs (nunca imprime segredo).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SemEndpoint => f.write_str(
                "este conector ainda não tem servidor MCP para consultar; mostro a lista embutida",
            ),
            Self::ExigeSegredo { env_var } => write!(
                f,
                "o servidor pede uma chave; defina {env_var} com a sua chave e tente de novo"
            ),
            Self::Rede(m) => write!(
                f,
                "não consegui falar com o servidor ({m}); verifique a conexão e tente de novo"
            ),
            Self::Protocolo(m) => {
                write!(f, "o servidor respondeu algo inesperado ({m}); mostro a lista embutida")
            }
            Self::RespostaGrande => {
                f.write_str("a resposta do servidor passou do limite seguro e foi descartada")
            }
            Self::Timeout => {
                f.write_str("o servidor demorou demais para responder; tente de novo em instantes")
            }
            Self::Recusado(m) => {
                write!(f, "o servidor recusou o pedido ({m}); confira a chave em uso")
            }
        }
    }
}

impl std::error::Error for ErroMcp {}

/// Produz uma prévia de ferramentas de um conector. Somente leitura.
pub trait Previador {
    /// O quê: devolve a prévia do conector `c`, usando `s` se houver. Onde: CLI (U3).
    fn previa(&self, c: &Conector, s: Option<&Segredo>) -> Result<Previa, ErroMcp>;
}

/// Prévia sem rede: usa `Conector::ferramentas_estaticas`.
#[derive(Debug, Clone, Copy, Default)]
pub struct PreviaEstatica;

impl Previador for PreviaEstatica {
    /// O quê: converte a lista estática em [`Previa`] (origem `Estatica`). Onde: CLI, fallback.
    fn previa(&self, c: &Conector, _s: Option<&Segredo>) -> Result<Previa, ErroMcp> {
        let ferramentas = c
            .ferramentas_estaticas
            .iter()
            .map(|f| FerramentaViva {
                nome: f.nome.to_string(),
                descricao: f.descricao.to_string(),
                escrita: f.escrita,
            })
            .collect();
        Ok(Previa {
            conector_id: c.id.to_string(),
            origem: Origem::Estatica,
            protocolo: None,
            servidor: None,
            ferramentas,
        })
    }
}

/// Prévia ao vivo via `tools/list` (corpo em `viva.rs`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PreviaViva {
    /// Timeout duro total, em segundos.
    pub timeout_s: u64,
    /// Teto de bytes por resposta HTTP.
    pub max_bytes: usize,
    /// Teto de ferramentas devolvidas.
    pub max_ferramentas: usize,
}

impl Default for PreviaViva {
    /// O quê: 10 s, 1 MiB, 200 ferramentas. Onde: CLI.
    fn default() -> Self {
        Self { timeout_s: 10, max_bytes: 1024 * 1024, max_ferramentas: 200 }
    }
}
