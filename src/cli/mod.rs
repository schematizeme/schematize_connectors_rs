//! CLI do `schematize-connectors`: despacho, ambiente injetável e utilitários comuns.
//!
//! O quê: `executar` parseia os argumentos, roda o subcomando e devolve o exit code; nada aqui
//! chama `process::exit` nem toca o mundo real direto (tudo vem em `Amb`), então é testável.
//! Onde: chamada por `src/main.rs`; os testes unitários usam fakes de `Previador`/`Claude`.
//!
//! Regras: o `--json` nunca é traduzido nem leva anúncio; segredo nunca é impresso (só o NOME
//! da env var); `--aplicar` é o único caminho que altera algo fora deste processo.

pub mod amb;
pub mod anuncios_cmd;
pub mod args;
pub mod claude;
pub mod diag;
pub mod enable;
pub mod falha;
pub mod lista;
pub mod previa;
pub mod receita;
pub mod texto;

#[cfg(test)]
mod testes;

use crate::anuncio::Idioma;
use crate::catalogo::{buscar, catalogo};
use crate::conector::{Auth, Conector, Status};
use amb::{idioma_de_env, Amb};
use args::{Cli, Cmd, LangArg};
use clap::Parser;
use falha::Falha;
use serde_json::{json, Value};
use std::io::Write;
use texto::{tf, K};

/// Contexto de execução de um comando: ambiente, idioma e as saídas.
pub struct Rt<'a> {
    /// Ambiente injetado.
    pub amb: &'a Amb<'a>,
    /// Idioma da saída humana.
    pub l: Idioma,
    /// stdout.
    pub out: &'a mut dyn Write,
    /// stderr.
    pub err: &'a mut dyn Write,
    /// `--sem-anuncios` foi passado.
    pub flag_sem_anuncios: bool,
}

/// O quê: executa a CLI com `args` (incluindo o argv[0]) e devolve o exit code.
/// Onde: `main` (real) e os testes (fakes). Erros de clap saem com 2; ajuda/versão com 0.
pub fn executar(args: &[String], amb: &Amb<'_>, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let cli = match Cli::try_parse_from(args) {
        Ok(c) => c,
        Err(e) => return mostrar_erro_clap(&e, out, err),
    };
    let l = match cli.lang {
        Some(LangArg::Pt) => Idioma::Pt,
        Some(LangArg::En) => Idioma::En,
        None => idioma_de_env(amb.env),
    };
    let json = cli.cmd.json();
    let mut rt = Rt { amb, l, out, err, flag_sem_anuncios: cli.sem_anuncios };
    match despachar(&mut rt, &cli.cmd) {
        Ok(()) => 0,
        Err(f) => reportar(&mut rt, &f, json),
    }
}

impl Cmd {
    /// O quê: o comando pediu `--json`? Onde: `executar`, para formatar falhas.
    pub fn json(&self) -> bool {
        match self {
            Cmd::List { json }
            | Cmd::Preview { json, .. }
            | Cmd::Enable { json, .. }
            | Cmd::Disable { json, .. }
            | Cmd::Anuncios { json, .. }
            | Cmd::Diag { json } => *json,
        }
    }
}

/// O quê: roda o subcomando. Onde: `executar`.
fn despachar(rt: &mut Rt<'_>, cmd: &Cmd) -> Result<(), Falha> {
    match cmd {
        Cmd::List { json } => lista::lista(rt, *json),
        Cmd::Preview { id, ao_vivo, json } => previa::previa(rt, id, *ao_vivo, *json),
        Cmd::Enable { id, cliente, aplicar, json } => {
            enable::enable(rt, id, *cliente, *aplicar, *json)
        }
        Cmd::Disable { id, cliente, aplicar, json } => {
            enable::disable(rt, id, *cliente, *aplicar, *json)
        }
        Cmd::Anuncios { slot, json } => anuncios_cmd::anuncios(rt, *slot, *json),
        Cmd::Diag { json } => diag::diag(rt, *json),
    }
}

/// O quê: imprime o erro do clap (ajuda/versão no stdout, o resto no stderr) e dá o exit.
/// Onde: `executar`. Falha ao escrever (pipe fechado) não muda o exit.
fn mostrar_erro_clap(e: &clap::Error, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    let txt = e.render().to_string();
    let r = if e.use_stderr() { write!(err, "{txt}") } else { write!(out, "{txt}") };
    match r {
        Ok(()) => e.exit_code(),
        Err(io) => Falha::from(io).exit.max(e.exit_code()),
    }
}

/// O quê: mostra a falha (JSON no stdout se `--json`, mensagem humana no stderr) e devolve o exit.
/// Onde: `executar`. Pipe fechado é saída normal e não imprime nada.
fn reportar(rt: &mut Rt<'_>, f: &Falha, json: bool) -> i32 {
    if f.slug == "saida_fechada" {
        return f.exit;
    }
    if json {
        let doc =
            json!({"ok": false, "erro": {"codigo": f.slug, "exit": f.exit, "env_var": f.env_var}});
        if let Err(e) = writeln!(rt.out, "{doc}") {
            return Falha::from(e).exit.max(f.exit);
        }
    }
    let rotulo = if rt.l == Idioma::Pt { "erro" } else { "error" };
    match writeln!(rt.err, "{rotulo}: {}", f.msg) {
        Ok(()) | Err(_) => f.exit, // stderr fechado: não há mais onde avisar; o exit já diz.
    }
}

/// O quê: troca controles (ESC, NUL, quebras) por `?` e limita a 64 chars, para ecoar entrada
/// hostil com segurança no terminal. Onde: mensagens que citam o id digitado.
pub fn limpar(s: &str) -> String {
    let mut o: String = s.chars().take(64).map(|c| if c.is_control() { '?' } else { c }).collect();
    if s.chars().count() > 64 {
        o.push('…');
    }
    o
}

/// O quê: resolve o id no catálogo; vazio/desconhecido/hostil viram falha exit 1 com a lista de
/// ids disponíveis. Onde: preview/enable/disable.
pub fn achar(l: Idioma, id: &str) -> Result<&'static Conector, Falha> {
    if id.trim().is_empty() {
        return Err(Falha::uso("id_vazio", tf(l, K::IdVazio, &[])));
    }
    buscar(id).ok_or_else(|| {
        let ids: Vec<&str> = catalogo().iter().map(|c| c.id).collect();
        Falha::uso("id_desconhecido", tf(l, K::IdDesconhecido, &[&limpar(id), &ids.join(", ")]))
    })
}

/// O quê: slug estável do status (`ativo`/`endpoint_pendente`/`sem_mcp`). Onde: JSON.
pub fn status_slug(c: &Conector) -> &'static str {
    match c.status {
        Status::Ativo(_) => "ativo",
        Status::EndpointPendente => "endpoint_pendente",
        Status::SemMcp => "sem_mcp",
    }
}

/// O quê: rótulo humano do status no idioma. Onde: `list`.
pub fn status_txt(l: Idioma, c: &Conector) -> &'static str {
    texto::t(
        l,
        match c.status {
            Status::Ativo(_) => K::StAtivo,
            Status::EndpointPendente => K::StPendente,
            Status::SemMcp => K::StSemMcp,
        },
    )
}

/// O quê: slug estável do tipo de auth. Onde: JSON.
pub fn auth_slug(c: &Conector) -> &'static str {
    match c.auth {
        Auth::Nenhuma => "nenhuma",
        Auth::BearerChave { .. } => "bearer_chave",
        Auth::OAuth => "oauth",
    }
}

/// O quê: nome da env var da chave do conector, se usa `BearerChave`. Onde: preview/enable/JSON.
pub fn env_var_de(c: &Conector) -> Option<&'static str> {
    match c.auth {
        Auth::BearerChave { env_var, .. } => Some(env_var),
        _ => None,
    }
}

/// O quê: grava um documento JSON numa linha. Onde: todo `--json`.
pub fn emitir_json(rt: &mut Rt<'_>, v: &Value) -> Result<(), Falha> {
    writeln!(rt.out, "{v}")?;
    Ok(())
}

/// O quê: link de documentação para criar a chave do conector. Onde: mensagem de chave ausente.
pub fn link_docs(c: &Conector) -> String {
    if c.id == "asender" {
        "https://developers.asender.net/guias/mcp.html".to_string()
    } else {
        format!("https://{}", c.dominio)
    }
}

/// O quê: prefixo esperado da chave (`sk_`), ou vazio. Onde: mensagens e validação de formato.
pub fn prefixo_de(c: &Conector) -> &'static str {
    match c.auth {
        Auth::BearerChave { prefixo, .. } => prefixo,
        _ => "",
    }
}
