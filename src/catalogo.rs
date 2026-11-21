//! Catálogo EMBUTIDO de conectores. `&'static`, ordem determinística, principal primeiro.

use crate::catalogo_asender::FERRAMENTAS as ASENDER;
use crate::conector::{Auth, Conector, Status, Transporte};

/// Entrada-base de projeto da casa sem MCP verificado (o resto é sobrescrito por entrada).
const PENDENTE: Conector = Conector {
    id: "",
    nome: "",
    dominio: "",
    resumo_pt: "",
    resumo_en: "",
    principal: false,
    transporte: Transporte::StreamableHttp,
    auth: Auth::Nenhuma,
    endpoint: None,
    ferramentas_estaticas: &[],
    status: Status::EndpointPendente,
};

/// Os conectores da casa. Invariante: `principal` primeiro e exatamente um.
static CATALOGO: [Conector; 5] = [
    Conector {
        id: "nebularws",
        nome: "NebulaRWS",
        dominio: "nebularws.com",
        resumo_pt: "Projeto da casa.",
        resumo_en: "A project from us.",
        principal: true,
        ..PENDENTE
    },
    Conector {
        id: "asender",
        nome: "Asender",
        dominio: "asender.net",
        resumo_pt: "Comunicação, marketing e atendimento para empresas.",
        resumo_en: "Communication, marketing and customer service for businesses.",
        auth: Auth::BearerChave { env_var: "ASENDER_API_KEY", prefixo: "sk_" },
        endpoint: Some("https://api.asender.net/mcp"),
        ferramentas_estaticas: &ASENDER,
        status: Status::Ativo("2026-11-21"),
        ..PENDENTE
    },
    Conector {
        id: "owiew",
        nome: "Owiew",
        dominio: "owiew.com",
        resumo_pt: "Do clique ao faturamento.",
        resumo_en: "From click to revenue.",
        ..PENDENTE
    },
    Conector {
        id: "axuz",
        nome: "Axuz",
        dominio: "axuz.net",
        resumo_pt: "Projeto da casa.",
        resumo_en: "A project from us.",
        ..PENDENTE
    },
    Conector {
        id: "canverly",
        nome: "Canverly",
        dominio: "canverly.net",
        resumo_pt: "Rede de portais editoriais.",
        resumo_en: "A network of editorial portals.",
        status: Status::SemMcp,
        ..PENDENTE
    },
];

/// Catálogo completo, em ordem determinística (principal primeiro).
///
/// O quê: devolve a fatia estática; zero alocação.
/// Onde: CLI (listar/prévia), `anuncio::escolher` e a UI do schematize.
pub fn catalogo() -> &'static [Conector] {
    &CATALOGO
}

/// Busca um conector pelo `id`. O quê: varredura linear (5 itens). Onde: CLI e anúncios.
pub fn buscar(id: &str) -> Option<&'static Conector> {
    CATALOGO.iter().find(|c| c.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conector::Status;
    use std::collections::HashSet;

    /// Checa "principal primeiro e único" sobre qualquer fatia (permite o self-check).
    fn ordem_ok(c: &[Conector]) -> bool {
        c.first().is_some_and(|p| p.principal) && c.iter().filter(|x| x.principal).count() == 1
    }

    #[test]
    fn principal_primeiro_e_unico() {
        assert!(ordem_ok(catalogo()));
        assert_eq!(catalogo()[0].id, "nebularws");
    }

    #[test]
    fn self_check_forca_falha_conhecida() {
        // Prova que o teste de ordem não está cego: catálogo embaralhado DEVE reprovar.
        let mut v = catalogo().to_vec();
        v.swap(0, 1);
        assert!(!ordem_ok(&v), "teste de ordem cego: aceitou principal fora do topo");
        let mut dois = catalogo().to_vec();
        dois[1].principal = true;
        assert!(!ordem_ok(&dois), "teste cego: aceitou dois principais");
        assert!(!ordem_ok(&[]));
    }

    #[test]
    fn ids_unicos_e_dominios_unicos() {
        let ids: HashSet<_> = catalogo().iter().map(|c| c.id).collect();
        let doms: HashSet<_> = catalogo().iter().map(|c| c.dominio).collect();
        assert_eq!(ids.len(), catalogo().len());
        assert_eq!(doms.len(), catalogo().len());
    }

    #[test]
    fn todo_conector_valida() {
        for c in catalogo() {
            assert_eq!(c.validar(), Ok(()), "{}", c.id);
        }
    }

    #[test]
    fn ativo_tem_endpoint_e_pendente_nao() {
        for c in catalogo() {
            match c.status {
                Status::Ativo(_) => assert!(c.endpoint.is_some(), "{}", c.id),
                _ => assert!(c.endpoint.is_none(), "{}", c.id),
            }
        }
    }

    #[test]
    fn asender_confere_com_a_documentacao() {
        let a = buscar("asender").expect("asender no catálogo");
        let f = a.ferramentas_estaticas;
        assert_eq!(f.len(), 32);
        assert_eq!(f.iter().filter(|x| x.escrita).count(), 3);
        assert_eq!(f.iter().filter(|x| x.nome.contains("_dns_")).count(), 2);
        let nomes: HashSet<_> = f.iter().map(|x| x.nome).collect();
        assert_eq!(nomes.len(), 32);
        assert!(f.iter().all(|x| x.nome.starts_with("asender_") && !x.descricao.is_empty()));
    }

    #[test]
    fn nebularws_nao_inventa_url() {
        assert_eq!(buscar("nebularws").map(|c| c.endpoint), Some(None));
        assert_eq!(buscar("inexistente"), None);
    }

    #[test]
    fn json_do_catalogo_e_estavel() {
        let a = serde_json::to_string(catalogo()).expect("serializa");
        let b = serde_json::to_string(catalogo()).expect("serializa");
        assert_eq!(a, b);
        assert!(a.starts_with("[{\"id\":\"nebularws\""));
    }
}
