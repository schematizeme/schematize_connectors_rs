//! `preview`: ferramentas de um conector, SOMENTE LEITURA (nunca `tools/call`).
//!
//! O quê: por padrão a lista EMBUTIDA (offline); `--ao-vivo` consulta o servidor via `tools/list`.
//! O segredo vem só da env var do conector; o valor nunca é impresso (só o nome).
//! Onde: subcomando `preview`; sempre via `&dyn Previador` (testável com fake).

use crate::anuncio::Idioma;
use crate::cli::falha::Falha;
use crate::cli::texto::{t, tf, K};
use crate::cli::{achar, emitir_json, env_var_de, link_docs, prefixo_de, Rt};
use crate::conector::Conector;
use crate::mcp::{Origem, Previa};
use crate::segredo::{resolver_com, Segredo};
use serde_json::{json, Value};

/// O quê: executa `preview`. Onde: despacho.
pub fn previa(rt: &mut Rt<'_>, id: &str, ao_vivo: bool, json: bool) -> Result<(), Falha> {
    let c = achar(rt.l, id)?;
    exigir_endpoint(rt.l, c)?;
    let segredo = if ao_vivo { segredo_do_conector(rt, c)? } else { None };
    let previador = if ao_vivo { rt.amb.vivo } else { rt.amb.estatico };
    let p = previador.previa(c, segredo.as_ref()).map_err(|e| {
        let var = env_var_de(c).unwrap_or("");
        Falha::de_mcp(&e, rt.l, var, &link_docs(c), prefixo_de(c))
    })?;
    if json {
        return emitir_json(rt, &doc_previa(&p));
    }
    let formato_ok = segredo.as_ref().is_none_or(|s| s.expor().starts_with(prefixo_de(c)));
    humano(rt, c, &p, segredo.is_some(), formato_ok)
}

/// O quê: conector sem endpoint (pendente/sem MCP) vira falha exit 3 acionável.
/// Onde: início de `previa`.
pub fn exigir_endpoint(l: Idioma, c: &Conector) -> Result<(), Falha> {
    if c.endpoint.is_some() {
        return Ok(());
    }
    let msg = match c.status {
        crate::conector::Status::SemMcp => tf(l, K::SemEndpointSemMcp, &[c.nome]),
        _ => tf(l, K::SemEndpointPendente, &[c.nome, c.dominio]),
    };
    Err(Falha::sem_endpoint(msg))
}

/// O quê: lê a chave SÓ da env var do conector; ausente/vazia vira exit 4 sem culpa. Conector
/// sem chave (OAuth/nenhuma) devolve `None`. Onde: `previa --ao-vivo`.
fn segredo_do_conector(rt: &Rt<'_>, c: &Conector) -> Result<Option<Segredo>, Falha> {
    let Some(var) = env_var_de(c) else { return Ok(None) };
    match resolver_com(var, |k| (rt.amb.env)(k)) {
        Ok(s) => Ok(Some(s)),
        Err(_) => {
            let msg = tf(rt.l, K::ExigeChave, &[var, &link_docs(c), prefixo_de(c)]);
            Err(Falha::exige_segredo(msg, var))
        }
    }
}

/// O quê: documento JSON da prévia (chaves estáveis, sem prosa). Onde: `--json` e testes.
pub fn doc_previa(p: &Previa) -> Value {
    let origem = match p.origem {
        Origem::Viva => "viva",
        Origem::Estatica => "estatica",
    };
    let escrita = p.ferramentas.iter().filter(|f| f.escrita).count();
    json!({
        "ok": true, "conector": p.conector_id, "origem": origem, "protocolo": p.protocolo,
        "servidor": p.servidor, "total": p.ferramentas.len(), "escrita": escrita,
        "ferramentas": p.ferramentas,
    })
}

/// O quê: tira caracteres de controle de texto vindo do servidor. Onde: saída humana.
fn sem_controle(s: &str) -> String {
    s.chars().filter(|c| !c.is_control()).collect()
}

/// O quê: saída humana: título, origem, tabela leitura/escrita e avisos. Onde: `previa`.
fn humano(
    rt: &mut Rt<'_>,
    c: &Conector,
    p: &Previa,
    chave: bool,
    fmt_ok: bool,
) -> Result<(), Falha> {
    let l = rt.l;
    let total = p.ferramentas.len().to_string();
    writeln!(rt.out, "{}", tf(l, K::PreviaTitulo, &[c.nome, &total]))?;
    match p.origem {
        Origem::Estatica => writeln!(rt.out, "{}", t(l, K::NotaEstatica))?,
        Origem::Viva => {
            let srv = sem_controle(p.servidor.as_deref().unwrap_or(c.dominio));
            writeln!(rt.out, "{}", tf(l, K::NotaViva, &[&srv]))?;
        }
    }
    if let (true, Some(var)) = (chave, env_var_de(c)) {
        writeln!(rt.out, "{}", tf(l, K::ChaveDefinida, &[var]))?;
        if !fmt_ok {
            writeln!(rt.out, "{}", tf(l, K::ChaveFormato, &[var, prefixo_de(c)]))?;
        }
    }
    let larg = p.ferramentas.iter().map(|f| f.nome.chars().count()).max().unwrap_or(0).min(48);
    for f in &p.ferramentas {
        let modo = t(l, if f.escrita { K::Escrita } else { K::Leitura });
        let nome = sem_controle(&f.nome);
        writeln!(rt.out, "  {nome:<larg$}  {modo:<8}  {}", sem_controle(&f.descricao))?;
    }
    if p.ferramentas.iter().any(|f| f.escrita) {
        writeln!(rt.out, "\n{}", t(l, K::AvisoEscrita))?;
    }
    Ok(())
}
