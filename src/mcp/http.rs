//! Costura de transporte HTTP: tipos de requisição/resposta, parse do HTTP cru e status.

use super::tipos::ErroMcp;
use std::fmt;

/// Uma requisição POST da prévia. Não derive `Debug`: o `bearer` é segredo.
pub struct ReqHttp {
    /// URL https do servidor MCP.
    pub url: String,
    /// Corpo JSON-RPC.
    pub corpo: String,
    /// Chave para `Authorization: Bearer` (nunca vai em argv nem em log).
    pub bearer: Option<String>,
    /// `Mcp-Session-Id` recebido do servidor, se houver.
    pub sessao: Option<String>,
    /// `MCP-Protocol-Version` negociada, se já houver.
    pub protocolo: Option<String>,
    /// Orçamento de tempo desta requisição, em segundos (>= 1).
    pub timeout_s: u64,
    /// Teto de bytes lidos da resposta (cabeçalhos + corpo).
    pub max_bytes: usize,
}

impl fmt::Debug for ReqHttp {
    /// O quê: imprime a requisição ocultando a chave. Onde: asserts e logs de teste.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let bearer = if self.bearer.is_some() { "<oculto>" } else { "-" };
        f.debug_struct("ReqHttp").field("url", &self.url).field("bearer", &bearer).finish()
    }
}

/// Resposta HTTP já separada em partes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RespHttp {
    /// Código de status.
    pub status: u16,
    /// `Content-Type` em minúsculas (vazio se ausente).
    pub content_type: String,
    /// `Mcp-Session-Id` da resposta, validado.
    pub sessao: Option<String>,
    /// Corpo bruto.
    pub corpo: Vec<u8>,
}

/// Transporte plugável: o real é `CurlTransporte`; os testes usam um Fake.
pub trait Transporte {
    /// O quê: envia o POST e devolve a resposta. Onde: `viva::Sessao::enviar`.
    fn post(&self, req: &ReqHttp) -> Result<RespHttp, ErroMcp>;
}

/// O quê: aceita só texto visível simples (sem espaço, aspas, barra invertida ou controle).
/// Onde: antes de qualquer conexão e de novo ao montar a config do curl.
pub fn validar_segredo(v: &str) -> Result<(), ErroMcp> {
    let ok = !v.is_empty() && v.bytes().all(|b| b.is_ascii_graphic() && b != b'"' && b != b'\\');
    if ok {
        Ok(())
    } else {
        Err(ErroMcp::Recusado(
            "a chave tem caracteres inválidos; use só texto simples, sem aspas nem quebras".into(),
        ))
    }
}

/// O quê: aceita valor de header seguro (ASCII visível, sem aspas/barra, até 256).
/// Onde: `Mcp-Session-Id` recebido, antes de ser reenviado.
fn header_seguro(v: &str) -> bool {
    !v.is_empty() && v.len() <= 256 && validar_segredo(v).is_ok()
}

/// O quê: separa a saída `curl --include` em status, headers úteis e corpo (pula 1xx e o
/// bloco do CONNECT de proxy). Onde: `CurlTransporte` após ler o stdout limitado.
pub fn parse_http_cru(cru: &[u8]) -> Result<RespHttp, ErroMcp> {
    let mut resto = cru;
    loop {
        let fim = achar(resto, b"\r\n\r\n")
            .ok_or_else(|| ErroMcp::Protocolo("resposta HTTP truncada".into()))?;
        let cab = String::from_utf8_lossy(&resto[..fim]).into_owned();
        resto = &resto[fim + 4..];
        let status = status_da_linha(cab.lines().next().unwrap_or(""))?;
        if (100..200).contains(&status) || cab.contains("Connection established") {
            continue;
        }
        return montar(status, &cab, resto);
    }
}

/// O quê: posição da primeira ocorrência de `agulha`. Onde: `parse_http_cru`.
fn achar(h: &[u8], agulha: &[u8]) -> Option<usize> {
    h.windows(agulha.len()).position(|w| w == agulha)
}

/// O quê: extrai o código de `HTTP/x NNN ...`. Onde: `parse_http_cru`.
fn status_da_linha(linha: &str) -> Result<u16, ErroMcp> {
    let mut it = linha.split_whitespace();
    match (it.next(), it.next().and_then(|c| c.parse::<u16>().ok())) {
        (Some(v), Some(c)) if v.starts_with("HTTP/") && (100..600).contains(&c) => Ok(c),
        _ => Err(ErroMcp::Protocolo("linha de status HTTP inválida".into())),
    }
}

/// O quê: lê `Content-Type` e `Mcp-Session-Id` dos headers. Onde: `parse_http_cru`.
fn montar(status: u16, cab: &str, corpo: &[u8]) -> Result<RespHttp, ErroMcp> {
    let (mut content_type, mut sessao) = (String::new(), None);
    for linha in cab.lines().skip(1) {
        let Some((nome, valor)) = linha.split_once(':') else { continue };
        let valor = valor.trim();
        match nome.trim().to_ascii_lowercase().as_str() {
            "content-type" if content_type.is_empty() => content_type = valor.to_ascii_lowercase(),
            "mcp-session-id" if sessao.is_none() => {
                if !header_seguro(valor) {
                    return Err(ErroMcp::Protocolo("Mcp-Session-Id inválido".into()));
                }
                sessao = Some(valor.to_string());
            }
            _ => {}
        }
    }
    Ok(RespHttp { status, content_type, sessao, corpo: corpo.to_vec() })
}

/// O quê: traduz o status HTTP em erro acionável (401/403 sem chave pedem a env var).
/// Onde: `viva::Sessao::enviar`, depois de cada POST.
pub fn mapear_status(
    status: u16,
    enviou_segredo: bool,
    env_var: Option<&str>,
) -> Result<(), ErroMcp> {
    match status {
        200..=299 => Ok(()),
        401 | 403 => match (enviou_segredo, env_var) {
            (false, Some(v)) => Err(ErroMcp::ExigeSegredo { env_var: v.to_string() }),
            _ => Err(ErroMcp::Recusado(format!("HTTP {status}"))),
        },
        300..=399 => Err(ErroMcp::Protocolo("redirecionamento recusado".into())),
        408 | 429 => Err(ErroMcp::Rede(format!("HTTP {status}, tente mais tarde"))),
        500..=599 => Err(ErroMcp::Rede(format!("erro HTTP {status} do servidor"))),
        _ => Err(ErroMcp::Protocolo(format!("HTTP {status}"))),
    }
}
