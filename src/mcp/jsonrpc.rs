//! Parse PURO de respostas JSON-RPC (JSON ou SSE) e sanitização do que vem do servidor.

use super::http::RespHttp;
use super::tipos::{ErroMcp, FerramentaViva};
use serde_json::Value;

/// Limite de caracteres da descrição de ferramenta.
pub(crate) const MAX_DESCRICAO: usize = 300;
/// Limite de caracteres do nome de ferramenta.
pub(crate) const MAX_NOME: usize = 128;

/// O quê: texto do servidor seguro para terminal: remove sequências ANSI (CSI/OSC), controles,
/// bidi/zero-width, troca quebras por espaço e trunca em `max` chars. Onde: todo texto devolvido.
pub fn sanitizar(s: &str, max: usize) -> String {
    let mut out = String::new();
    let mut it = s.chars().peekable();
    while let Some(c) = it.next() {
        match c {
            '\x1b' => pular_escape(&mut it),
            '\n' | '\r' | '\t' => out.push(' '),
            c if c.is_control() || invisivel(c) => {}
            c => out.push(c),
        }
    }
    let limpo = out.trim();
    if limpo.chars().count() > max {
        let corte: String = limpo.chars().take(max.saturating_sub(1)).collect();
        return format!("{}…", corte.trim_end());
    }
    limpo.to_string()
}

/// O quê: consome o resto de uma sequência de escape já iniciada. Onde: `sanitizar`.
fn pular_escape(it: &mut std::iter::Peekable<std::str::Chars<'_>>) {
    match it.next() {
        Some('[') => {
            for c in it.by_ref() {
                if ('\u{40}'..='\u{7e}').contains(&c) {
                    break;
                }
            }
        }
        Some(']') => {
            while let Some(c) = it.next() {
                if c == '\x07' || (c == '\x1b' && it.next_if_eq(&'\\').is_some()) {
                    break;
                }
            }
        }
        _ => {}
    }
}

/// O quê: caracteres de formatação invisível (bidi, zero-width, BOM). Onde: `sanitizar`.
fn invisivel(c: char) -> bool {
    matches!(c, '\u{200B}'..='\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2060}'..='\u{2069}' | '\u{FEFF}')
}

/// O quê: extrai as mensagens JSON-RPC do corpo (JSON único/lote ou eventos SSE) e conta os
/// eventos inválidos. Onde: `resposta_para_id`.
pub fn extrair_mensagens(r: &RespHttp) -> Result<(Vec<Value>, usize), ErroMcp> {
    let texto = std::str::from_utf8(&r.corpo)
        .map_err(|_| ErroMcp::Protocolo("resposta não é texto UTF-8".into()))?;
    if r.content_type.contains("text/event-stream") {
        return Ok(mensagens_sse(texto));
    }
    match serde_json::from_str::<Value>(texto) {
        Ok(Value::Array(v)) => Ok((v, 0)),
        Ok(v) => Ok((vec![v], 0)),
        Err(_) => Err(ErroMcp::Protocolo("JSON inválido ou truncado".into())),
    }
}

/// O quê: quebra o SSE em eventos (linhas `data:` até linha em branco) e parseia cada um.
/// Onde: `extrair_mensagens`. Evento final sem linha em branco também é considerado.
fn mensagens_sse(texto: &str) -> (Vec<Value>, usize) {
    let (mut msgs, mut ruins, mut dados) = (Vec::new(), 0usize, Vec::<&str>::new());
    let mut fechar = |dados: &mut Vec<&str>| {
        if dados.is_empty() {
            return;
        }
        match serde_json::from_str::<Value>(&dados.join("\n")) {
            Ok(Value::Array(v)) => msgs.extend(v),
            Ok(v) => msgs.push(v),
            Err(_) => ruins += 1,
        }
        dados.clear();
    };
    for linha in texto.split('\n') {
        let linha = linha.strip_suffix('\r').unwrap_or(linha);
        if linha.is_empty() {
            fechar(&mut dados);
        } else if let Some(v) = linha.strip_prefix("data:") {
            dados.push(v.strip_prefix(' ').unwrap_or(v));
        }
    }
    fechar(&mut dados);
    (msgs, ruins)
}

/// O quê: acha a resposta do id `id` e devolve `result`; `error` vira `Protocolo`/`Recusado`.
/// Onde: `viva::Sessao::rpc`.
pub fn resposta_para_id(r: &RespHttp, id: u64) -> Result<Value, ErroMcp> {
    let (msgs, ruins) = extrair_mensagens(r)?;
    let mut viu_outro_id = false;
    for m in msgs {
        match m.get("id").and_then(Value::as_u64) {
            Some(i) if i == id => return resultado_ou_erro(&m),
            Some(_) => viu_outro_id = true,
            None => {}
        }
    }
    let motivo = if viu_outro_id {
        "o id da resposta não confere"
    } else if ruins > 0 {
        "JSON inválido ou truncado"
    } else {
        "resposta sem o id esperado"
    };
    Err(ErroMcp::Protocolo(motivo.into()))
}

/// O quê: separa `result` de `error` numa mensagem. Onde: `resposta_para_id`.
fn resultado_ou_erro(m: &Value) -> Result<Value, ErroMcp> {
    if let Some(e) = m.get("error") {
        let msg = sanitizar(e.get("message").and_then(Value::as_str).unwrap_or("sem detalhe"), 120);
        let cod = e.get("code").and_then(Value::as_i64).unwrap_or(0);
        return Err(if (-32700..=-32600).contains(&cod) {
            ErroMcp::Protocolo(format!("erro JSON-RPC {cod}: {msg}"))
        } else {
            ErroMcp::Recusado(msg)
        });
    }
    m.get("result")
        .cloned()
        .ok_or_else(|| ErroMcp::Protocolo("mensagem sem result nem error".into()))
}

/// O quê: lê `protocolVersion` e `serverInfo` do `initialize`. Onde: `viva::previa`.
pub fn parse_inicializacao(r: &Value) -> Result<(String, Option<String>), ErroMcp> {
    let versao = r
        .get("protocolVersion")
        .and_then(Value::as_str)
        .map(|v| sanitizar(v, 32))
        .ok_or_else(|| ErroMcp::Protocolo("initialize sem protocolVersion".into()))?;
    let info = r.get("serverInfo");
    let campo = |k: &str| info.and_then(|i| i.get(k)).and_then(Value::as_str);
    let servidor = campo("name").map(|n| match campo("version") {
        Some(v) => sanitizar(&format!("{n} {v}"), 80),
        None => sanitizar(n, 80),
    });
    Ok((versao, servidor))
}

/// O quê: lê `tools` e `nextCursor` de uma página de `tools/list`, sanitizando; só
/// `readOnlyHint == true` vira leitura. Onde: `viva::previa`.
pub fn parse_ferramentas(r: &Value) -> Result<(Vec<FerramentaViva>, Option<String>), ErroMcp> {
    let lista = r
        .get("tools")
        .and_then(Value::as_array)
        .ok_or_else(|| ErroMcp::Protocolo("tools/list sem a lista tools".into()))?;
    let mut out = Vec::with_capacity(lista.len().min(256));
    for t in lista {
        let nome = sanitizar(t.get("name").and_then(Value::as_str).unwrap_or(""), MAX_NOME);
        if nome.is_empty() {
            continue;
        }
        let desc = t.get("description").and_then(Value::as_str).unwrap_or("");
        let leitura = t.pointer("/annotations/readOnlyHint").and_then(Value::as_bool) == Some(true);
        out.push(FerramentaViva {
            nome,
            descricao: sanitizar(desc, MAX_DESCRICAO),
            escrita: !leitura,
        });
    }
    let cursor = r
        .get("nextCursor")
        .and_then(Value::as_str)
        .filter(|c| !c.is_empty() && c.len() <= 2048)
        .map(str::to_string);
    Ok((out, cursor))
}
