//! Prévia ao vivo: `initialize`, `notifications/initialized` e `tools/list` paginado.
//! Somente leitura: este módulo NÃO tem como invocar ferramentas.

use super::curl::CurlTransporte;
use super::http::{mapear_status, validar_segredo, ReqHttp, RespHttp, Transporte};
use super::jsonrpc::{parse_ferramentas, parse_inicializacao, resposta_para_id};
use super::tipos::{ErroMcp, FerramentaViva, Origem, Previa, PreviaViva, Previador};
use crate::conector::{Auth, Conector, Transporte as TipoTransporte};
use crate::segredo::Segredo;
use serde_json::{json, Value};
use std::time::{Duration, Instant};

/// Versões de protocolo aceitas, da mais nova para a mais antiga.
pub(crate) const VERSOES: [&str; 3] = ["2025-11-25", "2025-06-18", "2025-03-26"];
/// Máximo de páginas de `tools/list`.
const MAX_PAGINAS: usize = 10;

/// Alvo da prévia já validado: `(id do conector, url https, env var da chave)`.
pub(crate) type Alvo<'a> = (&'a str, &'a str, Option<&'a str>);

/// Estado de uma sessão MCP efêmera (um `previa`).
struct Sessao<'a> {
    t: &'a dyn Transporte,
    url: &'a str,
    bearer: Option<String>,
    env_var: Option<&'a str>,
    sessao: Option<String>,
    protocolo: Option<String>,
    prox_id: u64,
    prazo: Instant,
    max_bytes: usize,
}

impl PreviaViva {
    /// O quê: teto de memória da prévia: `8 x max_bytes + 1 MiB` (buffer + texto + árvore JSON
    /// + sanitizados). Onde: BUDGET.md e teste `mcp_budget`.
    pub fn teto_memoria_bytes(&self) -> usize {
        self.max_bytes.saturating_mul(8).saturating_add(1024 * 1024)
    }

    /// O quê: executa o fluxo MCP sobre `t` num endpoint JÁ validado. Onde: `previa` (curl real)
    /// e testes (Fake/servidor local).
    pub(crate) fn fluxo(
        &self,
        alvo: Alvo<'_>,
        segredo: Option<&Segredo>,
        t: &dyn Transporte,
    ) -> Result<Previa, ErroMcp> {
        let (id_conector, url, env_var) = alvo;
        let bearer = segredo.map(|s| s.expor().to_string());
        if let Some(b) = &bearer {
            validar_segredo(b)?;
        }
        let mut s = Sessao {
            t,
            url,
            bearer,
            env_var,
            sessao: None,
            protocolo: None,
            prox_id: 1,
            prazo: Instant::now() + Duration::from_secs(self.timeout_s.max(1)),
            max_bytes: self.max_bytes,
        };
        s.executar(id_conector, self.max_ferramentas)
    }
}

impl Previador for PreviaViva {
    /// O quê: valida o conector (nada de rede se inválido) e faz a prévia viva via curl.
    /// Onde: CLI (U3).
    fn previa(&self, c: &Conector, s: Option<&Segredo>) -> Result<Previa, ErroMcp> {
        c.validar().map_err(|_| ErroMcp::SemEndpoint)?;
        let url = c.endpoint.ok_or(ErroMcp::SemEndpoint)?;
        if c.transporte != TipoTransporte::StreamableHttp {
            return Err(ErroMcp::Protocolo("só o transporte Streamable HTTP é suportado".into()));
        }
        let env = match c.auth {
            Auth::BearerChave { env_var, .. } => Some(env_var),
            _ => None,
        };
        self.fluxo((c.id, url, env), s, &CurlTransporte::new())
    }
}

impl Sessao<'_> {
    /// O quê: roda initialize, notificação e a paginação de tools/list. Onde: `fluxo`.
    fn executar(&mut self, id_conector: &str, max: usize) -> Result<Previa, ErroMcp> {
        let init = json!({"protocolVersion": VERSOES[0], "capabilities": {},
            "clientInfo": {"name": "schematize-connectors", "version": env!("CARGO_PKG_VERSION")}});
        let r = self.rpc("initialize", init)?;
        let (versao, servidor) = parse_inicializacao(&r)?;
        if !VERSOES.contains(&versao.as_str()) {
            return Err(ErroMcp::Protocolo(format!("versão de protocolo não suportada: {versao}")));
        }
        self.protocolo = Some(versao.clone());
        self.notificar("notifications/initialized")?;
        let ferramentas = self.listar(max)?;
        Ok(Previa {
            conector_id: id_conector.to_string(),
            origem: Origem::Viva,
            protocolo: Some(versao),
            servidor,
            ferramentas,
        })
    }

    /// O quê: pagina `tools/list` até `max` ferramentas, 10 páginas ou cursor repetido.
    /// Onde: `executar`.
    fn listar(&mut self, max: usize) -> Result<Vec<FerramentaViva>, ErroMcp> {
        let (mut todas, mut cursor, mut visto) = (Vec::new(), None::<String>, Vec::new());
        for _ in 0..MAX_PAGINAS {
            let params = cursor.as_ref().map_or(json!({}), |c| json!({ "cursor": c }));
            let r = self.rpc("tools/list", params)?;
            let (mut pagina, prox) = parse_ferramentas(&r)?;
            todas.append(&mut pagina);
            if todas.len() >= max {
                todas.truncate(max);
                break;
            }
            match prox {
                Some(p) if !visto.contains(&p) => {
                    visto.push(p.clone());
                    cursor = Some(p);
                }
                _ => break,
            }
        }
        Ok(todas)
    }

    /// O quê: envia uma requisição JSON-RPC e devolve o `result` do id correspondente.
    /// Onde: `executar` e `listar`.
    fn rpc(&mut self, metodo: &str, params: Value) -> Result<Value, ErroMcp> {
        let id = self.prox_id;
        self.prox_id += 1;
        let corpo = json!({"jsonrpc": "2.0", "id": id, "method": metodo, "params": params});
        let resp = self.enviar(&corpo)?;
        if let Some(s) = &resp.sessao {
            self.sessao = Some(s.clone());
        }
        resposta_para_id(&resp, id)
    }

    /// O quê: envia a notificação (sem id). Falha de rede/HTTP não crítica é ignorada de
    /// propósito (servidor stateless pode responder 202/204/404); prazo e teto propagam.
    /// Onde: `executar`.
    fn notificar(&mut self, metodo: &str) -> Result<(), ErroMcp> {
        let corpo = json!({"jsonrpc": "2.0", "method": metodo});
        match self.enviar(&corpo) {
            Err(e @ (ErroMcp::Timeout | ErroMcp::RespostaGrande)) => Err(e),
            Ok(_) | Err(_) => Ok(()),
        }
    }

    /// O quê: POST com o orçamento de tempo restante e mapeamento do status HTTP.
    /// Onde: `rpc` e `notificar`.
    fn enviar(&self, corpo: &Value) -> Result<RespHttp, ErroMcp> {
        let resto = self.prazo.checked_duration_since(Instant::now()).ok_or(ErroMcp::Timeout)?;
        let req = ReqHttp {
            url: self.url.to_string(),
            corpo: corpo.to_string(),
            bearer: self.bearer.clone(),
            sessao: self.sessao.clone(),
            protocolo: self.protocolo.clone(),
            timeout_s: resto.as_secs_f64().ceil().max(1.0) as u64,
            max_bytes: self.max_bytes,
        };
        let resp = self.t.post(&req)?;
        mapear_status(resp.status, self.bearer.is_some(), self.env_var)?;
        Ok(resp)
    }
}
