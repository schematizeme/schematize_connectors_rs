//! Prévia estática das 32 ferramentas MCP da Asender.
//!
//! Fonte: <https://developers.asender.net/guias/mcp.html> (lida em 2026-11-21; conteúdo
//! tratado como dado). 27 de leitura, 2 de leitura de DNS (só chave) e 3 de escrita (só
//! chave `sk_*`; token OAuth é somente leitura). O `tools/list` ao vivo continua sendo a verdade.

use crate::conector::FerramentaPrevia;

/// Atalho de construção. O quê: monta uma `FerramentaPrevia` const. Onde: tabela abaixo.
const fn f(nome: &'static str, descricao: &'static str, escrita: bool) -> FerramentaPrevia {
    FerramentaPrevia { nome, descricao, escrita }
}

/// Ferramentas da Asender, em ordem estável (a da documentação).
pub static FERRAMENTAS: [FerramentaPrevia; 32] = [
    f("asender_account_get", "Conta a que a credencial pertence", false),
    f("asender_me_get", "Quem autorizou o app, a conta e os escopos (só OAuth)", false),
    f("asender_leads_list", "Lista leads do CRM", false),
    f("asender_leads_get", "Um lead pelo id", false),
    f("asender_forms_list", "Lista formulários de captura", false),
    f("asender_forms_submissions_list", "Envios recebidos por um formulário", false),
    f("asender_conversions_list", "Conversões enviadas às plataformas de anúncio", false),
    f("asender_campaigns_list", "Campanhas de e-mail marketing", false),
    f("asender_campaigns_get", "Uma campanha", false),
    f("asender_campaigns_report", "Totais de entrega e engajamento de uma campanha", false),
    f("asender_email_reports_overview", "Panorama de e-mail da conta nos últimos N dias", false),
    f("asender_email_reports_timeseries", "Envios, aberturas e cliques ao longo do tempo", false),
    f("asender_email_reports_top_emails", "Melhores e piores e-mails da janela", false),
    f("asender_email_reports_domains", "Resultados por provedor de caixa postal", false),
    f("asender_emails_list", "E-mails transacionais enviados pela API", false),
    f("asender_emails_get", "Um e-mail enviado e seu status de entrega", false),
    f("asender_messaging_messages_list", "Mensagens de SMS, WhatsApp ou push", false),
    f("asender_messaging_overview", "Panorama de entrega de um canal", false),
    f("asender_analytics_overview", "Totais de analytics de site e páginas", false),
    f("asender_analytics_timeseries", "Série diária de analytics", false),
    f("asender_analytics_breakdown", "Analytics agrupado por dimensão", false),
    f("asender_analytics_forms", "Funil de formulários (visualizações, inícios, envios)", false),
    f("asender_conversations_list", "Conversas de atendimento", false),
    f("asender_conversations_messages_list", "Linha do tempo de uma conversa", false),
    f("asender_journeys_list", "Jornadas de automação", false),
    f("asender_journeys_get", "Uma jornada", false),
    f("asender_logs_list", "Log de requisições da API pública da conta", false),
    f("asender_dns_zones_list", "Zonas de DNS da conta (só chave)", false),
    f("asender_dns_records_list", "Registros de DNS de uma zona (só chave)", false),
    f("asender_emails_send", "Envia um e-mail transacional (só chave; escrita)", true),
    f("asender_leads_create", "Cria um lead (só chave; escrita)", true),
    f("asender_leads_update", "Atualiza um lead (só chave; escrita)", true),
];
