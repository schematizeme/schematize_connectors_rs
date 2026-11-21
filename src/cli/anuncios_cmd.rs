//! `anuncios`: audita o que `escolher` devolveria agora e POR QUÊ. Só informa.
//!
//! O quê: monta o `Contexto` real (tty, opt-out) e explica a decisão. Em `--json` ou sem tty o
//! resultado é só informativo, no próprio comando (id do candidato, nunca o texto do anúncio).
//! Onde: subcomando `anuncios`; `contexto_anuncio` também alimenta o banner do `list`.
//! Regra da casa: anúncio nunca vai para prompt/contexto de agente nem para `--json`.

use crate::anuncio::{escolher, optout_de_env, render_texto, Contexto, Slot};
use crate::cli::amb::optout_de_config;
use crate::cli::args::SlotArg;
use crate::cli::falha::Falha;
use crate::cli::texto::{t, tf, K};
use crate::cli::{emitir_json, Rt};
use serde_json::json;

/// De onde veio o opt-out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptOut {
    /// `SCHEMATIZE_SEM_ANUNCIOS`.
    Env,
    /// `--sem-anuncios`.
    Flag,
    /// Arquivo de config.
    Config,
}

impl OptOut {
    /// O quê: slug estável. Onde: JSON do `anuncios`/`diag`.
    pub fn slug(self) -> &'static str {
        match self {
            OptOut::Env => "env",
            OptOut::Flag => "flag",
            OptOut::Config => "config",
        }
    }

    /// O quê: texto humano da origem. Onde: `anuncios` e `diag`.
    pub fn texto(self, rt: &Rt<'_>) -> &'static str {
        t(
            rt.l,
            match self {
                OptOut::Env => K::OptEnv,
                OptOut::Flag => K::OptFlag,
                OptOut::Config => K::OptConfig,
            },
        )
    }
}

/// O quê: qual opt-out está ligado (env, flag ou config), se algum. Onde: contexto e `diag`.
pub fn optout_origem(rt: &Rt<'_>) -> Option<OptOut> {
    if optout_de_env(|k| (rt.amb.env)(k)) {
        Some(OptOut::Env)
    } else if rt.flag_sem_anuncios {
        Some(OptOut::Flag)
    } else if optout_de_config(rt.amb.env) {
        Some(OptOut::Config)
    } else {
        None
    }
}

/// O quê: o `Contexto` da invocação (sessao_n = 0, exibidos = 0; sem conectores ativos
/// conhecidos). Onde: `list` (banner) e `anuncios`.
pub fn contexto_anuncio(rt: &Rt<'_>, slot: Slot, modo_json: bool) -> Contexto<'static> {
    Contexto {
        slot,
        idioma: rt.l,
        tty: rt.amb.tty,
        modo_json,
        optout: optout_origem(rt).is_some(),
        conectores_ativos: &[],
        sessao_n: 0,
        exibidos_na_sessao: 0,
        agora: rt.amb.hoje,
    }
}

/// O quê: slot da lib e seu nome estável. Onde: `anuncios`.
fn slot_de(s: SlotArg) -> (Slot, &'static str) {
    match s {
        SlotArg::TelaInicial => (Slot::TelaInicial, "tela-inicial"),
        SlotArg::EstadoVazio => (Slot::EstadoVazio, "estado-vazio"),
        SlotArg::BannerCli => (Slot::BannerCli, "banner-cli"),
    }
}

/// O quê: executa `anuncios`. Onde: despacho.
pub fn anuncios(rt: &mut Rt<'_>, slot: SlotArg, json: bool) -> Result<(), Falha> {
    let (s, nome) = slot_de(slot);
    let ctx = contexto_anuncio(rt, s, json);
    let origem = optout_origem(rt);
    let bloqueio = if json {
        Some("modo_json")
    } else if !ctx.tty {
        Some("sem_tty")
    } else if origem.is_some() {
        Some("optout")
    } else {
        None
    };
    let livre = Contexto { modo_json: false, tty: true, optout: false, ..ctx };
    let candidato = escolher(&livre);
    let entregue = escolher(&ctx);
    if json {
        let c = candidato.map(|a| json!({"id": a.id, "conector_id": a.conector_id}));
        let ctxj = json!({"tty": ctx.tty, "json": json, "optout": origem.is_some(), "optout_origem": origem.map(OptOut::slug)});
        let doc = json!({"ok": true, "slot": nome, "contexto": ctxj, "bloqueio": bloqueio, "entregue": false, "candidato": c});
        return emitir_json(rt, &doc);
    }
    humano(
        rt,
        nome,
        &ctx,
        origem,
        bloqueio,
        candidato.map(|a| a.id),
        entregue.map(|a| render_texto(a, rt.l)),
    )
}

/// O quê: saída humana do `anuncios`. Onde: `anuncios` sem `--json`.
fn humano(
    rt: &mut Rt<'_>,
    nome: &str,
    ctx: &Contexto<'_>,
    origem: Option<OptOut>,
    bloqueio: Option<&str>,
    candidato: Option<&str>,
    texto: Option<String>,
) -> Result<(), Falha> {
    let l = rt.l;
    let sn = |b: bool| t(l, if b { K::Sim } else { K::Nao });
    let o = origem.map_or(t(l, K::Nao), |_| t(l, K::Sim));
    writeln!(rt.out, "{}", tf(l, K::AnContexto, &[nome, sn(ctx.tty), sn(false), o]))?;
    match (texto, bloqueio) {
        (Some(txt), _) => writeln!(rt.out, "{}\n{txt}", t(l, K::AnEntregue))?,
        (None, Some("sem_tty")) => writeln!(rt.out, "{}", t(l, K::AnBloqTty))?,
        (None, Some("optout")) => {
            let por = origem.map_or("", |x| x.texto(rt));
            writeln!(rt.out, "{}", tf(l, K::AnBloqOptout, &[por]))?
        }
        (None, _) => writeln!(rt.out, "{}", t(l, K::AnNenhum))?,
    }
    if let (Some(id), Some(_)) = (candidato, bloqueio) {
        writeln!(rt.out, "{}", tf(l, K::AnCandidato, &[id]))?;
    }
    Ok(())
}
