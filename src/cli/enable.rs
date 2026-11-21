//! `enable` / `disable`: IMPRIME o comando/trecho exato; `--aplicar` só existe p/ claude-code.
//!
//! O quê: consentimento explícito — sem `--aplicar` nada é alterado. Com `--aplicar` roda
//! `claude mcp add|remove` por `argv` puro (sem shell), avisando que o cliente guarda a chave.
//! O valor do segredo nunca vai para stdout/stderr (a saída do `claude` é higienizada).
//! Onde: subcomandos `enable` e `disable`.

use crate::anuncio::Idioma;
use crate::cli::args::Cliente;
use crate::cli::falha::Falha;
use crate::cli::previa::exigir_endpoint;
use crate::cli::receita::{
    argv_add_real, argv_remove, arquivo_de, cliente_slug, linha_shell, receita,
};
use crate::cli::texto::{t, tf, K};
use crate::cli::{achar, emitir_json, env_var_de, link_docs, prefixo_de, Rt};
use crate::conector::{Auth, Conector};
use crate::segredo::resolver_com;
use serde_json::json;

/// O quê: executa `enable`. Onde: despacho.
pub fn enable(
    rt: &mut Rt<'_>,
    id: &str,
    cli: Cliente,
    aplicar: bool,
    json: bool,
) -> Result<(), Falha> {
    let c = achar(rt.l, id)?;
    exigir_endpoint(rt.l, c)?;
    if aplicar {
        return aplicar_add(rt, c, cli, json);
    }
    let r = receita(c, cli).ok_or_else(|| Falha::sem_endpoint(String::new()))?;
    if json {
        let doc = json!({
            "ok": true, "conector": c.id, "cliente": cliente_slug(cli), "aplicado": false,
            "comando": r.argv, "trecho": r.trecho, "arquivo": r.arquivo, "env_var": env_var_de(c),
        });
        return emitir_json(rt, &doc);
    }
    escrever_receita(rt, c, cli, &r)
}

/// O quê: texto humano da receita (comando ou trecho) + notas de env/OAuth + "nada alterado".
/// Onde: `enable` sem `--aplicar`.
fn escrever_receita(
    rt: &mut Rt<'_>,
    c: &Conector,
    cli: Cliente,
    r: &crate::cli::receita::Receita,
) -> Result<(), Falha> {
    let l = rt.l;
    let arq = r.arquivo.unwrap_or("");
    match cli {
        Cliente::ClaudeCode => writeln!(rt.out, "{}", tf(l, K::EnClaude, &[c.nome]))?,
        Cliente::Cursor => writeln!(rt.out, "{}", tf(l, K::EnCursor, &[c.nome, arq]))?,
        Cliente::Vscode => writeln!(rt.out, "{}", tf(l, K::EnVscode, &[c.nome, arq]))?,
    }
    if let Some(a) = &r.argv {
        writeln!(rt.out, "\n  {}", linha_shell(a))?;
    }
    if let Some(t) = &r.trecho {
        let bonito =
            serde_json::to_string_pretty(t).map_err(|e| Falha::uso("json", e.to_string()))?;
        writeln!(rt.out, "\n{bonito}")?;
    }
    match (c.auth, env_var_de(c)) {
        (_, Some(v)) => writeln!(rt.out, "\n{}", tf(l, K::EnEnv, &[v]))?,
        (Auth::OAuth, _) => writeln!(rt.out, "\n{}", t(l, K::EnOauth))?,
        _ => {}
    }
    writeln!(rt.out, "{}", t(l, K::EnNada))?;
    Ok(())
}

/// O quê: recusa `--aplicar` fora do claude-code (exit 1). Onde: `enable`/`disable`.
fn exigir_claude_code(l: Idioma, cli: Cliente) -> Result<(), Falha> {
    if cli == Cliente::ClaudeCode {
        return Ok(());
    }
    Err(Falha::uso("aplicar_so_claude_code", tf(l, K::EnAplicarSo, &[])))
}

/// O quê: recusa se o `claude` não está no PATH (exit 1, acionável). Onde: `--aplicar`.
fn exigir_claude(rt: &Rt<'_>, c: &Conector) -> Result<(), Falha> {
    if rt.amb.claude.disponivel() {
        return Ok(());
    }
    Err(Falha::uso("sem_claude", tf(rt.l, K::EnSemClaude, &[c.id])))
}

/// O quê: `enable --aplicar`: exige a env var, avisa, roda `claude mcp add` e higieniza a saída.
/// Onde: `enable`.
fn aplicar_add(rt: &mut Rt<'_>, c: &Conector, cli: Cliente, json: bool) -> Result<(), Falha> {
    exigir_claude_code(rt.l, cli)?;
    let segredo = match env_var_de(c) {
        None => None,
        Some(var) => Some(resolver_com(var, |k| (rt.amb.env)(k)).map_err(|_| {
            let m = tf(rt.l, K::ExigeChave, &[var, &link_docs(c), prefixo_de(c)]);
            Falha::exige_segredo(m, var)
        })?),
    };
    exigir_claude(rt, c)?;
    let valor = segredo.as_ref().map(|s| s.expor());
    let argv = argv_add_real(c, valor).ok_or_else(|| Falha::sem_endpoint(String::new()))?;
    writeln!(rt.err, "{}", t(rt.l, K::EnAviso))?;
    rodar_claude(rt, &argv, valor)?;
    finalizar(rt, c, cli, json, K::EnOk, Some(&argv_publico(c)))
}

/// O quê: o argv sem o valor da chave (para o JSON do resultado). Onde: `aplicar_add`.
fn argv_publico(c: &Conector) -> Vec<String> {
    receita(c, Cliente::ClaudeCode).and_then(|r| r.argv).unwrap_or_default()
}

/// O quê: roda `claude <argv[1..]>`, mostra a saída SEM o segredo e converte falha em exit 1.
/// Onde: `aplicar_add` e `aplicar_remove`.
fn rodar_claude(rt: &mut Rt<'_>, argv: &[String], segredo: Option<&str>) -> Result<(), Falha> {
    let ex = rt.amb.claude.rodar(&argv[1..]).map_err(|e| {
        Falha::uso("claude_nao_iniciou", e.replace(segredo.unwrap_or("\u{0}"), "***"))
    })?;
    let mut saida = ex.saida;
    if let Some(s) = segredo {
        saida = saida.replace(s, "***");
    }
    let limpa: String = saida.chars().filter(|ch| !ch.is_control() || *ch == '\n').collect();
    if !limpa.trim().is_empty() {
        writeln!(rt.err, "{}", limpa.trim_end())?;
    }
    if ex.codigo != 0 {
        return Err(Falha::uso("claude_falhou", tf(rt.l, K::EnFalhou, &[&ex.codigo.to_string()])));
    }
    Ok(())
}

/// O quê: mensagem/JSON de sucesso do `--aplicar`. Onde: fim de `aplicar_add`/`aplicar_remove`.
fn finalizar(
    rt: &mut Rt<'_>,
    c: &Conector,
    cli: Cliente,
    json: bool,
    ok: K,
    comando: Option<&Vec<String>>,
) -> Result<(), Falha> {
    if json {
        let doc = json!({"ok": true, "conector": c.id, "cliente": cliente_slug(cli), "aplicado": true, "comando": comando});
        return emitir_json(rt, &doc);
    }
    writeln!(rt.out, "{}", tf(rt.l, ok, &[c.nome]))?;
    Ok(())
}

/// O quê: executa `disable`: imprime `claude mcp remove` (ou a instrução manual) e, com
/// `--aplicar` (só claude-code), roda. Onde: despacho.
pub fn disable(
    rt: &mut Rt<'_>,
    id: &str,
    cli: Cliente,
    aplicar: bool,
    json: bool,
) -> Result<(), Falha> {
    let c = achar(rt.l, id)?;
    let argv = (cli == Cliente::ClaudeCode).then(|| argv_remove(c));
    if aplicar {
        exigir_claude_code(rt.l, cli)?;
        exigir_claude(rt, c)?;
        let a = argv.clone().unwrap_or_default();
        rodar_claude(rt, &a, None)?;
        return finalizar(rt, c, cli, json, K::DisOk, Some(&a));
    }
    if json {
        let doc = json!({
            "ok": true, "conector": c.id, "cliente": cliente_slug(cli), "aplicado": false,
            "comando": argv, "arquivo": arquivo_de(cli),
        });
        return emitir_json(rt, &doc);
    }
    escrever_remocao(rt, c, cli, argv)
}

/// O quê: texto humano do `disable` sem `--aplicar`. Onde: `disable`.
fn escrever_remocao(
    rt: &mut Rt<'_>,
    c: &Conector,
    cli: Cliente,
    argv: Option<Vec<String>>,
) -> Result<(), Falha> {
    let l = rt.l;
    match (argv, arquivo_de(cli)) {
        (Some(a), _) => {
            writeln!(rt.out, "{}\n\n  {}\n", tf(l, K::DisClaude, &[c.nome]), linha_shell(&a))?
        }
        (None, Some(arq)) => writeln!(rt.out, "{}", tf(l, K::DisManual, &[c.id, arq]))?,
        (None, None) => {}
    }
    writeln!(rt.out, "{}", t(l, K::EnNada))?;
    Ok(())
}
