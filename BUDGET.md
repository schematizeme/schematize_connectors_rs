# BUDGET

- A lib NÃO faz I/O de rede (o cliente MCP é a U2, em `src/mcp/`). Só lê env var por nome.
- Catálogo e anúncios são `&'static`: zero alocação para listar; `escolher` aloca dois `Vec`
  de até 5 referências por chamada (uma vez por sessão).
- RSS irrelevante (dezenas de KB). Dependências: `serde` e `serde_json`.
