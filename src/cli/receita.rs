//! Receitas de `enable`/`disable`: o comando/trecho EXATO por cliente MCP. Funções puras.
//!
//! O quê: monta o `argv` do `claude mcp add` e os trechos JSON do Cursor e do VS Code. O segredo
//! NUNCA entra aqui: só a REFERÊNCIA (`${ENV}` / `${env:ENV}` / `${input:...}`); a expansão é do
//! shell ou do cliente. A única função que recebe o valor é `argv_add_real` (usada no `--aplicar`).
//! Onde: `enable.rs`.

use crate::cli::args::Cliente;
use crate::conector::{Auth, Conector, Transporte};
use serde_json::{json, Value};

/// A receita de ativação de um conector num cliente.
#[derive(Debug, Clone, PartialEq)]
pub struct Receita {
    /// `argv` de `claude mcp add ...` (só claude-code), com `${VAR}` literal.
    pub argv: Option<Vec<String>>,
    /// Trecho JSON a colar (só cursor/vscode).
    pub trecho: Option<Value>,
    /// Arquivo onde colar o trecho.
    pub arquivo: Option<&'static str>,
}

/// O quê: nome do cliente estável (JSON). Onde: `enable`/`disable`.
pub fn cliente_slug(c: Cliente) -> &'static str {
    match c {
        Cliente::ClaudeCode => "claude-code",
        Cliente::Cursor => "cursor",
        Cliente::Vscode => "vscode",
    }
}

/// O quê: arquivo de config MCP do cliente (None p/ claude-code). Onde: receita e `disable`.
pub fn arquivo_de(c: Cliente) -> Option<&'static str> {
    match c {
        Cliente::ClaudeCode => None,
        Cliente::Cursor => Some("~/.cursor/mcp.json"),
        Cliente::Vscode => Some(".vscode/mcp.json"),
    }
}

/// O quê: transporte no vocabulário dos clientes (`http`/`sse`). Onde: receitas.
fn transporte(c: &Conector) -> &'static str {
    match c.transporte {
        Transporte::Sse => "sse",
        _ => "http",
    }
}

/// O quê: o header de auth com a REFERÊNCIA da chave, `Bearer <ref>`, ou `None` sem chave.
/// Onde: receitas; `ref_` é o texto de expansão (`${VAR}`, `${env:VAR}`, ...).
fn header_ref(c: &Conector, ref_: impl Fn(&str) -> String) -> Option<String> {
    match c.auth {
        Auth::BearerChave { env_var, .. } => Some(format!("Bearer {}", ref_(env_var))),
        _ => None,
    }
}

/// O quê: o `argv` de `claude mcp add` com `${VAR}` literal (o shell expande). Onde: `receita`.
fn argv_add(c: &Conector, url: &str, header: Option<String>) -> Vec<String> {
    let mut v: Vec<String> = ["claude", "mcp", "add", "--transport", transporte(c), c.id, url]
        .iter()
        .map(|s| s.to_string())
        .collect();
    if let Some(h) = header {
        v.push("--header".into());
        v.push(format!("Authorization: {h}"));
    }
    v
}

/// O quê: a receita do `enable` para o cliente. `None` se o conector não tem endpoint.
/// Onde: `enable`.
pub fn receita(c: &Conector, cliente: Cliente) -> Option<Receita> {
    let url = c.endpoint?;
    Some(match cliente {
        Cliente::ClaudeCode => {
            let h = header_ref(c, |v| format!("${{{v}}}"));
            Receita { argv: Some(argv_add(c, url, h)), trecho: None, arquivo: None }
        }
        Cliente::Cursor => {
            let mut s = json!({"url": url});
            if let Some(h) = header_ref(c, |v| format!("${{env:{v}}}")) {
                s["headers"] = json!({"Authorization": h});
            }
            let t = json!({"mcpServers": {c.id: s}});
            Receita { argv: None, trecho: Some(t), arquivo: arquivo_de(cliente) }
        }
        Cliente::Vscode => Receita {
            argv: None,
            trecho: Some(trecho_vscode(c, url)),
            arquivo: arquivo_de(cliente),
        },
    })
}

/// O quê: trecho do VS Code com `${input:...}` (pede a chave uma vez, com campo de senha).
/// Onde: `receita`.
fn trecho_vscode(c: &Conector, url: &str) -> Value {
    let input_id = format!("{}-api-key", c.id);
    let mut s = json!({"type": transporte(c), "url": url});
    let mut t = json!({});
    if let Some(h) = header_ref(c, |_| format!("${{input:{input_id}}}")) {
        s["headers"] = json!({"Authorization": h});
        let desc = match c.auth {
            Auth::BearerChave { env_var, .. } => format!("{} ({env_var})", c.nome),
            _ => c.nome.to_string(),
        };
        t["inputs"] = json!([{"type": "promptString", "id": input_id, "description": desc, "password": true}]);
    }
    t["servers"] = json!({c.id: s});
    t
}

/// O quê: o `argv` REAL do `--aplicar`, com o valor da chave no header (argv puro, sem shell).
/// Onde: `enable --aplicar`. É o único ponto que recebe o valor do segredo; nunca é impresso.
pub fn argv_add_real(c: &Conector, segredo: Option<&str>) -> Option<Vec<String>> {
    let url = c.endpoint?;
    let h = match (c.auth, segredo) {
        (Auth::BearerChave { .. }, Some(s)) => Some(format!("Bearer {s}")),
        _ => None,
    };
    Some(argv_add(c, url, h))
}

/// O quê: `claude mcp remove <id>`. Onde: `disable`.
pub fn argv_remove(c: &Conector) -> Vec<String> {
    ["claude", "mcp", "remove", c.id].iter().map(|s| s.to_string()).collect()
}

/// O quê: formata o `argv` como linha de shell para COPIAR: argumentos com espaço ou `$` vão em
/// aspas duplas (a `${VAR}` continua expansível); o resto sai cru. Onde: saída humana.
pub fn linha_shell(argv: &[String]) -> String {
    let seguro = |a: &str| {
        !a.is_empty() && a.chars().all(|c| c.is_ascii_alphanumeric() || "_./:=@%+-".contains(c))
    };
    let citar = |a: &String| {
        if seguro(a) {
            return a.clone();
        }
        let mut o = String::from("\"");
        for ch in a.chars() {
            if matches!(ch, '"' | '\\' | '`') {
                o.push('\\');
            }
            o.push(ch);
        }
        o.push('"');
        o
    };
    argv.iter().map(citar).collect::<Vec<_>>().join(" ")
}
