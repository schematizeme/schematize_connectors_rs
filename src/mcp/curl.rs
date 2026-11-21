//! Transporte real: binário `curl` com argv puro; a config (inclusive a chave) vai por STDIN.

use super::http::{parse_http_cru, validar_segredo, ReqHttp, RespHttp, Transporte};
use super::tipos::ErroMcp;
use std::io::{ErrorKind, Read, Write};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::time::{Duration, Instant};

/// Transporte via `curl`. Só https; o modo `http_local` existe APENAS em `#[cfg(test)]`
/// (compilado fora do binário release) e só aceita `http://127.0.0.1:` para os testes locais.
#[derive(Debug, Default)]
pub struct CurlTransporte {
    /// Teste: permite `http://127.0.0.1:<porta>` (nunca existe no release).
    #[cfg(test)]
    pub(crate) http_local: bool,
}

impl CurlTransporte {
    /// O quê: transporte de produção (https estrito). Onde: `PreviaViva::previa`.
    pub fn new() -> Self {
        Self::default()
    }

    /// O quê: `true` só em teste com `http_local`. Onde: `post`.
    fn http_local(&self) -> bool {
        #[cfg(test)]
        {
            self.http_local
        }
        #[cfg(not(test))]
        {
            false
        }
    }
}

/// O quê: argv FIXO do curl (`-q` ignora curlrc; `--config -` lê a config do stdin).
/// Onde: `executar`. Nunca recebe URL, header nem segredo.
pub(crate) fn argv() -> [&'static str; 3] {
    ["-q", "--config", "-"]
}

/// O quê: escapa um valor para string entre aspas da config do curl; recusa controles.
/// Onde: `montar_config`, para todo valor interpolado.
fn esc(v: &str) -> Result<String, ErroMcp> {
    if v.chars().any(char::is_control) {
        return Err(ErroMcp::Protocolo("valor com caractere de controle".into()));
    }
    Ok(v.replace('\\', "\\\\").replace('"', "\\\""))
}

/// O quê: monta o texto da config do curl (https estrito, sem redirect, TLS >= 1.2, com
/// verificação de certificado). Onde: `post`; vai por stdin, jamais por argv.
pub(crate) fn montar_config(req: &ReqHttp, http_local: bool) -> Result<String, ErroMcp> {
    let local = http_local && req.url.starts_with("http://127.0.0.1:");
    if !local && !req.url.starts_with("https://") {
        return Err(ErroMcp::Rede("só https é permitido".into()));
    }
    let proto = if local { "=http" } else { "=https" };
    let mut c = format!(
        "silent\nshow-error\ninclude\nrequest = \"POST\"\nproto = \"{proto}\"\n\
         proto-redir = \"=https\"\nmax-redirs = 0\ntlsv1.2\nmax-time = {}\n",
        req.timeout_s
    );
    if local {
        c.push_str("noproxy = \"127.0.0.1\"\n");
    }
    let mut linha = |k: &str, v: &str| -> Result<(), ErroMcp> {
        c.push_str(&format!("{k} = \"{}\"\n", esc(v)?));
        Ok(())
    };
    linha("url", &req.url)?;
    linha("header", "Content-Type: application/json")?;
    linha("header", "Accept: application/json, text/event-stream")?;
    if let Some(b) = &req.bearer {
        validar_segredo(b)?;
        linha("header", &format!("Authorization: Bearer {b}"))?;
    }
    if let Some(s) = &req.sessao {
        linha("header", &format!("Mcp-Session-Id: {s}"))?;
    }
    if let Some(p) = &req.protocolo {
        linha("header", &format!("MCP-Protocol-Version: {p}"))?;
    }
    linha("data-raw", &req.corpo)?;
    Ok(c)
}

/// O quê: mata o filho e colhe o status. Onde: `executar` em teto/timeout/erro.
fn matar(filho: &mut Child) -> Result<(), ErroMcp> {
    match filho.kill() {
        Ok(()) => {}
        Err(e) if e.kind() == ErrorKind::InvalidInput => {}
        Err(_) => return Err(ErroMcp::Rede("não consegui encerrar o curl".into())),
    }
    filho.wait().map(|_| ()).map_err(|_| ErroMcp::Rede("não consegui encerrar o curl".into()))
}

/// O quê: traduz o código de saída do curl em erro sem culpa. Onde: `executar`.
fn erro_de_saida(codigo: Option<i32>) -> ErroMcp {
    match codigo {
        Some(28) => ErroMcp::Timeout,
        Some(6) => ErroMcp::Rede("não encontrei o endereço do servidor".into()),
        Some(7) => ErroMcp::Rede("conexão recusada".into()),
        Some(35 | 51 | 58 | 59 | 60 | 77) => ErroMcp::Rede("falha de TLS/certificado".into()),
        Some(1 | 3 | 47) => ErroMcp::Rede("o curl recusou o protocolo ou redirecionamento".into()),
        Some(c) => ErroMcp::Rede(format!("curl terminou com código {c}")),
        None => ErroMcp::Rede("o curl foi interrompido".into()),
    }
}

/// O quê: roda o curl com a config no stdin, lê o stdout com teto e prazo duros.
/// Onde: `Transporte::post`. Efeito: processo filho, sempre encerrado antes de retornar.
fn executar(config: &str, req: &ReqHttp) -> Result<RespHttp, ErroMcp> {
    let mut filho = Command::new("curl")
        .args(argv())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| match e.kind() {
            ErrorKind::NotFound => ErroMcp::Rede("o programa curl não está instalado".into()),
            _ => ErroMcp::Rede("não consegui iniciar o curl".into()),
        })?;
    let prazo = Instant::now() + Duration::from_secs(req.timeout_s);
    let escrita = filho.stdin.take().map(|mut i| i.write_all(config.as_bytes()));
    if !matches!(escrita, Some(Ok(()))) {
        return matar(&mut filho).and(Err(ErroMcp::Rede("falha ao configurar o curl".into())));
    }
    let rx = ler_stdout(&mut filho)?;
    let mut buf: Vec<u8> = Vec::new();
    loop {
        let resto = prazo.saturating_duration_since(Instant::now());
        match rx.recv_timeout(resto) {
            Ok(Ok(parte)) => buf.extend_from_slice(&parte),
            Ok(Err(_)) => {
                return matar(&mut filho).and(Err(ErroMcp::Rede("leitura falhou".into())))
            }
            Err(RecvTimeoutError::Timeout) => return matar(&mut filho).and(Err(ErroMcp::Timeout)),
            Err(RecvTimeoutError::Disconnected) => break,
        }
        if buf.len() > req.max_bytes {
            return matar(&mut filho).and(Err(ErroMcp::RespostaGrande));
        }
    }
    let st = filho.wait().map_err(|_| ErroMcp::Rede("não consegui colher o curl".into()))?;
    if !st.success() {
        return Err(erro_de_saida(st.code()));
    }
    parse_http_cru(&buf)
}

/// O quê: thread que lê o stdout do filho em blocos de 8 KiB e os envia por canal.
/// Onde: `executar`; o fim do canal (Disconnected) sinaliza EOF.
fn ler_stdout(filho: &mut Child) -> Result<mpsc::Receiver<std::io::Result<Vec<u8>>>, ErroMcp> {
    let mut out = filho.stdout.take().ok_or_else(|| ErroMcp::Rede("sem saída do curl".into()))?;
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let mut b = [0u8; 8192];
        loop {
            let r = out.read(&mut b).map(|n| b[..n].to_vec());
            let fim = !matches!(&r, Ok(v) if !v.is_empty());
            if fim {
                if let Err(e) = r {
                    // Erro de leitura é enviado; o receptor já pode ter saído (ok).
                    if tx.send(Err(e)).is_err() {
                        return;
                    }
                }
                return;
            }
            if tx.send(r).is_err() {
                return;
            }
        }
    });
    Ok(rx)
}

impl Transporte for CurlTransporte {
    /// O quê: POST via curl (config por stdin). Onde: `viva::Sessao::enviar`.
    fn post(&self, req: &ReqHttp) -> Result<RespHttp, ErroMcp> {
        let config = montar_config(req, self.http_local())?;
        executar(&config, req)
    }
}
