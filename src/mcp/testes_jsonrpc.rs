//! Testes do parse puro (JSON/SSE), sanitização e mapeamento de erros JSON-RPC.

use super::http::RespHttp;
use super::jsonrpc::*;
use super::tipos::ErroMcp;
use serde_json::json;

fn resp(ct: &str, corpo: &str) -> RespHttp {
    RespHttp {
        status: 200,
        content_type: ct.into(),
        sessao: None,
        corpo: corpo.as_bytes().to_vec(),
    }
}

#[test]
fn sanitiza_ansi_controle_e_bidi() {
    assert_eq!(sanitizar("\x1b[31mvermelho\x1b[0m", 50), "vermelho");
    assert_eq!(sanitizar("a\x1b]0;titulo\x07b", 50), "ab");
    assert_eq!(sanitizar("a\x1b]0;t\x1b\\b", 50), "ab");
    assert_eq!(sanitizar("lin1\nlin2\x00\x07\u{202E}x", 50), "lin1 lin2x");
    assert_eq!(sanitizar("\x1b", 50), "");
}

#[test]
fn trunca_descricao_enorme() {
    let s = sanitizar(&"é".repeat(10_000), MAX_DESCRICAO);
    assert_eq!(s.chars().count(), MAX_DESCRICAO);
    assert!(s.ends_with('…'));
}

#[test]
fn json_simples_e_resultado() {
    let r = resp("application/json", r#"{"jsonrpc":"2.0","id":3,"result":{"ok":1}}"#);
    assert_eq!(resposta_para_id(&r, 3).unwrap(), json!({"ok":1}));
}

#[test]
fn json_invalido_truncado_e_vazio() {
    for c in [r#"{"jsonrpc":"2.0","id":1,"res"#, "não é json", ""] {
        let e = resposta_para_id(&resp("application/json", c), 1).unwrap_err();
        assert!(matches!(e, ErroMcp::Protocolo(_)), "{c}");
    }
    let nao_utf8 = RespHttp { corpo: vec![0xff, 0xfe], ..resp("application/json", "") };
    assert!(matches!(resposta_para_id(&nao_utf8, 1), Err(ErroMcp::Protocolo(_))));
}

#[test]
fn id_errado_e_recusado() {
    let r = resp("application/json", r#"{"jsonrpc":"2.0","id":9,"result":{}}"#);
    let e = resposta_para_id(&r, 1).unwrap_err();
    assert_eq!(e, ErroMcp::Protocolo("o id da resposta não confere".into()));
    let sem = resp("application/json", r#"{"jsonrpc":"2.0","result":{}}"#);
    assert!(resposta_para_id(&sem, 1).is_err());
}

#[test]
fn erro_jsonrpc_vira_protocolo_ou_recusado() {
    let p = resp("application/json", r#"{"id":1,"error":{"code":-32601,"message":"nope"}}"#);
    assert!(matches!(resposta_para_id(&p, 1), Err(ErroMcp::Protocolo(m)) if m.contains("nope")));
    let r = resp(
        "application/json",
        r#"{"id":1,"error":{"code":-32001,"message":"\u001b[31msem permissão"}}"#,
    );
    assert_eq!(resposta_para_id(&r, 1), Err(ErroMcp::Recusado("sem permissão".into())));
}

#[test]
fn sse_com_varios_eventos_e_notificacao_antes() {
    let corpo = "event: message\ndata: {\"jsonrpc\":\"2.0\",\"method\":\"notifications/progress\"}\n\n\
                 event: message\r\ndata: {\"jsonrpc\":\"2.0\",\r\ndata: \"id\":2,\"result\":{\"v\":2}}\r\n\r\n: comentario\n";
    let r = resp("text/event-stream; charset=utf-8", corpo);
    assert_eq!(resposta_para_id(&r, 2).unwrap(), json!({"v":2}));
}

#[test]
fn sse_truncado_sem_linha_em_branco_e_invalido() {
    let r = resp("text/event-stream", "data: {\"id\":1,\"result\":{\"a\":");
    let e = resposta_para_id(&r, 1).unwrap_err();
    assert_eq!(e, ErroMcp::Protocolo("JSON inválido ou truncado".into()));
    let ok = resp("text/event-stream", "data: {\"id\":1,\"result\":{}}");
    assert!(resposta_para_id(&ok, 1).is_ok());
}

#[test]
fn lote_json_array() {
    let r = resp("application/json", r#"[{"id":5,"result":1},{"id":1,"result":{"x":true}}]"#);
    assert_eq!(resposta_para_id(&r, 1).unwrap(), json!({"x":true}));
}

#[test]
fn ferramentas_hostis_e_readonly_hint() {
    let grande = "d".repeat(10_240);
    let r = json!({"tools":[
        {"name":"\x1b[31mlista","description":grande,"annotations":{"readOnlyHint":true}},
        {"name":"envia","description":"x"},
        {"name":"talvez","annotations":{"readOnlyHint":false}},
        {"name":"texto","annotations":{"readOnlyHint":"true"}},
        {"name":"\x1b[0m"}, {"description":"sem nome"}, {"name":7}
    ],"nextCursor":"c2"});
    let (f, cur) = parse_ferramentas(&r).unwrap();
    assert_eq!(cur.as_deref(), Some("c2"));
    let nomes: Vec<_> = f.iter().map(|x| x.nome.as_str()).collect();
    assert_eq!(nomes, ["lista", "envia", "talvez", "texto"]);
    assert!(!f[0].escrita && f[1].escrita && f[2].escrita && f[3].escrita);
    assert_eq!(f[0].descricao.chars().count(), MAX_DESCRICAO);
    assert!(parse_ferramentas(&json!({"tools":"x"})).is_err());
}

#[test]
fn inicializacao_le_versao_e_servidor() {
    let r = json!({"protocolVersion":"2025-06-18","serverInfo":{"name":"\x1b[1mAsender","version":"1.2"}});
    let (v, s) = parse_inicializacao(&r).unwrap();
    assert_eq!((v.as_str(), s.as_deref()), ("2025-06-18", Some("Asender 1.2")));
    assert!(parse_inicializacao(&json!({})).is_err());
}
