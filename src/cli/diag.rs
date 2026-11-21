//! `diag`: tamanhos/ordem do catálogo, validação de cada conector, opt-out e tty.
//!
//! O quê: relatório de saúde local, sem rede. Sai com exit 1 se algum conector não validar.
//! Onde: subcomando `diag`.

use crate::anuncio::anuncios;
use crate::catalogo::catalogo;
use crate::cli::anuncios_cmd::optout_origem;
use crate::cli::falha::Falha;
use crate::cli::texto::{t, tf, K};
use crate::cli::{emitir_json, Rt};
use serde_json::{json, Value};

/// O quê: resultado da validação de cada conector: `(id, None=ok | Some(código))`.
/// Onde: `diag` (JSON e humano).
fn validacoes() -> Vec<(&'static str, Option<String>)> {
    catalogo().iter().map(|c| (c.id, c.validar().err().map(|e| format!("{e:?}")))).collect()
}

/// O quê: a ordem do catálogo é válida (principal primeiro e único)? Onde: `diag`.
fn ordem_ok() -> bool {
    let c = catalogo();
    c.first().is_some_and(|p| p.principal) && c.iter().filter(|x| x.principal).count() == 1
}

/// O quê: quantos anúncios embutidos validam. Onde: `diag`.
fn anuncios_validos() -> usize {
    anuncios().iter().filter(|a| a.validar().is_ok()).count()
}

/// O quê: documento JSON do diag (chaves estáveis). Onde: `diag --json`.
fn doc(rt: &Rt<'_>, v: &[(&str, Option<String>)]) -> Value {
    let ordem: Vec<&str> = catalogo().iter().map(|c| c.id).collect();
    let origem = optout_origem(rt);
    let itens: Vec<Value> =
        v.iter().map(|(id, e)| json!({"id": id, "valido": e.is_none(), "erro": e})).collect();
    json!({
        "ok": v.iter().all(|(_, e)| e.is_none()) && ordem_ok(),
        "catalogo": {"total": ordem.len(), "ordem": ordem, "ordem_ok": ordem_ok(),
                     "principal": catalogo().first().map(|c| c.id)},
        "conectores": itens,
        "anuncios": {"total": anuncios().len(), "validos": anuncios_validos()},
        "optout": {"ativo": origem.is_some(), "origem": origem.map(|o| o.slug())},
        "tty": rt.amb.tty,
    })
}

/// O quê: executa `diag`. Onde: despacho. Falha de validação vira exit 1 após o relatório.
pub fn diag(rt: &mut Rt<'_>, json: bool) -> Result<(), Falha> {
    let v = validacoes();
    let d = doc(rt, &v);
    let saudavel = d["ok"] == Value::Bool(true);
    if json {
        emitir_json(rt, &d)?;
    } else {
        humano(rt, &v)?;
    }
    if saudavel {
        return Ok(());
    }
    Err(Falha::uso("catalogo_invalido", tf(rt.l, K::Falha, &["catalogo"])))
}

/// O quê: relatório humano do diag. Onde: `diag` sem `--json`.
fn humano(rt: &mut Rt<'_>, v: &[(&str, Option<String>)]) -> Result<(), Falha> {
    let l = rt.l;
    let sn = |b: bool| t(l, if b { K::Sim } else { K::Nao });
    let c = catalogo();
    let principal = c.first().map_or("-", |p| p.id);
    writeln!(rt.out, "{}", tf(l, K::DgCatalogo, &[&c.len().to_string(), principal]))?;
    let ordem: Vec<&str> = c.iter().map(|x| x.id).collect();
    let estado = t(l, if ordem_ok() { K::Ok } else { K::Nenhum });
    writeln!(rt.out, "{}", tf(l, K::DgOrdem, &[&ordem.join(" > "), estado]))?;
    for (id, e) in v {
        let r = e.as_deref().map_or_else(|| t(l, K::Ok).to_string(), |m| tf(l, K::Falha, &[m]));
        writeln!(rt.out, "{}", tf(l, K::DgConector, &[id, &r]))?;
    }
    let (tot, val) = (anuncios().len().to_string(), anuncios_validos().to_string());
    writeln!(rt.out, "{}", tf(l, K::DgAnuncios, &[&tot, &val]))?;
    let o = optout_origem(rt);
    let ot = o.map_or_else(
        || t(l, K::Nao).to_string(),
        |x| format!("{} ({})", t(l, K::Sim), x.texto(rt)),
    );
    writeln!(rt.out, "{}", tf(l, K::DgOptout, &[&ot]))?;
    writeln!(rt.out, "{}", tf(l, K::DgTty, &[sn(rt.amb.tty)]))?;
    Ok(())
}
