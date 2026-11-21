//! Testes do fluxo MCP sobre um transporte Fake (sem rede).

use super::http::{ReqHttp, RespHttp, Transporte};
use super::tipos::*;
use crate::conector::{Auth, Conector, Status, Transporte as Tr};
use crate::segredo::resolver_com;
use serde_json::{json, Value};
use std::sync::Mutex;

type Resp = Result<RespHttp, ErroMcp>;
type Vista = (Value, Option<String>, Option<String>);
type Roteiro = Box<dyn Fn(&str, u64, usize) -> Resp + Send + Sync>;

/// Fake: grava o que recebeu e responde pelo roteiro `(método, id, nº da chamada)`.
struct Fake {
    roteiro: Roteiro,
    vistas: Mutex<Vec<Vista>>,
}

impl Fake {
    fn new(r: impl Fn(&str, u64, usize) -> Resp + Send + Sync + 'static) -> Self {
        Self { roteiro: Box::new(r), vistas: Mutex::new(Vec::new()) }
    }
    fn chamadas(&self) -> usize {
        self.vistas.lock().unwrap().len()
    }
}

impl Transporte for Fake {
    fn post(&self, req: &ReqHttp) -> Resp {
        let corpo: Value = serde_json::from_str(&req.corpo).unwrap();
        let metodo = corpo["method"].as_str().unwrap_or("").to_string();
        let id = corpo["id"].as_u64().unwrap_or(0);
        let n = {
            let mut v = self.vistas.lock().unwrap();
            v.push((corpo, req.sessao.clone(), req.protocolo.clone()));
            v.len()
        };
        (self.roteiro)(&metodo, id, n)
    }
}

fn ok_json(id: u64, result: Value) -> Resp {
    let corpo = json!({"jsonrpc":"2.0","id":id,"result":result}).to_string();
    Ok(RespHttp {
        status: 200,
        content_type: "application/json".into(),
        sessao: None,
        corpo: corpo.into_bytes(),
    })
}

fn vazio(status: u16) -> Resp {
    Ok(RespHttp { status, content_type: String::new(), sessao: None, corpo: vec![] })
}

fn init(id: u64) -> Resp {
    let mut r = ok_json(
        id,
        json!({"protocolVersion":"2025-06-18","serverInfo":{"name":"Asender MCP","version":"1.0"}}),
    );
    if let Ok(x) = r.as_mut() {
        x.sessao = Some("sess-42".into());
    }
    r
}

fn pagina(prefixo: &str, n: usize, cursor: Option<&str>) -> Value {
    let tools: Vec<Value> =
        (0..n).map(|i| json!({"name": format!("{prefixo}{i}"), "description":"d"})).collect();
    match cursor {
        Some(c) => json!({"tools": tools, "nextCursor": c}),
        None => json!({"tools": tools}),
    }
}

const ALVO: (&str, &str, Option<&str>) =
    ("asender", "https://api.asender.net/mcp", Some("ASENDER_API_KEY"));

#[test]
fn fluxo_completo_com_paginacao_e_sessao() {
    let t = Fake::new(|m, id, _| match m {
        "initialize" => init(id),
        "notifications/initialized" => vazio(202),
        _ if id == 2 => ok_json(id, pagina("a", 2, Some("p2"))),
        _ => ok_json(id, pagina("b", 1, None)),
    });
    let seg = resolver_com("X", |_| Some("sk_live_abc".into())).unwrap();
    let p = PreviaViva::default().fluxo(ALVO, Some(&seg), &t).unwrap();
    assert_eq!(
        (p.origem, p.protocolo.as_deref(), p.servidor.as_deref()),
        (Origem::Viva, Some("2025-06-18"), Some("Asender MCP 1.0"))
    );
    assert_eq!(
        p.ferramentas.iter().map(|f| f.nome.as_str()).collect::<Vec<_>>(),
        ["a0", "a1", "b0"]
    );
    assert!(p.ferramentas.iter().all(|f| f.escrita));
    let v = t.vistas.lock().unwrap();
    let metodos: Vec<_> = v.iter().map(|x| x.0["method"].as_str().unwrap()).collect();
    assert_eq!(metodos, ["initialize", "notifications/initialized", "tools/list", "tools/list"]);
    let i = &v[0].0["params"];
    assert_eq!(i["protocolVersion"], "2025-11-25");
    assert_eq!(i["clientInfo"]["name"], "schematize-connectors");
    assert_eq!(i["capabilities"], json!({}));
    assert!(v[0].1.is_none() && v[0].2.is_none());
    assert!(v[1..]
        .iter()
        .all(|x| x.1.as_deref() == Some("sess-42") && x.2.as_deref() == Some("2025-06-18")));
    assert_eq!(v[3].0["params"]["cursor"], "p2");
}

#[test]
fn notificacao_que_falha_nao_derruba() {
    let t = Fake::new(|m, id, _| match m {
        "initialize" => init(id),
        "notifications/initialized" => Err(ErroMcp::Rede("x".into())),
        _ => ok_json(id, pagina("a", 1, None)),
    });
    assert_eq!(PreviaViva::default().fluxo(ALVO, None, &t).unwrap().ferramentas.len(), 1);
}

#[test]
fn http_401_sem_chave_exige_segredo_e_com_chave_recusa() {
    let t = Fake::new(|_, _, _| vazio(401));
    let e = PreviaViva::default().fluxo(ALVO, None, &t).unwrap_err();
    assert_eq!(e, ErroMcp::ExigeSegredo { env_var: "ASENDER_API_KEY".into() });
    assert!(e.to_string().contains("ASENDER_API_KEY"));
    let seg = resolver_com("X", |_| Some("sk_live_TOPSECRET".into())).unwrap();
    let e = PreviaViva::default().fluxo(ALVO, Some(&seg), &t).unwrap_err();
    assert!(matches!(e, ErroMcp::Recusado(_)));
    assert!(!e.to_string().contains("TOPSECRET"));
}

#[test]
fn versao_nao_suportada_e_recusada() {
    let t = Fake::new(|_, id, _| ok_json(id, json!({"protocolVersion":"2024-01-01"})));
    assert!(matches!(PreviaViva::default().fluxo(ALVO, None, &t), Err(ErroMcp::Protocolo(_))));
}

#[test]
fn cursor_infinito_novo_a_cada_pagina_corta_em_10() {
    let t = Fake::new(|m, id, n| match m {
        "initialize" => init(id),
        "notifications/initialized" => vazio(202),
        _ => ok_json(id, pagina("t", 1, Some(&format!("c{n}")))),
    });
    let p = PreviaViva::default().fluxo(ALVO, None, &t).unwrap();
    assert_eq!(p.ferramentas.len(), 10);
    assert_eq!(t.chamadas(), 12);
}

#[test]
fn cursor_repetido_corta() {
    let t = Fake::new(|m, id, _| match m {
        "initialize" => init(id),
        "notifications/initialized" => vazio(202),
        _ => ok_json(id, pagina("t", 1, Some("mesmo"))),
    });
    let p = PreviaViva::default().fluxo(ALVO, None, &t).unwrap();
    assert_eq!(p.ferramentas.len(), 2);
}

#[test]
fn max_ferramentas_corta() {
    let t = Fake::new(|m, id, _| match m {
        "initialize" => init(id),
        "notifications/initialized" => vazio(202),
        _ => ok_json(id, pagina("t", 3, Some(&format!("c{id}")))),
    });
    let v = PreviaViva { max_ferramentas: 5, ..PreviaViva::default() };
    assert_eq!(v.fluxo(ALVO, None, &t).unwrap().ferramentas.len(), 5);
}

#[test]
fn segredo_hostil_nao_faz_nenhuma_chamada() {
    let t = Fake::new(|_, id, _| ok_json(id, json!({})));
    let seg = resolver_com("X", |_| Some("sk_a\"b\nX-Evil: 1".into())).unwrap();
    let e = PreviaViva::default().fluxo(ALVO, Some(&seg), &t).unwrap_err();
    assert!(matches!(e, ErroMcp::Recusado(_)));
    assert_eq!(t.chamadas(), 0);
}

#[test]
fn conector_invalido_ou_sem_endpoint_nao_conecta() {
    let base = Conector {
        id: "t",
        nome: "T",
        dominio: "asender.net",
        resumo_pt: "",
        resumo_en: "",
        principal: false,
        transporte: Tr::StreamableHttp,
        auth: Auth::Nenhuma,
        endpoint: None,
        ferramentas_estaticas: &[],
        status: Status::EndpointPendente,
    };
    let fora = Conector {
        endpoint: Some("https://evil.com/mcp"),
        status: Status::Ativo("2026-11-21"),
        ..base
    };
    let http = Conector {
        endpoint: Some("http://api.asender.net/mcp"),
        status: Status::Ativo("2026-11-21"),
        ..base
    };
    for c in [base, fora, http] {
        assert_eq!(PreviaViva::default().previa(&c, None), Err(ErroMcp::SemEndpoint));
    }
    let sse = Conector {
        transporte: Tr::Sse,
        endpoint: Some("https://api.asender.net/mcp"),
        status: Status::Ativo("2026-11-21"),
        ..base
    };
    assert!(matches!(PreviaViva::default().previa(&sse, None), Err(ErroMcp::Protocolo(_))));
}

#[test]
fn previa_estatica_usa_o_catalogo_sem_rede() {
    let c = crate::buscar("asender").unwrap();
    let p = PreviaEstatica.previa(c, None).unwrap();
    assert_eq!(p.origem, Origem::Estatica);
    assert!(p.protocolo.is_none() && p.servidor.is_none());
    assert_eq!(p.ferramentas.len(), c.ferramentas_estaticas.len());
    assert!(!p.ferramentas.is_empty());
}

#[test]
fn defaults_e_mensagens() {
    let v = PreviaViva::default();
    assert_eq!((v.timeout_s, v.max_bytes, v.max_ferramentas), (10, 1024 * 1024, 200));
    for e in [ErroMcp::SemEndpoint, ErroMcp::RespostaGrande, ErroMcp::Timeout] {
        assert!(!e.to_string().is_empty());
    }
}
