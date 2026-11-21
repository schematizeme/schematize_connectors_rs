# schematize connectors

Catálogo de **conectores MCP com prévia** e **anúncios da casa** para o schematize.
A lib tem tipos, catálogo embutido e regras de anúncio (lógica **pura**); a prévia ao vivo está em
`src/mcp/` e a CLI `schematize-connectors` em `src/cli/`.

## Catálogo

`catalogo()` devolve uma fatia `&'static`, em ordem determinística: o conector **principal**
(`nebularws`) é sempre o primeiro e há exatamente um. Entradas: `nebularws`, `asender`
(MCP real, 32 ferramentas de prévia), `owiew`, `axuz` (endpoint pendente) e `canverly`
(sem MCP por enquanto). Nenhuma URL é inventada: endpoint pendente é `None`.

## Segurança por construção

- `Conector::validar()`: endpoint só `https://`, host igual ao domínio do conector ou
  subdomínio dele, sem userinfo, porta só 443, sem query/fragmento, ASCII estrito (anti-SSRF
  e anti-lookalike).
- Segredo **nunca** é gravado nem logado: só se referencia por **nome** de env var
  (`ASENDER_API_KEY`). `resolver_env` devolve um `Segredo` cujo `Debug`/`Display` não
  imprimem o valor; `mascarar` mostra no máximo `sk_l…wxyz`.

## Anúncios

`escolher(&Contexto)` é pura e determinística (sem `rand`, sem relógio). Regras: nunca em
modo JSON nem sem TTY; nunca com opt-out (`SCHEMATIZE_SEM_ANUNCIOS=1|true|yes|sim`); nunca de
conector já ativo; respeita expiração; no máximo 1 por sessão; o principal tem prioridade
(sessões pares); rótulo obrigatório `Anúncio · da casa` / `Ad · from us`.

> **Regra inegociável: anúncio NUNCA vai para prompt/contexto de agente.** Gasta token e é
> vetor de prompt injection. A única saída é `render_texto` (texto puro, para terminal).

Os anúncios atuais são **modelos** aguardando a copy do dono.

## Uso da CLI

Todos os comandos aceitam `--lang pt|en` (padrão: `SCHEMATIZE_LANG`, `LC_ALL`, `LC_MESSAGES`,
`LANG`; sem nada, inglês) e, onde faz sentido, `--json` (chaves estáveis, **nunca traduzido**:
os bytes são idênticos nos dois idiomas; falha em `--json` sai como `{"ok":false,"erro":{...}}`).

```sh
schematize-connectors list                       # tabela; ★ no principal (nebularws primeiro)
schematize-connectors preview asender            # lista EMBUTIDA, offline
schematize-connectors preview asender --ao-vivo  # consulta o servidor (tools/list, só leitura)
schematize-connectors enable asender             # IMPRIME o comando do Claude Code
schematize-connectors enable asender --cliente cursor|vscode   # imprime o trecho JSON
schematize-connectors enable asender --aplicar   # roda `claude mcp add` por você (só claude-code)
schematize-connectors disable asender [--aplicar]
schematize-connectors anuncios [--slot tela-inicial|estado-vazio|banner-cli] [--json]
schematize-connectors diag
```

- `preview` nunca chama ferramenta (`tools/call`). `--ao-vivo` lê a chave **só** da env var do
  conector (`ASENDER_API_KEY`); o nome aparece, o valor nunca. Ferramentas de escrita exigem chave
  `sk_*` (OAuth é só leitura).
- `enable` não altera nada sem `--aplicar`. O comando impresso usa a referência da variável
  (`--header "Authorization: Bearer ${ASENDER_API_KEY}"`, expandida pelo seu shell); Cursor usa
  `${env:VAR}` e VS Code `${input:...}`. Com `--aplicar` o `claude` guarda a chave na config
  dele (e ela fica visível na lista de processos por instantes: o aviso é impresso).
- Opt-out de anúncios: `SCHEMATIZE_SEM_ANUNCIOS=1`, `--sem-anuncios` ou o arquivo vazio
  `$XDG_CONFIG_HOME/schematize/connectors/sem-anuncios` (`~/.config/...`).

### Exit codes

`0` ok · `1` erro de uso/pedido recusado (id inexistente, `--aplicar` recusado, `claude` ausente) ·
`2` argumento inválido (clap) · `3` conector sem endpoint/sem MCP · `4` exige segredo ·
`5` falha de rede/protocolo · `6` timeout.

### Regras de anúncio na CLI

Um único anúncio do slot `BannerCli` ao fim do `list`, por `escolher` (rótulo obrigatório), e
**só** com terminal interativo, sem `--json` e sem opt-out. **Nunca** no prompt/contexto de agente,
**nunca** em `--json`, nunca em pipe. `anuncios` só audita (em `--json` devolve o id do candidato e
o motivo do bloqueio, jamais o texto).

### Pendências do dono

1. Endpoints MCP de nebularws, owiew e axuz (cadastrar em `src/catalogo.rs`; hoje exit 3).
2. Copy final dos anúncios (hoje modelos).
3. Criar o remoto e dar o push (o repo só tem commits locais).
4. Mensagens do clap (`--help`, erros de flag) saem em inglês; só a saída do app é pt/en.

## Desenvolvimento

```sh
cargo fmt && cargo clippy --all-targets -- -D warnings && cargo test
```

O CI remoto (`.github/workflows/ci.yml`) roda só sob `workflow_dispatch` ou em tag (`hml-*`, `v*`).
