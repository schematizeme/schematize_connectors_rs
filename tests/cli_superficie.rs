//! A superfície da CLI é CONTRATO: o que a pessoa DIGITA (comandos, flags) não muda sem aviso.
//!
//! Compara a árvore do clap com `tests/superficie-cli.txt` (linhas `CMD`/`ARG`; as `SOBRE`
//! viajam no arquivo mas não entram na comparação). Mudança intencional: regrave com
//! `CONNECTORS_REGRAVA_SUPERFICIE=1 cargo test --test cli_superficie` e leia o diff.

use clap::CommandFactory;
use connectors::cli::args::Cli;

/// O quê: lista as linhas CMD/ARG/SOBRE de um comando e dos filhos. Onde: o teste abaixo.
fn superficie(c: &clap::Command, caminho: &str, out: &mut Vec<String>) {
    let nome = if caminho.is_empty() {
        c.get_name().to_string()
    } else {
        format!("{caminho} {}", c.get_name())
    };
    out.push(format!("CMD {nome}"));
    if let Some(s) = c.get_about() {
        out.push(format!("  SOBRE {}", s.to_string().lines().next().unwrap_or("").trim()));
    }
    let mut args: Vec<String> = c
        .get_arguments()
        .map(|a| {
            let longo = a.get_long().map(|l| format!("--{l}")).unwrap_or_default();
            let val =
                if a.get_num_args().is_some_and(|n| n.takes_values()) { " <valor>" } else { "" };
            let obrig = if a.is_required_set() { " OBRIGATORIO" } else { "" };
            format!("  ARG {:<20} {longo}{val}{obrig}", a.get_id())
        })
        .collect();
    args.sort();
    out.extend(args);
    let mut subs: Vec<&clap::Command> = c.get_subcommands().collect();
    subs.sort_by_key(|s| s.get_name());
    for s in subs {
        superficie(s, &nome, out);
    }
}

/// O quê: só a parte contratual (sem SOBRE). Onde: comparação.
fn contrato(t: &str) -> Vec<String> {
    t.lines().filter(|l| !l.trim_start().starts_with("SOBRE ")).map(String::from).collect()
}

#[test]
fn superficie_da_cli_nao_mudou() {
    let mut linhas = Vec::new();
    superficie(&Cli::command(), "", &mut linhas);
    let atual = linhas.join("\n") + "\n";
    let snap = std::path::Path::new("tests/superficie-cli.txt");
    if std::env::var_os("CONNECTORS_REGRAVA_SUPERFICIE").is_some() {
        std::fs::write(snap, &atual).expect("gravar o snapshot");
        return;
    }
    let esperado = std::fs::read_to_string(snap).expect("tests/superficie-cli.txt ausente");
    assert_eq!(contrato(&atual), contrato(&esperado), "a superfície da CLI mudou");
}

#[test]
fn self_check_a_comparacao_enxerga_uma_flag_a_menos() {
    let mut linhas = Vec::new();
    superficie(&Cli::command(), "", &mut linhas);
    let completo = linhas.join("\n");
    let sem_json: Vec<&str> = completo.lines().filter(|l| !l.contains("--json")).collect();
    assert_ne!(contrato(&completo), contrato(&sem_json.join("\n")), "comparação cega");
}
