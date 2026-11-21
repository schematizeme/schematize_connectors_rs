//! Testes da config do curl, do parse HTTP cru e do mapeamento de status.

use super::curl::{argv, montar_config};
use super::http::*;
use super::tipos::ErroMcp;

const CHAVE: &str = "sk_live_SEGREDO123";

fn req(url: &str, bearer: Option<&str>) -> ReqHttp {
    ReqHttp {
        url: url.into(),
        corpo: r#"{"a":"b\"c"}"#.into(),
        bearer: bearer.map(str::to_string),
        sessao: Some("sess-1".into()),
        protocolo: Some("2025-11-25".into()),
        timeout_s: 7,
        max_bytes: 1000,
    }
}

#[test]
fn config_tem_flags_de_seguranca_e_o_segredo_so_nela() {
    let c = montar_config(&req("https://api.asender.net/mcp", Some(CHAVE)), false).unwrap();
    for f in [
        "proto = \"=https\"",
        "proto-redir = \"=https\"",
        "max-redirs = 0",
        "tlsv1.2",
        "max-time = 7",
    ] {
        assert!(c.contains(f), "falta {f}");
    }
    assert!(c.contains(&format!("header = \"Authorization: Bearer {CHAVE}\"")));
    assert!(c.contains("Mcp-Session-Id: sess-1") && c.contains("MCP-Protocol-Version: 2025-11-25"));
    assert!(c.contains(r#"data-raw = "{\"a\":\"b\\\"c\"}""#));
    assert!(!c.contains("insecure") && !c.contains("noproxy"));
}

#[test]
fn argv_e_fixo_e_nao_contem_segredo_nem_url() {
    let a = argv();
    assert_eq!(a, ["-q", "--config", "-"]);
    assert!(a.iter().all(|x| !x.contains("sk_") && !x.contains("http") && *x != "-k"));
}

#[test]
fn segredo_hostil_e_rejeitado() {
    for ruim in ["a\nb", "a\rb", "a\0b", "a\"b", "a\\b", "a b", "", "chavé"] {
        assert!(validar_segredo(ruim).is_err(), "{ruim:?}");
        let e = montar_config(&req("https://x.asender.net/", Some(ruim)), false);
        assert!(matches!(e, Err(ErroMcp::Recusado(_))), "{ruim:?}");
    }
    assert!(validar_segredo(CHAVE).is_ok());
}

#[test]
fn erro_recusa_nao_vaza_o_segredo() {
    let e = montar_config(&req("https://x.asender.net/", Some("sk_\nSEGREDO")), false).unwrap_err();
    assert!(!e.to_string().contains("SEGREDO"));
    assert!(!format!("{:?}", req("https://a.b/", Some(CHAVE))).contains("SEGREDO123"));
}

#[test]
fn so_https_e_http_local_so_no_loopback() {
    assert!(montar_config(&req("http://api.asender.net/mcp", None), false).is_err());
    assert!(montar_config(&req("http://127.0.0.1:80/", None), false).is_err());
    assert!(montar_config(&req("http://evil.com:80/", None), true).is_err());
    assert!(montar_config(&req("http://127.0.0.1:8080/", None), true).unwrap().contains("=http"));
}

#[test]
fn header_com_quebra_e_recusado() {
    let mut r = req("https://x.asender.net/", None);
    r.sessao = Some("a\r\nX-Evil: 1".into());
    assert!(montar_config(&r, false).is_err());
}

#[test]
fn parse_http_cru_pula_100_continue_e_le_headers() {
    let cru = b"HTTP/1.1 100 Continue\r\n\r\nHTTP/2 200 \r\nContent-Type: Text/Event-Stream\r\nMcp-Session-Id: abc123\r\n\r\ndata: {}\n\n";
    let r = parse_http_cru(cru).unwrap();
    assert_eq!((r.status, r.content_type.as_str()), (200, "text/event-stream"));
    assert_eq!(r.sessao.as_deref(), Some("abc123"));
    assert_eq!(r.corpo, b"data: {}\n\n");
}

#[test]
fn parse_http_cru_recusa_truncado_e_sessao_hostil() {
    assert!(parse_http_cru(b"HTTP/1.1 200 OK\r\nContent-Ty").is_err());
    assert!(parse_http_cru(b"lixo\r\n\r\n").is_err());
    assert!(parse_http_cru(b"").is_err());
    let r = parse_http_cru(b"HTTP/1.1 200 OK\r\nMcp-Session-Id: a\"b\r\n\r\n");
    assert!(r.is_err());
}

#[test]
fn status_http_mapeado() {
    let ex = |s, tem, env| mapear_status(s, tem, env);
    assert!(
        ex(200, false, None).is_ok() && ex(202, false, None).is_ok() && ex(204, true, None).is_ok()
    );
    assert_eq!(
        ex(401, false, Some("ASENDER_API_KEY")),
        Err(ErroMcp::ExigeSegredo { env_var: "ASENDER_API_KEY".into() })
    );
    assert_eq!(ex(403, false, Some("K")), Err(ErroMcp::ExigeSegredo { env_var: "K".into() }));
    assert!(matches!(ex(401, true, Some("K")), Err(ErroMcp::Recusado(_))));
    assert!(matches!(ex(401, false, None), Err(ErroMcp::Recusado(_))));
    assert!(matches!(ex(302, false, None), Err(ErroMcp::Protocolo(_))));
    assert!(matches!(ex(429, false, None), Err(ErroMcp::Rede(_))));
    assert!(matches!(ex(503, false, None), Err(ErroMcp::Rede(_))));
    assert!(matches!(ex(404, false, None), Err(ErroMcp::Protocolo(_))));
}
