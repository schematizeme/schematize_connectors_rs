//! `list`: tabela dos conectores, JSON estável e (só em terminal humano) um anúncio.
//!
//! O quê: id, nome, domínio, status e ★ no principal; o catálogo já vem com nebularws primeiro.
//! Onde: subcomando `list`.

use crate::anuncio::{escolher, render_texto, Slot};
use crate::catalogo::catalogo;
use crate::cli::anuncios_cmd::contexto_anuncio;
use crate::cli::falha::Falha;
use crate::cli::texto::{t, K};
use crate::cli::{auth_slug, emitir_json, env_var_de, status_slug, status_txt, Rt};
use serde_json::{json, Value};

/// O quê: documento JSON da lista (chaves estáveis, sem prosa). Onde: `list --json` e testes.
pub fn doc_lista() -> Value {
    let itens: Vec<Value> = catalogo()
        .iter()
        .map(|c| {
            json!({
                "id": c.id, "nome": c.nome, "dominio": c.dominio, "principal": c.principal,
                "status": status_slug(c), "endpoint": c.endpoint, "auth": auth_slug(c),
                "env_var": env_var_de(c),
            })
        })
        .collect();
    json!({"ok": true, "conectores": itens})
}

/// O quê: executa `list`. Onde: despacho.
pub fn lista(rt: &mut Rt<'_>, json: bool) -> Result<(), Falha> {
    if json {
        return emitir_json(rt, &doc_lista());
    }
    let l = rt.l;
    writeln!(
        rt.out,
        "  {:<10} {:<10} {:<14} {}",
        "ID",
        t(l, K::ColNome),
        t(l, K::ColDominio),
        t(l, K::ColStatus)
    )?;
    for c in catalogo() {
        let estrela = if c.principal { "★" } else { " " };
        writeln!(
            rt.out,
            "{estrela} {:<10} {:<10} {:<14} {}",
            c.id,
            c.nome,
            c.dominio,
            status_txt(l, c)
        )?;
    }
    writeln!(rt.out, "\n{}", t(l, K::ListaRodape))?;
    banner(rt)
}

/// O quê: UM anúncio do slot BannerCli ao fim, se tty, humano e sem opt-out (`escolher` decide;
/// sessao_n = 0 e exibidos = 0 por invocação). Onde: fim de `list`. Nunca no `--json`.
fn banner(rt: &mut Rt<'_>) -> Result<(), Falha> {
    let ctx = contexto_anuncio(rt, Slot::BannerCli, false);
    if let Some(a) = escolher(&ctx) {
        writeln!(rt.out, "\n{}", render_texto(a, rt.l))?;
    }
    Ok(())
}
