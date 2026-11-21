//! Testes do BINÁRIO real: contrato `--json`, exit codes, sem anúncio fora de tty e segredo
//! que nunca aparece. Sem rede (a prévia padrão é offline; `--ao-vivo` sem chave nem conecta).

use serde_json::Value;
use std::collections::BTreeSet;
use std::process::{Command, Output};

const CHAVE: &str = "sk_live_SEGREDO_DO_TESTE_BINARIO_987654";

/// O quê: roda o binário com ambiente limpo de idioma/config. Onde: todos os testes.
fn bin(args: &[&str], envs: &[(&str, &str)]) -> Output {
    let mut c = Command::new(env!("CARGO_BIN_EXE_schematize-connectors"));
    for v in [
        "SCHEMATIZE_LANG",
        "LC_ALL",
        "LC_MESSAGES",
        "LANG",
        "SCHEMATIZE_SEM_ANUNCIOS",
        "ASENDER_API_KEY",
    ] {
        c.env_remove(v);
    }
    c.env("XDG_CONFIG_HOME", "/nonexistent-cfg").env("HOME", "/nonexistent-home");
    c.args(args).envs(envs.iter().copied()).output().expect("rodar o binário")
}

fn texto(o: &Output) -> (String, String) {
    (
        String::from_utf8_lossy(&o.stdout).into_owned(),
        String::from_utf8_lossy(&o.stderr).into_owned(),
    )
}

/// O quê: caminhos de chaves de um JSON. Onde: contrato.
fn chaves(v: &Value, pref: &str, out: &mut BTreeSet<String>) {
    match v {
        Value::Object(m) => {
            for (k, x) in m {
                out.insert(format!("{pref}.{k}"));
                chaves(x, &format!("{pref}.{k}"), out);
            }
        }
        Value::Array(a) => a.iter().for_each(|x| chaves(x, &format!("{pref}[]"), out)),
        _ => {}
    }
}

/// O quê: JSON de stdout contém todas as chaves? Onde: contrato e self-check.
fn tem_chaves(stdout: &str, obrig: &[&str]) -> bool {
    let Ok(v) = serde_json::from_str::<Value>(stdout.trim()) else { return false };
    let mut s = BTreeSet::new();
    chaves(&v, "", &mut s);
    obrig.iter().all(|k| s.contains(*k))
}

#[test]
fn json_e_identico_nos_dois_idiomas_e_valido() {
    let casos: [(&[&str], &[&str]); 4] = [
        (&["list", "--json"], &[".conectores[].id", ".conectores[].status"]),
        (&["preview", "asender", "--json"], &[".total", ".escrita", ".ferramentas[].nome"]),
        (&["diag", "--json"], &[".catalogo.ordem_ok", ".optout.ativo"]),
        (&["anuncios", "--json"], &[".bloqueio", ".entregue"]),
    ];
    for (args, obrig) in casos {
        let mut pt = args.to_vec();
        pt.extend(["--lang", "pt"]);
        let mut en = args.to_vec();
        en.extend(["--lang", "en"]);
        let (a, b) = (texto(&bin(&pt, &[])).0, texto(&bin(&en, &[])).0);
        assert_eq!(a, b, "{args:?}: --json difere entre idiomas");
        assert!(tem_chaves(&a, obrig), "{args:?}: contrato quebrado: {a}");
    }
}

#[test]
fn self_check_do_contrato_do_binario_reprova_falha_conhecida() {
    assert!(!tem_chaves("{\"ok\":true}", &[".conectores[].id"]), "contrato cego");
    assert!(!tem_chaves("isto nao e json", &[".ok"]));
    assert!(!tem_chaves("", &[".ok"]));
}

#[test]
fn nunca_ha_anuncio_em_json_nem_sem_tty() {
    // stdout capturado = pipe = sem tty: nenhum anúncio, mesmo no modo humano.
    for args in [&["list"][..], &["list", "--json"], &["list", "--lang", "pt"]] {
        let (o, _) = texto(&bin(args, &[]));
        assert!(!o.contains("Anúncio") && !o.contains("Ad ·"), "{args:?}");
    }
    let (o, _) = texto(&bin(&["anuncios", "--lang", "pt"], &[]));
    assert!(o.contains("não é um terminal interativo") && !o.contains("Anúncio · da casa"));
}

#[test]
fn exit_codes_do_contrato() {
    let casos: [(&[&str], i32); 8] = [
        (&["list"], 0),
        (&["diag"], 0),
        (&["preview", "asender"], 0),
        (&["preview", "nao-existe"], 1),
        (&["preview", ""], 1),
        (&["preview", "nebularws"], 3),
        (&["preview", "asender", "--ao-vivo"], 4),
        (&["list", "--nada"], 2),
    ];
    for (args, exit) in casos {
        let o = bin(args, &[]);
        assert_eq!(o.status.code(), Some(exit), "{args:?}: {:?}", texto(&o));
        assert!(!texto(&o).1.contains("panicked"), "{args:?}");
    }
}

#[test]
fn ao_vivo_sem_chave_orienta_sem_culpa() {
    let o = bin(&["preview", "asender", "--ao-vivo", "--lang", "en"], &[]);
    let (so, se) = texto(&o);
    assert!(so.is_empty());
    assert!(
        se.contains("set ASENDER_API_KEY to a read-only sk_ key") && se.contains("run again"),
        "{se}"
    );
}

#[test]
fn enable_imprime_o_comando_e_nao_vaza_a_chave() {
    let o = bin(&["enable", "asender"], &[("ASENDER_API_KEY", CHAVE)]);
    let (so, se) = texto(&o);
    assert_eq!(o.status.code(), Some(0));
    assert!(so.contains("--header \"Authorization: Bearer ${ASENDER_API_KEY}\""));
    assert!(!so.contains(CHAVE) && !se.contains(CHAVE));
}

#[test]
fn aplicar_sem_claude_no_path_recusa_com_mensagem_acionavel() {
    let o = bin(
        &["enable", "asender", "--aplicar", "--lang", "en"],
        &[("ASENDER_API_KEY", CHAVE), ("PATH", "")],
    );
    let (so, se) = texto(&o);
    assert_eq!(o.status.code(), Some(1), "{se}");
    assert!(se.contains("`claude` command in PATH") && !so.contains(CHAVE) && !se.contains(CHAVE));
}

#[cfg(unix)]
#[test]
fn aplicar_roda_o_claude_real_por_argv_e_esconde_a_chave() {
    use std::os::unix::fs::PermissionsExt;
    let dir = std::env::temp_dir().join(format!("sc-fake-claude-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("criar dir");
    let log = dir.join("argv.log");
    let script =
        format!("#!/bin/sh\nprintf '%s\\n' \"$@\" > '{}'\necho \"ecoando $*\"\n", log.display());
    let alvo = dir.join("claude");
    std::fs::write(&alvo, script).expect("script");
    std::fs::set_permissions(&alvo, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    let path = format!("{}:/usr/bin:/bin", dir.display());
    let o = bin(
        &["enable", "asender", "--aplicar", "--lang", "pt"],
        &[("ASENDER_API_KEY", CHAVE), ("PATH", &path)],
    );
    let (so, se) = texto(&o);
    assert_eq!(o.status.code(), Some(0), "{se}");
    let argv = std::fs::read_to_string(&log).expect("log do fake");
    assert!(argv
        .contains("mcp\nadd\n--transport\nhttp\nasender\nhttps://api.asender.net/mcp\n--header\n"));
    assert!(
        argv.contains(&format!("Authorization: Bearer {CHAVE}")),
        "o claude precisa receber a chave real"
    );
    assert!(!so.contains(CHAVE) && !se.contains(CHAVE), "chave vazou: {so} {se}");
    assert!(se.contains("guardará a chave") && so.contains("Feito"));
    std::fs::remove_dir_all(&dir).expect("limpar");
}

#[test]
fn argumento_nao_utf8_nao_derruba_o_binario() {
    use std::os::unix::ffi::OsStrExt;
    let mut c = Command::new(env!("CARGO_BIN_EXE_schematize-connectors"));
    let o = c
        .arg("preview")
        .arg(std::ffi::OsStr::from_bytes(&[0xff, 0xfe, 0x41]))
        .output()
        .expect("rodar");
    assert_eq!(o.status.code(), Some(1));
    assert!(!String::from_utf8_lossy(&o.stderr).contains("panicked"));
}
