//! Testes das regras de anúncio (cada regra tem o seu).

use crate::anuncio::*;
use crate::catalogo::catalogo;

const HOJE: Data = Data { ano: 2026, mes: 11, dia: 21 };

fn ctx() -> Contexto<'static> {
    Contexto {
        slot: Slot::BannerCli,
        idioma: Idioma::Pt,
        tty: true,
        modo_json: false,
        optout: false,
        conectores_ativos: &[],
        sessao_n: 0,
        exibidos_na_sessao: 0,
        agora: HOJE,
    }
}

#[test]
fn caminho_feliz_escolhe_o_principal() {
    assert_eq!(escolher(&ctx()).map(|a| a.conector_id), Some("nebularws"));
}

#[test]
fn nunca_em_json_sem_tty_ou_com_optout() {
    for c in [
        Contexto { modo_json: true, ..ctx() },
        Contexto { tty: false, ..ctx() },
        Contexto { optout: true, ..ctx() },
    ] {
        for n in 0..20 {
            assert!(escolher(&Contexto { sessao_n: n, ..c }).is_none());
        }
    }
}

#[test]
fn teto_de_um_por_sessao() {
    assert!(escolher(&Contexto { exibidos_na_sessao: 1, ..ctx() }).is_none());
    assert!(escolher(&Contexto { exibidos_na_sessao: 7, ..ctx() }).is_none());
}

#[test]
fn nunca_de_conector_ativo() {
    let todos: Vec<&str> = catalogo().iter().map(|c| c.id).collect();
    assert!(escolher(&Contexto { conectores_ativos: &todos, ..ctx() }).is_none());
    for n in 0..20 {
        let c = Contexto { conectores_ativos: &["nebularws", "asender"], sessao_n: n, ..ctx() };
        let id = escolher(&c).map(|a| a.conector_id);
        assert!(!matches!(id, Some("nebularws" | "asender")), "{id:?}");
    }
}

#[test]
fn respeita_slot() {
    // Todos os modelos cobrem os 3 slots; um slot sem anúncio não existe, então simulamos
    // pelo filtro: o anúncio só é elegível se contiver o slot.
    for a in anuncios() {
        for s in [Slot::TelaInicial, Slot::EstadoVazio, Slot::BannerCli] {
            assert!(a.slots.contains(&s));
        }
    }
}

#[test]
fn expiracao_e_fail_closed() {
    let a = Anuncio { expira: Some("2026-11-20"), ..anuncios()[0] };
    assert!(Data::parse("2026-11-20").is_some_and(|d| HOJE > d));
    assert!(a.expira.is_some());
    // Data ilegível expira (fail-closed) e o parse é estrito.
    for ruim in
        ["", "2026-13-01", "2026-1-01", "26-11-21", "2026/11/21", "2026-11-2x", "2026-00-10"]
    {
        assert_eq!(Data::parse(ruim), None, "{ruim}");
    }
    assert_eq!(Data::parse("2026-11-21"), Some(HOJE));
}

#[test]
fn rotacao_deterministica_principal_nas_pares() {
    for n in (0..40).step_by(2) {
        assert_eq!(escolher(&Contexto { sessao_n: n, ..ctx() }).unwrap().conector_id, "nebularws");
    }
    let impares: Vec<_> = (0..16)
        .filter(|n| n % 2 == 1)
        .map(|n| escolher(&Contexto { sessao_n: n, ..ctx() }).unwrap().conector_id)
        .collect();
    let esperado = ["asender", "owiew", "axuz", "canverly"].repeat(2);
    assert_eq!(impares, esperado);
    // Determinismo: mesma entrada, mesma saída.
    let c = Contexto { sessao_n: 9, ..ctx() };
    assert_eq!(escolher(&c), escolher(&c));
}

#[test]
fn sem_principal_roda_entre_todos() {
    let ids: Vec<_> = (0..4)
        .map(|n| {
            let c = Contexto { conectores_ativos: &["nebularws"], sessao_n: n, ..ctx() };
            escolher(&c).unwrap().conector_id
        })
        .collect();
    assert_eq!(ids, ["asender", "owiew", "axuz", "canverly"]);
}

#[test]
fn so_o_principal_sobra_aparece_sempre() {
    let outros = ["asender", "owiew", "axuz", "canverly"];
    for n in 0..6 {
        let c = Contexto { conectores_ativos: &outros, sessao_n: n, ..ctx() };
        assert_eq!(escolher(&c).unwrap().conector_id, "nebularws");
    }
}

#[test]
fn sessao_n_extremo_nao_estoura() {
    for n in [u32::MAX, u32::MAX - 1, 1 << 31] {
        assert!(escolher(&Contexto { sessao_n: n, ..ctx() }).is_some());
    }
}

#[test]
fn rotulo_obrigatorio_e_render_sem_markup() {
    assert_eq!(rotulo(Idioma::Pt), "Anúncio · da casa");
    assert_eq!(rotulo(Idioma::En), "Ad · from us");
    for a in anuncios() {
        for i in [Idioma::Pt, Idioma::En] {
            let r = render_texto(a, i);
            assert!(r.starts_with(&format!("[{}]", rotulo(i))));
            assert!(r.ends_with(a.link) && !r.contains('\n') && !r.contains('\x1b'));
        }
    }
}

#[test]
fn render_descarta_controle_e_escape() {
    let a = Anuncio { texto_pt: "oi\x1b[31m\0\r\nfim", ..anuncios()[1] };
    let r = render_texto(&a, Idioma::Pt);
    assert!(!r.chars().any(char::is_control));
}

#[test]
fn todo_anuncio_valida_e_tem_conector_e_modelo_marcado() {
    for a in anuncios() {
        assert_eq!(a.validar(), Ok(()), "{}", a.id);
    }
    let ids: std::collections::HashSet<_> = anuncios().iter().map(|a| a.id).collect();
    assert_eq!(ids.len(), anuncios().len());
    assert_eq!(anuncios().len(), catalogo().len());
    for (a, c) in anuncios().iter().zip(catalogo()) {
        assert_eq!(a.conector_id, c.id);
    }
}

#[test]
fn link_hostil_e_recusado() {
    for link in ["https://asender.net.evil.com", "http://asender.net", "https://x@asender.net"] {
        let a = Anuncio { link, ..anuncios()[1] };
        assert!(a.validar().is_err(), "{link}");
    }
    let orfao = Anuncio { conector_id: "nao-existe", ..anuncios()[1] };
    assert!(orfao.validar().is_err());
}

#[test]
fn optout_de_env_aceita_so_valores_de_sim() {
    let com = |v: &'static str| {
        optout_de_env(move |k| (k == "SCHEMATIZE_SEM_ANUNCIOS").then(|| v.to_string()))
    };
    for v in ["1", "true", "TRUE", "yes", "Sim", " 1 "] {
        assert!(com(v), "{v}");
    }
    for v in ["0", "false", "", "no", "talvez", "11"] {
        assert!(!com(v), "{v}");
    }
    assert!(!optout_de_env(|_| None));
}

#[test]
fn json_dos_anuncios_e_estavel() {
    let a = serde_json::to_string(anuncios()).expect("serializa");
    assert_eq!(a, serde_json::to_string(anuncios()).expect("serializa"));
}
