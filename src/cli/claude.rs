//! Executor do `claude` (Claude Code): `argv` puro, sem shell, saída capturada.
//!
//! O quê: abstrai o processo filho para a CLI e para os testes (fake).
//! Onde: `enable --aplicar` e `disable --aplicar`.

use std::process::Command;

/// Resultado de uma execução.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Execucao {
    /// Código de saída (-1 se morto por sinal).
    pub codigo: i32,
    /// stdout + stderr do filho (a CLI higieniza antes de mostrar).
    pub saida: String,
}

/// Executor do `claude`.
pub trait Claude {
    /// O quê: `claude` está no PATH? Onde: checagem antes de `--aplicar`.
    fn disponivel(&self) -> bool;
    /// O quê: roda `claude <args...>` (argv puro). Onde: `--aplicar`. Erro = não conseguiu iniciar.
    fn rodar(&self, args: &[String]) -> Result<Execucao, String>;
}

/// Implementação real, sobre `std::process::Command`.
pub struct ClaudeReal;

/// O quê: procura um executável `nome` em `PATH`. Onde: `ClaudeReal::disponivel`.
fn no_path(nome: &str) -> bool {
    let Some(path) = std::env::var_os("PATH") else { return false };
    std::env::split_paths(&path).any(|d| d.join(nome).is_file())
}

impl Claude for ClaudeReal {
    /// O quê: varre o PATH por `claude`. Onde: `--aplicar`.
    fn disponivel(&self) -> bool {
        no_path("claude")
    }

    /// O quê: `Command::new("claude").args(args).output()`; sem shell. Onde: `--aplicar`.
    fn rodar(&self, args: &[String]) -> Result<Execucao, String> {
        let o = Command::new("claude").args(args).output().map_err(|e| e.to_string())?;
        let mut saida = String::from_utf8_lossy(&o.stdout).into_owned();
        saida.push_str(&String::from_utf8_lossy(&o.stderr));
        Ok(Execucao { codigo: o.status.code().unwrap_or(-1), saida })
    }
}
