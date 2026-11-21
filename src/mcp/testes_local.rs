//! Integração REAL sem internet: servidor TCP mínimo em 127.0.0.1 + o binário `curl` de verdade.
//! `CurlTransporte { http_local: true }` só existe em `#[cfg(test)]` (nunca no release).

use super::curl::CurlTransporte;
use super::tipos::*;
use crate::segredo::resolver_com;
use serde_json::{json, Value};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Lê uma requisição HTTP/1.1 (headers + corpo por Content-Length).
fn ler_req(s: &mut std::net::TcpStream) -> Option<(String, String)> {
    let mut buf = Vec::new();
    let mut b = [0u8; 1024];
    loop {
        let n = s.read(&mut b).ok()?;
        if n == 0 {
            return None;
        }
        buf.extend_from_slice(&b[..n]);
        let txt = String::from_utf8_lossy(&buf).into_owned();
        if let Some(i) = txt.find("\r\n\r\n") {
            let cab = txt[..i].to_string();
            let cl = cab
                .to_lowercase()
                .lines()
                .find_map(|l| {
                    l.strip_prefix("content-length:")
                        .map(|v| v.trim().parse::<usize>().unwrap_or(0))
                })
                .unwrap_or(0);
            if buf.len() >= i + 4 + cl {
                return Some((cab, txt[i + 4..i + 4 + cl].to_string()));
            }
        }
    }
}

type Responde = Box<dyn Fn(&str, &Value) -> String + Send>;

/// Sobe o servidor; devolve a URL e o log das requisições (cabeçalhos).
fn servidor(n_conexoes: usize, f: Responde) -> (String, Arc<Mutex<Vec<String>>>) {
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://127.0.0.1:{}/mcp", l.local_addr().unwrap().port());
    let log = Arc::new(Mutex::new(Vec::new()));
    let log2 = log.clone();
    std::thread::spawn(move || {
        for _ in 0..n_conexoes {
            let Ok((mut s, _)) = l.accept() else { return };
            let Some((cab, corpo)) = ler_req(&mut s) else { continue };
            log2.lock().unwrap().push(cab);
            let v: Value = serde_json::from_str(&corpo).unwrap_or(Value::Null);
            let metodo = v["method"].as_str().unwrap_or("").to_string();
            let resp = f(&metodo, &v);
            if s.write_all(resp.as_bytes()).is_err() {
                return;
            }
        }
    });
    (url, log)
}

fn http(status: &str, ct: &str, extra: &str, corpo: &str) -> String {
    format!("HTTP/1.1 {status}\r\nContent-Type: {ct}\r\n{extra}Content-Length: {}\r\nConnection: close\r\n\r\n{corpo}", corpo.len())
}

fn local() -> CurlTransporte {
    CurlTransporte { http_local: true }
}

#[test]
fn ponta_a_ponta_json_e_sse_com_curl_real() {
    let (url, log) = servidor(
        3,
        Box::new(|m, v| {
            let id = v["id"].clone();
            match m {
            "initialize" => http("200 OK", "application/json", "Mcp-Session-Id: s-1\r\n", &json!({"jsonrpc":"2.0","id":id,"result":{"protocolVersion":"2025-11-25","serverInfo":{"name":"Fake","version":"9"}}}).to_string()),
            "notifications/initialized" => http("202 Accepted", "text/plain", "", ""),
            _ => {
                let msg = json!({"jsonrpc":"2.0","id":id,"result":{"tools":[
                    {"name":"\u{1b}[31mler","description":"lê","annotations":{"readOnlyHint":true}},
                    {"name":"enviar","description":"envia"}]}});
                http("200 OK", "text/event-stream", "", &format!("event: message\ndata: {msg}\n\n"))
            }
        }
        }),
    );
    let seg = resolver_com("X", |_| Some("sk_live_LOCAL".into())).unwrap();
    let p = PreviaViva::default().fluxo(("fake", &url, Some("K")), Some(&seg), &local()).unwrap();
    assert_eq!(p.protocolo.as_deref(), Some("2025-11-25"));
    assert_eq!(p.servidor.as_deref(), Some("Fake 9"));
    assert_eq!(p.ferramentas.len(), 2);
    assert_eq!((p.ferramentas[0].nome.as_str(), p.ferramentas[0].escrita), ("ler", false));
    assert!(p.ferramentas[1].escrita);
    let l = log.lock().unwrap();
    assert!(
        l[0].contains("Authorization: Bearer sk_live_LOCAL"),
        "o curl real recebeu a chave via config"
    );
    assert!(l[2].contains("Mcp-Session-Id: s-1"));
}

#[test]
fn resposta_maior_que_o_teto_vira_resposta_grande() {
    let (url, _) =
        servidor(1, Box::new(|_, _| http("200 OK", "application/json", "", &"x".repeat(60_000))));
    let v = PreviaViva { max_bytes: 10_000, ..PreviaViva::default() };
    assert_eq!(v.fluxo(("f", &url, None), None, &local()), Err(ErroMcp::RespostaGrande));
}

#[test]
fn servidor_mudo_vira_timeout_e_nao_pendura() {
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://127.0.0.1:{}/mcp", l.local_addr().unwrap().port());
    std::thread::spawn(move || {
        if let Ok((_s, _)) = l.accept() {
            std::thread::sleep(Duration::from_secs(5));
        }
    });
    let v = PreviaViva { timeout_s: 1, ..PreviaViva::default() };
    let t0 = std::time::Instant::now();
    assert_eq!(v.fluxo(("f", &url, None), None, &local()), Err(ErroMcp::Timeout));
    assert!(t0.elapsed() < Duration::from_secs(4));
}

#[test]
fn http_401_real_sem_chave_exige_segredo() {
    let (url, _) =
        servidor(1, Box::new(|_, _| http("401 Unauthorized", "application/json", "", "{}")));
    let e = PreviaViva::default()
        .fluxo(("f", &url, Some("ASENDER_API_KEY")), None, &local())
        .unwrap_err();
    assert_eq!(e, ErroMcp::ExigeSegredo { env_var: "ASENDER_API_KEY".into() });
}

#[test]
fn conexao_recusada_vira_rede() {
    let porta = TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
    let url = format!("http://127.0.0.1:{porta}/mcp");
    let e = PreviaViva::default().fluxo(("f", &url, None), None, &local()).unwrap_err();
    assert!(matches!(e, ErroMcp::Rede(_)), "{e:?}");
}

#[test]
fn transporte_de_producao_recusa_http_local() {
    let url = "http://127.0.0.1:9/mcp";
    let e =
        PreviaViva::default().fluxo(("f", url, None), None, &CurlTransporte::new()).unwrap_err();
    assert!(matches!(e, ErroMcp::Rede(_)));
}
