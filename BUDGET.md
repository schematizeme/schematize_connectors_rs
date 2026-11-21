# BUDGET

- A lib só faz rede na prévia ao vivo (`src/mcp/`, via binário `curl`, sem dependência nova); o resto só lê env var por nome.
- Catálogo e anúncios são `&'static`: zero alocação para listar; `escolher` aloca dois `Vec`
  de até 5 referências por chamada (uma vez por sessão).
- RSS irrelevante (dezenas de KB). Dependências: `serde` e `serde_json`.

## Prévia ao vivo (`src/mcp/`)

- teto de memória da prévia: 9 MiB (padrão) = `8 x max_bytes + 1 MiB`, com `max_bytes` = 1 MiB
  por resposta (buffer do stdout + texto + árvore JSON + textos sanitizados; o pico é de uma
  resposta por vez). Validado por `tests/mcp_budget.rs`; muda junto com `PreviaViva::max_bytes`.
- Tempo: 10 s no total (todas as requisições), `curl --max-time` + kill duro do filho.
- Saída limitada: no máximo 200 ferramentas, 10 páginas, descrição de 300 chars.

## CLI (`src/cli/`, binário `schematize-connectors`)

- Dependência nova: `clap` 4 (derive, sem features default: std, derive, help, usage,
  error-context, suggestions). Mantém o parse alinhado ao resto da casa; o custo é pequeno.
- Medido em release (strip, lto thin): binário ~1,2 MiB; RSS de `list` ~12 MiB (teto: 32 MiB
  idle). `list`/`preview` offline não alocam além de strings de saída.
- Sem rede fora de `preview --ao-vivo` (U2). `--aplicar` é o único caminho que cria processo.
