//! Garantias estáticas sobre o fonte de `src/mcp`: a prévia é somente leitura e enxuta.

use std::fs;

/// Fontes de produção de src/mcp (sem os arquivos `testes_*`).
fn fontes() -> Vec<(String, String)> {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/src/mcp");
    let mut v = Vec::new();
    for e in fs::read_dir(dir).expect("src/mcp existe") {
        let p = e.expect("entrada").path();
        let nome = p.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
        if nome.ends_with(".rs") && !nome.starts_with("testes_") {
            v.push((nome, fs::read_to_string(&p).expect("legível")));
        }
    }
    v
}

#[test]
fn nenhuma_funcao_de_invocacao_de_ferramenta() {
    let proibido = ["tools", "call"].join("/");
    for (nome, src) in fontes() {
        let codigo: Vec<&str> = src.lines().filter(|l| !l.trim_start().starts_with("//")).collect();
        assert!(
            !codigo.join("\n").contains(&proibido),
            "{nome} menciona a invocação fora de comentário"
        );
    }
}

#[test]
fn curl_nunca_desliga_verificacao_tls_nem_usa_shell() {
    for (nome, src) in fontes() {
        let codigo: Vec<&str> = src.lines().filter(|l| !l.trim_start().starts_with("//")).collect();
        let c = codigo.join("\n");
        for ruim in ["\"-k\"", "--insecure", "\"sh\"", "\"-c\"", "bash"] {
            assert!(!c.contains(ruim), "{nome} contém {ruim}");
        }
    }
}

#[test]
fn arquivos_dentro_do_teto_de_linhas() {
    for (nome, src) in fontes() {
        assert!(src.lines().count() <= 750, "{nome} passa de 750 linhas");
    }
}
