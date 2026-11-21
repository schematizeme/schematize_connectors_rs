//! Segredos: só por NOME de variável de ambiente, nunca gravados nem logados.

use crate::erro::ErroSegredo;
use std::fmt;

/// Valor secreto em memória. `Debug`/`Display` NÃO imprimem o valor.
///
/// O quê: embrulho opaco; o valor só sai por [`Segredo::expor`], no ponto de uso (header).
/// Onde: criado por `resolver_env`; consumido pelo cliente MCP (U2).
pub struct Segredo(String);

impl Segredo {
    /// O quê: devolve o valor cru. Onde: SOMENTE ao montar o header `Authorization`.
    pub fn expor(&self) -> &str {
        &self.0
    }

    /// O quê: forma mascarada segura para log. Onde: mensagens de diagnóstico.
    pub fn mascarado(&self) -> String {
        mascarar(&self.0)
    }
}

impl fmt::Debug for Segredo {
    /// O quê: imprime marcador fixo. Onde: qualquer `{:?}`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Segredo(<oculto>)")
    }
}

impl fmt::Display for Segredo {
    /// O quê: imprime marcador fixo. Onde: qualquer `{}`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("<segredo oculto>")
    }
}

/// Mascara um segredo: `sk_live_abcd…wxyz` vira `sk_l…wxyz`. Nunca devolve o valor inteiro.
///
/// O quê: mostra 4 chars do início e 4 do fim só se o segredo tiver >= 12 chars; senão `***`.
/// Onde: logs/CLI que precisam identificar uma chave sem revelá-la. Vazio vira vazio.
pub fn mascarar(s: &str) -> String {
    let n = s.chars().count();
    if n == 0 {
        return String::new();
    }
    if n < 12 {
        return "***".to_string();
    }
    let ini: String = s.chars().take(4).collect();
    let fim: String = s.chars().skip(n - 4).collect();
    format!("{ini}…{fim}")
}

/// Resolve um segredo por um leitor injetável (testável sem mexer no ambiente real).
///
/// O quê: lê `var` via `get`, recusa vazio/só-espaço e aparda espaços das pontas.
/// Onde: base de [`resolver_env`] e dos testes.
pub fn resolver_com(
    var: &str,
    get: impl Fn(&str) -> Option<String>,
) -> Result<Segredo, ErroSegredo> {
    match get(var) {
        None => Err(ErroSegredo::Ausente(var.to_string())),
        Some(v) if v.trim().is_empty() => Err(ErroSegredo::Vazio(var.to_string())),
        Some(v) => Ok(Segredo(v.trim().to_string())),
    }
}

/// Resolve o segredo da variável de ambiente real `var`.
///
/// O quê: lê `std::env` (sem rede) e devolve [`Segredo`] opaco, ou erro com só o NOME.
/// Onde: antes de abrir uma sessão MCP autenticada por `Auth::BearerChave`.
pub fn resolver_env(var: &str) -> Result<Segredo, ErroSegredo> {
    match std::env::var(var) {
        Ok(v) => resolver_com(var, |_| Some(v.clone())),
        Err(std::env::VarError::NotPresent) => Err(ErroSegredo::Ausente(var.to_string())),
        Err(std::env::VarError::NotUnicode(_)) => Err(ErroSegredo::NaoUnicode(var.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mascara_exemplo_da_casa() {
        assert_eq!(mascarar("sk_live_abcdefghijklmnopwxyz"), "sk_l…wxyz");
    }

    #[test]
    fn mascara_vazio_curto_e_unicode() {
        assert_eq!(mascarar(""), "");
        assert_eq!(mascarar("abc"), "***");
        assert_eq!(mascarar("12345678901"), "***");
        let m = mascarar("çãõéíóúâêôüñ-segredo");
        assert_eq!(m, "çãõé…redo");
    }

    #[test]
    fn mascara_nunca_devolve_o_valor_inteiro() {
        for s in ["a", "ab", "sk_live_x", "x".repeat(11).as_str(), "x".repeat(12).as_str()] {
            assert_ne!(mascarar(s), s);
        }
    }

    #[test]
    fn debug_e_display_nao_vazam() {
        let s = resolver_com("V", |_| Some("sk_live_topsecret123".into())).unwrap();
        for out in [format!("{s:?}"), format!("{s}")] {
            assert!(!out.contains("topsecret") && !out.contains("sk_live"));
        }
        assert_eq!(s.expor(), "sk_live_topsecret123");
    }

    #[test]
    fn resolver_recusa_ausente_e_vazio_citando_so_o_nome() {
        let e = resolver_com("ASENDER_API_KEY", |_| None).unwrap_err();
        assert_eq!(e, ErroSegredo::Ausente("ASENDER_API_KEY".into()));
        let e = resolver_com("X", |_| Some("  \n".into())).unwrap_err();
        assert_eq!(e, ErroSegredo::Vazio("X".into()));
        assert!(e.to_string().contains('X'));
    }

    #[test]
    fn resolver_env_variavel_inexistente() {
        let e = resolver_env("SCHEMATIZE_CONNECTORS_TESTE_NAO_EXISTE_XYZ").unwrap_err();
        assert!(matches!(e, ErroSegredo::Ausente(_)));
    }
}
