//! Testes unitários da CLI com fakes de `Previador` e `Claude` (sem rede, sem processo real).

use crate::anuncio::Data;
use crate::cli::amb::Amb;
use crate::cli::claude::{Claude, Execucao};
use crate::cli::executar;
use crate::conector::Conector;
use crate::mcp::{ErroMcp, FerramentaViva, Origem, Previa, PreviaEstatica, Previador};
use crate::segredo::Segredo;
use serde_json::Value;
use std::cell::{Cell, RefCell};
use std::collections::BTreeSet;

const CHAVE: &str = "sk_live_SEGREDO_NUNCA_NA_SAIDA_123456";

/// Prévia fake: devolve o resultado programado e registra se recebeu segredo.
struct FakePrev {
    r: Result<Previa, ErroMcp>,
    recebeu_segredo: Cell<bool>,
}

impl Previador for FakePrev {
    /// O quê: devolve o resultado programado. Onde: testes de `--ao-vivo`.
    fn previa(&self, _c: &Conector, s: Option<&Segredo>) -> Result<Previa, ErroMcp> {
        self.recebeu_segredo.set(s.is_some());
        self.r.clone()
    }
}

/// Claude fake: registra o argv recebido e devolve a execução programada.
struct FakeClaude {
    presente: bool,
    ex: Execucao,
    visto: RefCell<Vec<Vec<String>>>,
}

impl Claude for FakeClaude {
    /// O quê: presença programada. Onde: testes de `--aplicar`.
    fn disponivel(&self) -> bool {
        self.presente
    }
    /// O quê: registra o argv e devolve a execução. Onde: testes de `--aplicar`.
    fn rodar(&self, args: &[String]) -> Result<Execucao, String> {
        self.visto.borrow_mut().push(args.to_vec());
        Ok(self.ex.clone())
    }
}

fn claude(presente: bool, codigo: i32, saida: &str) -> FakeClaude {
    FakeClaude { presente, ex: Execucao { codigo, saida: saida.into() }, visto: RefCell::default() }
}

fn previa_viva() -> Result<Previa, ErroMcp> {
    Ok(Previa {
        conector_id: "asender".into(),
        origem: Origem::Viva,
        protocolo: Some("2025-06-18".into()),
        servidor: Some("asender 1.0".into()),
        ferramentas: vec![
            FerramentaViva {
                nome: "a_ler".into(),
                descricao: "lê\u{1b}[31m".into(),
                escrita: false,
            },
            FerramentaViva { nome: "a_gravar".into(), descricao: "grava".into(), escrita: true },
        ],
    })
}

/// O quê: roda a CLI com ambiente fake; devolve (exit, stdout, stderr). Onde: todos os testes.
fn rodar(
    args: &[&str],
    env: &[(&str, &str)],
    tty: bool,
    vivo: &FakePrev,
    cl: &FakeClaude,
) -> (i32, String, String) {
    let mapa: Vec<(String, String)> =
        env.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
    let get = move |k: &str| mapa.iter().find(|(n, _)| n == k).map(|(_, v)| v.clone());
    let amb = Amb {
        env: &get,
        tty,
        hoje: Data { ano: 2026, mes: 11, dia: 21 },
        estatico: &PreviaEstatica,
        vivo,
        claude: cl,
    };
    let mut argv = vec!["schematize-connectors".to_string()];
    argv.extend(args.iter().map(|s| s.to_string()));
    let (mut o, mut e) = (Vec::new(), Vec::new());
    let code = executar(&argv, &amb, &mut o, &mut e);
    (code, String::from_utf8_lossy(&o).into_owned(), String::from_utf8_lossy(&e).into_owned())
}

fn simples(args: &[&str], env: &[(&str, &str)], tty: bool) -> (i32, String, String) {
    let v = FakePrev { r: previa_viva(), recebeu_segredo: Cell::new(false) };
    rodar(args, env, tty, &v, &claude(true, 0, ""))
}

/// O quê: conjunto de caminhos de chaves de um JSON (ignora valores). Onde: contrato `--json`.
fn chaves(v: &Value, pref: &str, out: &mut BTreeSet<String>) {
    match v {
        Value::Object(m) => {
            for (k, x) in m {
                let p = format!("{pref}.{k}");
                out.insert(p.clone());
                chaves(x, &p, out);
            }
        }
        Value::Array(a) => a.iter().for_each(|x| chaves(x, &format!("{pref}[]"), out)),
        _ => {}
    }
}

/// O quê: JSON válido com as chaves obrigatórias? Onde: contrato e self-check.
fn contrato_ok(v: &Value, obrigatorias: &[&str]) -> bool {
    let mut s = BTreeSet::new();
    chaves(v, "", &mut s);
    obrigatorias.iter().all(|k| s.contains(*k))
}

fn json(args: &[&str]) -> (i32, Value, String) {
    let (c, o, e) = simples(args, &[], false);
    (c, serde_json::from_str(o.trim()).unwrap_or(Value::Null), e)
}

#[test]
fn list_json_tem_o_mesmo_contrato_e_os_mesmos_bytes_nos_dois_idiomas() {
    let (_, pt, _) = simples(&["list", "--json", "--lang", "pt"], &[], false);
    let (_, en, _) = simples(&["list", "--json", "--lang", "en"], &[], false);
    assert_eq!(pt, en);
    let v: Value = serde_json::from_str(pt.trim()).unwrap();
    assert!(contrato_ok(
        &v,
        &[
            ".ok",
            ".conectores[].id",
            ".conectores[].status",
            ".conectores[].principal",
            ".conectores[].env_var"
        ]
    ));
    assert_eq!(v["conectores"][0]["id"], "nebularws");
    assert_eq!(v["conectores"][1]["status"], "ativo");
}

#[test]
fn self_check_do_contrato_reprova_json_sem_chave() {
    let ruim: Value = serde_json::from_str(r#"{"ok":true,"conectores":[{"id":"x"}]}"#).unwrap();
    assert!(!contrato_ok(&ruim, &[".conectores[].status"]), "contrato cego: aceitou chave ausente");
    assert!(!contrato_ok(&Value::Null, &[".ok"]));
}

#[test]
fn list_humano_marca_principal_e_traduz() {
    let (c, pt, _) = simples(&["list", "--lang", "pt"], &[], false);
    assert_eq!(c, 0);
    assert!(pt.lines().next().unwrap().contains("NOME"));
    let primeira = pt.lines().nth(1).unwrap();
    assert!(primeira.starts_with('★') && primeira.contains("nebularws"));
    assert!(pt.contains("endpoint pendente") && pt.contains("sem MCP") && pt.contains("ativo"));
    let (_, en, _) = simples(&["list", "--lang", "en"], &[], false);
    assert!(en.contains("endpoint pending") && en.contains("no MCP") && en.contains("NAME"));
}

#[test]
fn anuncio_so_com_tty_humano_e_sem_optout() {
    let com = simples(&["list", "--lang", "pt"], &[], true).1;
    assert_eq!(com.matches("Anúncio · da casa").count(), 1, "exatamente um anúncio");
    assert!(com.trim_end().ends_with("https://nebularws.com"));
    assert!(!simples(&["list"], &[], false).1.contains("Ad ·"), "sem tty não há anúncio");
    let j = simples(&["list", "--json"], &[], true).1;
    assert!(!j.contains("Anúncio") && !j.contains("Ad ·") && !j.contains("da casa"));
    for env in [&[("SCHEMATIZE_SEM_ANUNCIOS", "1")][..], &[("SCHEMATIZE_SEM_ANUNCIOS", "SIM")][..]]
    {
        assert!(!simples(&["list"], env, true).1.contains("Ad ·"));
    }
    assert!(!simples(&["list", "--sem-anuncios"], &[], true).1.contains("Ad ·"));
}

#[test]
fn optout_por_config_desliga_o_banner() {
    let dir = std::env::temp_dir().join(format!("sc-cfg-{}", std::process::id()));
    let alvo = dir.join("schematize").join("connectors");
    std::fs::create_dir_all(&alvo).unwrap();
    std::fs::write(alvo.join("sem-anuncios"), "").unwrap();
    let d = dir.to_string_lossy().into_owned();
    let (_, o, _) = simples(&["list"], &[("XDG_CONFIG_HOME", &d)], true);
    assert!(!o.contains("Ad ·") && !o.contains("Anúncio"));
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn preview_estatico_lista_32_e_deixa_claro_que_e_embutida() {
    let (c, o, _) = simples(&["preview", "asender", "--lang", "pt"], &[], false);
    assert_eq!(c, 0);
    assert!(o.contains("32 ferramentas") && o.contains("EMBUTIDA") && o.contains("--ao-vivo"));
    assert_eq!(o.matches("ESCRITA").count(), 3);
    assert!(o.contains("sk_*"));
    let (_, v, _) = json(&["preview", "asender", "--json"]);
    assert!(contrato_ok(
        &v,
        &[".origem", ".total", ".escrita", ".ferramentas[].nome", ".ferramentas[].escrita"]
    ));
    assert_eq!(
        (v["total"].as_u64(), v["escrita"].as_u64(), v["origem"].as_str()),
        (Some(32), Some(3), Some("estatica"))
    );
}

#[test]
fn preview_pendente_e_sem_mcp_dao_exit_3_acionavel() {
    for (id, trecho) in [
        ("nebularws", "ainda não foi cadastrado"),
        ("owiew", "src/catalogo.rs"),
        ("canverly", "não tem servidor MCP"),
    ] {
        let (c, o, e) = simples(&["preview", id, "--lang", "pt"], &[], false);
        assert_eq!((c, o.as_str()), (3, ""), "{id}");
        assert!(e.contains(trecho) && !e.contains("panicked"), "{id}: {e}");
    }
    let (c, v, _) = json(&["preview", "axuz", "--ao-vivo", "--json"]);
    assert_eq!((c, v["erro"]["codigo"].as_str()), (3, Some("sem_endpoint")));
}

#[test]
fn id_vazio_inexistente_unicode_e_controle_dao_exit_1_sem_eco_perigoso() {
    for id in ["", "   ", "nao-existe", "ação-ñ-日本", "x\u{1b}[2J\u{0}", &"a".repeat(500)] {
        let (c, _, e) = simples(&["preview", id], &[], false);
        assert_eq!(c, 1, "{id:?}");
        assert!(!e.contains('\u{1b}') && !e.contains('\u{0}') && e.len() < 600, "{id:?}");
    }
    let (_, _, e) = simples(&["enable", "nao-existe", "--lang", "en"], &[], false);
    assert!(e.contains("nebularws, asender"));
}

#[test]
fn ao_vivo_sem_chave_pede_a_variavel_sem_culpa_e_nao_chama_o_servidor() {
    let v = FakePrev { r: previa_viva(), recebeu_segredo: Cell::new(false) };
    let (c, o, e) = rodar(
        &["preview", "asender", "--ao-vivo", "--lang", "pt"],
        &[],
        false,
        &v,
        &claude(true, 0, ""),
    );
    assert_eq!((c, o.as_str()), (4, ""));
    assert!(
        e.contains("defina ASENDER_API_KEY com uma chave sk_ só de leitura")
            && e.contains("developers.asender.net")
    );
    assert!(e.contains("rode de novo") && !v.recebeu_segredo.get());
}

#[test]
fn ao_vivo_com_chave_passa_o_segredo_ao_previador_e_nunca_o_imprime() {
    let v = FakePrev { r: previa_viva(), recebeu_segredo: Cell::new(false) };
    let env = [("ASENDER_API_KEY", CHAVE)];
    for args in [
        &["preview", "asender", "--ao-vivo"][..],
        &["preview", "asender", "--ao-vivo", "--json"][..],
    ] {
        let (c, o, e) = rodar(args, &env, false, &v, &claude(true, 0, ""));
        assert_eq!(c, 0, "{e}");
        assert!(v.recebeu_segredo.get());
        assert!(!o.contains(CHAVE) && !e.contains(CHAVE) && !o.contains("SEGREDO"));
        assert!(!o.contains('\u{1b}'), "ANSI do servidor vazou");
    }
    let (_, o, _) = rodar(
        &["preview", "asender", "--ao-vivo", "--lang", "pt"],
        &env,
        false,
        &v,
        &claude(true, 0, ""),
    );
    assert!(o.contains("ASENDER_API_KEY") && o.contains("asender 1.0") && o.contains("ESCRITA"));
    let (_, o, _) = rodar(
        &["preview", "asender", "--ao-vivo"],
        &[("ASENDER_API_KEY", "abc")],
        false,
        &v,
        &claude(true, 0, ""),
    );
    assert!(o.contains("não começa com sk_") || o.contains("does not start with sk_"));
}

#[test]
fn erros_do_previador_viram_o_exit_certo() {
    let casos = [
        (ErroMcp::Timeout, 6),
        (ErroMcp::Rede("dns".into()), 5),
        (ErroMcp::Protocolo("x".into()), 5),
        (ErroMcp::RespostaGrande, 5),
        (ErroMcp::Recusado("401".into()), 5),
        (ErroMcp::ExigeSegredo { env_var: "ASENDER_API_KEY".into() }, 4),
        (ErroMcp::SemEndpoint, 3),
    ];
    for (e, exit) in casos {
        let v = FakePrev { r: Err(e.clone()), recebeu_segredo: Cell::new(false) };
        let env = [("ASENDER_API_KEY", CHAVE)];
        let (c, o, err) = rodar(
            &["preview", "asender", "--ao-vivo", "--json"],
            &env,
            false,
            &v,
            &claude(true, 0, ""),
        );
        assert_eq!(c, exit, "{e:?}");
        let j: Value = serde_json::from_str(o.trim()).unwrap();
        assert!(
            contrato_ok(&j, &[".ok", ".erro.codigo", ".erro.exit"]) && j["erro"]["exit"] == exit
        );
        assert!(!o.contains(CHAVE) && !err.contains(CHAVE));
    }
}

#[test]
fn enable_claude_code_imprime_o_comando_exato_sem_o_segredo() {
    let env = [("ASENDER_API_KEY", CHAVE)];
    let (c, o, e) = simples(&["enable", "asender", "--lang", "pt"], &env, false);
    assert_eq!(c, 0);
    assert!(o.contains(
        "claude mcp add --transport http asender https://api.asender.net/mcp --header \"Authorization: Bearer ${ASENDER_API_KEY}\""
    ));
    assert!(!o.contains(CHAVE) && !e.contains(CHAVE) && o.contains("Nada foi alterado"));
}

#[test]
fn enable_cursor_e_vscode_usam_referencias_e_nunca_o_valor() {
    let env = [("ASENDER_API_KEY", CHAVE)];
    let (_, o, _) = simples(&["enable", "asender", "--cliente", "cursor", "--json"], &env, false);
    let v: Value = serde_json::from_str(o.trim()).unwrap();
    assert_eq!(
        v["trecho"]["mcpServers"]["asender"]["headers"]["Authorization"],
        "Bearer ${env:ASENDER_API_KEY}"
    );
    assert_eq!(v["arquivo"], "~/.cursor/mcp.json");
    let (_, o2, _) = simples(&["enable", "asender", "--cliente", "vscode", "--json"], &env, false);
    let w: Value = serde_json::from_str(o2.trim()).unwrap();
    assert_eq!(
        w["trecho"]["servers"]["asender"]["headers"]["Authorization"],
        "Bearer ${input:asender-api-key}"
    );
    assert_eq!(w["trecho"]["inputs"][0]["password"], true);
    for s in [&o, &o2] {
        assert!(!s.contains(CHAVE));
    }
    for cli in ["cursor", "vscode"] {
        let (_, h, _) =
            simples(&["enable", "asender", "--cliente", cli, "--lang", "en"], &env, false);
        assert!(h.contains("$") && h.contains("{") && !h.contains(CHAVE), "{cli}");
    }
}

#[test]
fn enable_json_tem_o_mesmo_contrato_nos_dois_idiomas() {
    let (_, pt, _) = simples(&["enable", "asender", "--json", "--lang", "pt"], &[], false);
    let (_, en, _) = simples(&["enable", "asender", "--json", "--lang", "en"], &[], false);
    assert_eq!(pt, en);
    let v: Value = serde_json::from_str(pt.trim()).unwrap();
    assert!(contrato_ok(&v, &[".conector", ".cliente", ".aplicado", ".comando", ".env_var"]));
    assert_eq!(v["comando"][0], "claude");
}

#[test]
fn aplicar_recusa_fora_do_claude_code_sem_chave_e_sem_claude() {
    let env = [("ASENDER_API_KEY", CHAVE)];
    let (c, _, e) =
        simples(&["enable", "asender", "--cliente", "cursor", "--aplicar"], &env, false);
    assert_eq!(c, 1);
    assert!(e.contains("claude-code"));
    let (c, _, e) = simples(&["enable", "asender", "--aplicar", "--lang", "pt"], &[], false);
    assert_eq!(c, 4);
    assert!(e.contains("ASENDER_API_KEY"));
    let v = FakePrev { r: previa_viva(), recebeu_segredo: Cell::new(false) };
    let sem = claude(false, 0, "");
    let (c, _, e) =
        rodar(&["enable", "asender", "--aplicar", "--lang", "pt"], &env, false, &v, &sem);
    assert_eq!(c, 1);
    assert!(e.contains("`claude` no PATH") && !e.contains(CHAVE) && sem.visto.borrow().is_empty());
}

#[test]
fn aplicar_roda_claude_por_argv_puro_avisa_e_esconde_o_segredo() {
    let v = FakePrev { r: previa_viva(), recebeu_segredo: Cell::new(false) };
    let cl = claude(true, 0, &format!("Added server (header Authorization: Bearer {CHAVE})"));
    let env = [("ASENDER_API_KEY", CHAVE)];
    let (c, o, e) =
        rodar(&["enable", "asender", "--aplicar", "--lang", "pt"], &env, false, &v, &cl);
    assert_eq!(c, 0, "{e}");
    let visto = cl.visto.borrow();
    assert_eq!(visto.len(), 1);
    assert_eq!(&visto[0][..3], ["mcp", "add", "--transport"]);
    assert_eq!(
        visto[0].last().map(String::as_str),
        Some(format!("Authorization: Bearer {CHAVE}").as_str())
    );
    assert!(e.contains("guardará a chave") && o.contains("Feito"));
    assert!(!o.contains(CHAVE) && !e.contains(CHAVE), "segredo vazou: {o} {e}");
}

#[test]
fn aplicar_com_claude_falhando_da_exit_1_sem_vazar() {
    let v = FakePrev { r: previa_viva(), recebeu_segredo: Cell::new(false) };
    let cl = claude(true, 7, &format!("erro com {CHAVE}"));
    let env = [("ASENDER_API_KEY", CHAVE)];
    let (c, o, e) = rodar(&["enable", "asender", "--aplicar", "--json"], &env, false, &v, &cl);
    assert_eq!(c, 1);
    assert!(
        !o.contains(CHAVE) && !e.contains(CHAVE) && e.contains("código 7") || e.contains("code 7")
    );
}

#[test]
fn disable_imprime_remove_e_so_aplica_com_a_flag() {
    let cl = claude(true, 0, "");
    let v = FakePrev { r: previa_viva(), recebeu_segredo: Cell::new(false) };
    let (c, o, _) = rodar(&["disable", "asender"], &[], false, &v, &cl);
    assert!(c == 0 && o.contains("claude mcp remove asender") && cl.visto.borrow().is_empty());
    let (c, _, _) = rodar(&["disable", "asender", "--aplicar"], &[], false, &v, &cl);
    assert_eq!((c, cl.visto.borrow().len()), (0, 1));
    assert_eq!(cl.visto.borrow()[0], ["mcp", "remove", "asender"]);
    let (_, o, _) =
        simples(&["disable", "asender", "--cliente", "cursor", "--lang", "en"], &[], false);
    assert!(o.contains(".cursor/mcp.json"));
}

#[test]
fn anuncios_json_informa_o_porque_e_nunca_traz_o_texto() {
    let (c, v, _) = json(&["anuncios", "--json"]);
    assert_eq!(c, 0);
    assert!(contrato_ok(
        &v,
        &[".slot", ".bloqueio", ".entregue", ".candidato.id", ".contexto.tty"]
    ));
    assert_eq!((v["bloqueio"].as_str(), v["entregue"].as_bool()), (Some("modo_json"), Some(false)));
    assert_eq!(v["candidato"]["id"], "nebularws-modelo");
    assert!(v["candidato"].get("texto").is_none());
    let (_, o, _) = simples(&["anuncios", "--lang", "pt"], &[], false);
    assert!(o.contains("não é um terminal interativo") && !o.contains("Anúncio · da casa"));
    let (_, o, _) = simples(&["anuncios", "--lang", "pt", "--slot", "tela-inicial"], &[], true);
    assert!(o.contains("Exibiria agora:") && o.contains("Anúncio · da casa"));
    let (_, o, _) =
        simples(&["anuncios", "--lang", "en"], &[("SCHEMATIZE_SEM_ANUNCIOS", "1")], true);
    assert!(o.contains("opt-out is on") && !o.contains("Ad ·"));
}

#[test]
fn diag_valida_o_catalogo() {
    let (c, v, _) = json(&["diag", "--json"]);
    assert_eq!(c, 0);
    assert!(contrato_ok(
        &v,
        &[
            ".catalogo.total",
            ".catalogo.ordem_ok",
            ".conectores[].valido",
            ".optout.ativo",
            ".tty",
            ".anuncios.validos"
        ]
    ));
    assert_eq!(v["catalogo"]["total"], 5);
    let (c, o, _) = simples(&["diag", "--lang", "pt"], &[], false);
    assert!(
        c == 0 && o.contains("Catálogo: 5 conectores") && o.contains("Opt-out de anúncios: não")
    );
}

#[test]
fn argumentos_invalidos_do_clap_dao_exit_2() {
    for a in [
        &["nada"][..],
        &["list", "--xyz"],
        &["preview"],
        &["--lang", "xx", "list"],
        &["enable", "asender", "--cliente", "emacs"],
        &["anuncios", "--slot", "x"],
    ] {
        let (c, o, e) = simples(a, &[], false);
        assert_eq!(c, 2, "{a:?}");
        assert!(o.is_empty() && !e.is_empty());
    }
    assert_eq!(simples(&["--help"], &[], false).0, 0);
    assert_eq!(simples(&["--version"], &[], false).0, 0);
}
