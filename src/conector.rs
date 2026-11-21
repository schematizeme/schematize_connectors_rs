//! Tipos do conector MCP e a validação de segurança do endpoint (anti-SSRF/lookalike).

use crate::erro::ErroConector;
use serde::Serialize;

/// Transporte MCP do conector.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Transporte {
    /// Streamable HTTP (um POST por mensagem JSON-RPC).
    StreamableHttp,
    /// Server-Sent Events.
    Sse,
    /// Processo local por stdin/stdout.
    Stdio,
}

/// Como o conector autentica. O segredo é só REFERENCIADO por nome de env var.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Auth {
    /// Sem autenticação.
    Nenhuma,
    /// Header `Authorization: Bearer <chave>`; a chave vem da env var `env_var`.
    BearerChave {
        /// Nome da variável de ambiente que guarda a chave (nunca o valor).
        env_var: &'static str,
        /// Prefixo esperado da chave (ex.: `sk_`), só para validar o formato.
        prefixo: &'static str,
    },
    /// OAuth 2.1 (fluxo interativo, a cargo do cliente MCP).
    OAuth,
}

/// Situação do conector no catálogo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Status {
    /// Endpoint conhecido; carrega a data (`AAAA-MM-DD`) da última verificação humana.
    Ativo(&'static str),
    /// Projeto da casa cujo endpoint MCP o dono ainda não informou.
    EndpointPendente,
    /// Sem servidor MCP por enquanto.
    SemMcp,
}

/// Ferramenta conhecida de antemão, para a prévia sem rede.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct FerramentaPrevia {
    /// Nome da ferramenta MCP.
    pub nome: &'static str,
    /// O que ela faz, em uma linha.
    pub descricao: &'static str,
    /// `true` se escreve/envia algo (precisa de confirmação do usuário).
    pub escrita: bool,
}

/// Um conector MCP do catálogo (estático, sem alocação).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Conector {
    /// Identificador estável (minúsculo, ASCII).
    pub id: &'static str,
    /// Nome de exibição.
    pub nome: &'static str,
    /// Domínio próprio do conector; o endpoint tem que ser dele ou de subdomínio.
    pub dominio: &'static str,
    /// Resumo em português.
    pub resumo_pt: &'static str,
    /// Resumo em inglês.
    pub resumo_en: &'static str,
    /// `true` para o conector principal da casa (exatamente um no catálogo).
    pub principal: bool,
    /// Transporte MCP.
    pub transporte: Transporte,
    /// Autenticação.
    pub auth: Auth,
    /// URL do servidor MCP; `None` enquanto pendente.
    pub endpoint: Option<&'static str>,
    /// Prévia estática de ferramentas (vazia = a prévia vem do `tools/list` ao vivo).
    pub ferramentas_estaticas: &'static [FerramentaPrevia],
    /// Situação.
    pub status: Status,
}

/// Valida `url` como `https://` do domínio `dominio` (host igual ou subdomínio).
///
/// O quê: parser manual e estrito: ASCII visível, sem userinfo, porta só 443, sem
/// query/fragmento, host sem rótulo vazio, igual a `dominio` ou terminado em `.dominio`.
/// Onde: `Conector::validar` (endpoint) e `Anuncio::validar` (link).
pub fn validar_https_do_dominio(url: &str, dominio: &str) -> Result<(), ErroConector> {
    let resto = url.strip_prefix("https://").ok_or(ErroConector::NaoHttps)?;
    if !resto.bytes().all(|b| b.is_ascii_graphic()) || resto.contains('\\') {
        return Err(ErroConector::CaractereInvalido);
    }
    if resto.contains(['?', '#']) {
        return Err(ErroConector::QueryOuFragmento);
    }
    let autoridade = resto.split('/').next().unwrap_or("");
    if autoridade.contains('@') {
        return Err(ErroConector::Credencial);
    }
    let host = match autoridade.split_once(':') {
        Some((h, "443")) => h,
        Some(_) => return Err(ErroConector::Porta),
        None => autoridade,
    };
    validar_host(host, dominio)
}

/// O quê: confere forma do host e pertencimento ao domínio. Onde: só `validar_https_do_dominio`.
fn validar_host(host: &str, dominio: &str) -> Result<(), ErroConector> {
    let charset = host.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'.');
    if host.is_empty() || !charset || host.split('.').any(str::is_empty) {
        return Err(ErroConector::HostMalformado);
    }
    let h = host.to_ascii_lowercase();
    let d = dominio.to_ascii_lowercase();
    let sub = h.len() > d.len() && h.ends_with(&d) && h.as_bytes()[h.len() - d.len() - 1] == b'.';
    if h == d || sub {
        Ok(())
    } else {
        Err(ErroConector::HostForaDoDominio)
    }
}

impl Conector {
    /// Valida a coerência e a segurança da entrada.
    ///
    /// O quê: endpoint (se houver) é https do próprio domínio; `Ativo` exige endpoint;
    /// `EndpointPendente`/`SemMcp` exigem `None`; env var de auth é `[A-Z0-9_]+`.
    /// Onde: testes de invariante do catálogo e, em runtime, antes de qualquer conexão.
    pub fn validar(&self) -> Result<(), ErroConector> {
        if let Some(url) = self.endpoint {
            validar_https_do_dominio(url, self.dominio)?;
        }
        match (self.status, self.endpoint) {
            (Status::Ativo(_), None) => return Err(ErroConector::Incoerente("ativo sem endpoint")),
            (Status::EndpointPendente | Status::SemMcp, Some(_)) => {
                return Err(ErroConector::Incoerente("sem MCP ativo mas com endpoint"));
            }
            _ => {}
        }
        if let Auth::BearerChave { env_var, .. } = self.auth {
            let ok = !env_var.is_empty()
                && env_var
                    .bytes()
                    .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_');
            if !ok {
                return Err(ErroConector::Incoerente("nome de env var inválido"));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok(u: &str) -> Result<(), ErroConector> {
        validar_https_do_dominio(u, "asender.net")
    }

    #[test]
    fn aceita_dominio_e_subdominio() {
        assert!(ok("https://asender.net").is_ok());
        assert!(ok("https://api.asender.net/mcp").is_ok());
        assert!(ok("https://API.Asender.NET:443/mcp").is_ok());
    }

    #[test]
    fn recusa_lookalike_e_ssrf() {
        use ErroConector::*;
        assert_eq!(ok("https://api.asender.net.evil.com"), Err(HostForaDoDominio));
        assert_eq!(ok("https://evil.com/asender.net"), Err(HostForaDoDominio));
        assert_eq!(ok("https://notasender.net"), Err(HostForaDoDominio));
        assert_eq!(ok("https://evilasender.net/mcp"), Err(HostForaDoDominio));
        assert_eq!(ok("http://api.asender.net/mcp"), Err(NaoHttps));
        assert_eq!(ok("HTTPS://api.asender.net"), Err(NaoHttps));
        assert_eq!(ok("https://x@y"), Err(Credencial));
        assert_eq!(ok("https://api.asender.net@evil.com/"), Err(Credencial));
        assert_eq!(ok("https://u:p@api.asender.net"), Err(Credencial));
        assert_eq!(ok("https://api.asender.net:8443/mcp"), Err(Porta));
        assert_eq!(ok("https://api.asender.net:/mcp"), Err(Porta));
        assert_eq!(ok("https://api.asender.net/mcp?token=sk_x"), Err(QueryOuFragmento));
        assert_eq!(ok("https://api.asender.net/mcp#x"), Err(QueryOuFragmento));
    }

    #[test]
    fn recusa_unicode_nulo_e_controle() {
        use ErroConector::*;
        assert_eq!(ok("https://аsender.net"), Err(CaractereInvalido));
        assert_eq!(ok("https://api.asender.net\0.evil.com"), Err(CaractereInvalido));
        assert_eq!(ok("https://api.asender.net/m\ncp"), Err(CaractereInvalido));
        assert_eq!(ok("https://api.asender.net /mcp"), Err(CaractereInvalido));
        assert_eq!(ok("https://evil.com\\@api.asender.net"), Err(CaractereInvalido));
    }

    #[test]
    fn recusa_host_malformado() {
        use ErroConector::*;
        assert_eq!(ok("https://"), Err(HostMalformado));
        assert_eq!(ok("https:///mcp"), Err(HostMalformado));
        assert_eq!(ok("https://api..asender.net"), Err(HostMalformado));
        assert_eq!(ok("https://.asender.net"), Err(HostMalformado));
        assert_eq!(ok("https://asender.net."), Err(HostMalformado));
        assert!(ok("https://[::1]/").is_err());
    }

    fn base() -> Conector {
        Conector {
            id: "t",
            nome: "T",
            dominio: "asender.net",
            resumo_pt: "",
            resumo_en: "",
            principal: false,
            transporte: Transporte::StreamableHttp,
            auth: Auth::Nenhuma,
            endpoint: None,
            ferramentas_estaticas: &[],
            status: Status::EndpointPendente,
        }
    }

    #[test]
    fn validar_exige_coerencia() {
        assert!(base().validar().is_ok());
        let ativo_sem = Conector { status: Status::Ativo("2026-11-21"), ..base() };
        assert!(ativo_sem.validar().is_err());
        let pend_com = Conector { endpoint: Some("https://api.asender.net/mcp"), ..base() };
        assert!(pend_com.validar().is_err());
        let ruim = Conector {
            auth: Auth::BearerChave { env_var: "minha chave", prefixo: "sk_" },
            ..base()
        };
        assert!(ruim.validar().is_err());
        let fora = Conector {
            endpoint: Some("https://evil.com/mcp"),
            status: Status::Ativo("2026-11-21"),
            ..base()
        };
        assert_eq!(fora.validar(), Err(ErroConector::HostForaDoDominio));
    }
}
