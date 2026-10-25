# Mandrake v0: o harness do Claude Code reproduzível em qualquer máquina

Repositório: https://github.com/brennercruvinel/mandrake (privado). Branch da tarefa: `tiamati/v0`. PR para `main`; o merge é do Brenner.
Tech lead: Tiamati, junto com o Brenner. Agente A (proxy e medição) e agente B (telas e wizard) trabalham em worktrees próprias. O Tiamati revisa, testa e faz o merge deles na `tiamati/v0`.
Legenda de dono: T = Tiamati, A = agente A, B = agente B, BR = Brenner.

## Decisões fechadas com o Brenner (2026-10-24)

- O nome é Mandrake. O `agent/mcps/mandrake` (REA) convive por enquanto; o Mandrake vai ser o superagente de harness e segurança.
- Uso sempre interno, Mac e Linux, sem nenhum caminho de máquina no código.
- Sem ADR: `docs/` explica o funcionamento e as possibilidades. → substituída em 2026-10-25 pela [ADR-0001](../adr/workflows/0001-decision-records-and-changelog-fragments.md)
- `core` é o system prompt sem o qual o agente perde capacidade; `soul` é um exemplo de agente com alma, de código e alto nível, que não copia o Tiamati.
- O prompt-trim é obrigatório ([ADR-0003](../adr/promptcut/0003-prompt-trim-always-on-safeguards-kept.md)). O fancy e o tui-embed são opcionais no wizard. O tui-slider entra nas telas.
- A POC prova a reconstrução: guardar a configuração atual em zip, mandar o que o Mandrake recria para a lixeira, instalar pelo Mandrake e voltar ao mesmo resultado.

## Contratos (T, antes de A e B começarem)

- C1 T: workspace em `rust/` ([ADR-0002](../adr/structure/0002-rust-workspace-in-rust.md)) com os crates `cli` (binário `mandrake`), `layout`, `proxy`, `meter` e `tui`; Rust 1.99, edition 2024. → **feito**
- C2 T: diretórios XDG nos dois sistemas, todos sobrescrevíveis por variável: config `~/.config/mandrake`, dados `~/.local/share/mandrake`, estado `~/.local/state/mandrake`. `MANDRAKE_HOME` isola tudo para teste. → **feito**
- C3 T: `meter::Event`, uma linha JSONL por requisição. Campos: id, ts, mode (`trim` ou `pass`), path, status, ms, bytes antes/depois, tokens antes/depois (via `count_tokens`), usage (input, output, cache read/create) e cortes por tipo. → **feito**
- C4 T: perfil `mandrake.toml`, escrito pelo wizard e lido por `setup --profile`. Contém a raiz de desenvolvimento, o destino de cada item, o proxy, os plugins, os souls e as skills. → **feito**
- C5 T: catálogo `cfgs/setup/items.toml`, com um item por entrada do `~/.claude`: id, caminho, tipo, grupo, descrição e se é sensível. → **feito (23 itens)**

## Layout e setup (T)

- L1 T: `mandrake capture` lê o `~/.claude` atual (symlinks e alvos) e escreve um perfil. → **feito**
- L2 T: `mandrake link plan` mostra o diff entre o estado atual e o perfil, sem tocar em nada. → **feito**
- L3 T: `mandrake link apply` cria os links. O que existe no caminho vai para o backup em `state/backups/<ts>`; nada é apagado, e cada passo fica no journal. → **feito**
- L4 T: `mandrake link rollback` desfaz o último apply pelo journal. → **feito (teste de unidade; não usado no real)**
- L5 T: o merge do `settings.json` grava só as chaves do Mandrake (`ANTHROPIC_BASE_URL`, `CLAUDE_CODE_PLUGIN_DIRS`) e preserva o resto. → **feito**
- L6 T: o trecho de shell `cfgs/shell/mandrake.sh` (bash e zsh) entra no rc por uma linha marcada e idempotente, e traz o `claude invoke <soul>` genérico. → **feito**
- L7 T: plugins. O prompt-trim é copiado para `data/` do Mandrake e entra por `PLUGIN_DIRS`. Fancy e tui-embed entram pelo marketplace `mandrake`, pulando um plugin de mesmo nome que já esteja ativo por outro marketplace. → **feito (fancy e tui-embed pulados: já ativos pelo marketplace tiamati)**
- L8 T: skill `invoke` opcional, com o comando da memória configurável. Sem memória, ela avisa e não quebra. → **feito (o invoke da memória dos monstros foi mantido)**
- L9 T: `install.sh` na raiz confere cargo, compila, instala o binário e chama `mandrake setup`. → **feito**
- L10 T: `mandrake doctor` confere links, proxy no ar, porta, settings, plugins e versão do Claude Code. → **feito**

## Proxy e medição (A)

- P1 A: proxy em hyper/tokio no `127.0.0.1:<porta>`, com upstream configurável e repasse em stream. → **feito**
- P2 A: regras de trim em `cfgs/proxy/trim.toml`: identidades do system, `<system-reminder>`, campos removidos. O padrão do produto remove só `metadata`; `safeguards` só sai por configuração explícita ([ADR-0003](../adr/promptcut/0003-prompt-trim-always-on-safeguards-kept.md)). → **feito**
- P3 A: modo `pass` (sem trim) na mesma base, para o A/B e o log "antes". → **feito**
- P4 A: corrige os achados da auditoria: abortar o upstream quando o cliente cai, não injetar JSON num SSE já aberto, tirar o `</system-reminder>` órfão e rotacionar o log. → **feito**
- P5 A: `count_tokens` do corpo antes e depois, com a credencial da própria requisição, assíncrono, sem atrasar a resposta. → **feito**
- P6 A: lê o `usage` do SSE (message_start e message_delta) e grava o evento C3. → **feito**
- P7 A: auditoria opt-in. Grava `audit/<id>.before.json` e `.after.json` com permissão 600, sem headers, redigindo padrões de segredo, com retenção configurável. → **feito**
- P8 A: serviço gerado a partir de modelo: launchd no Mac e `systemd --user` no Linux. `mandrake proxy install|uninstall|status`. → **feito (launchd testado no real; systemd só na geração do texto)**
- P9 A: `mandrake bench`. Repassa as fixtures de `tests/fixtures` pelo trim e mede antes e depois pelo `count_tokens`, num relatório JSON e Markdown. O A/B ao vivo com `claude -p` é opcional e tem teto. → **feito (sem credencial de API, só bytes)**
- P10 A: testes do trim sobre fixtures redigidas e teste de stream com upstream falso. → **feito (32 testes)**

## Telas e wizard (B)

- U1 B: `mandrake watch`, painel dividido ao vivo a partir do JSONL: à esquerda `pass` (sem trim), à direita `trim`, com tokens por requisição, economia acumulada e cache. → **feito**
- U2 B: `mandrake hex <id>`, hex lado a lado do antes e depois, com as regiões cortadas destacadas e o tui-slider para percorrer o offset. → **feito (diff por linhas e depois bytes: exato, cerca de 2 ms)**
- U3 B: `mandrake bench --view`, que mostra o relatório do P9 com barras de economia. → **feito**
- U4 B: `mandrake setup`, wizard item a item sobre o catálogo C5: manter, linkar para a pasta de desenvolvimento ou copiar. Em seguida plugins, proxy e souls. Sai como perfil C4 e chama o L3. → **feito (abre e sai num pty; não percorrido até o fim)**
- U5 B: modo `--embed` do `watch`, para abrir no painel do tui-embed dentro do Claude Code. → **feito (o embed num painel real do tui-embed não foi testado)**
- U6 B: testes de render com o `TestBackend` do ratatui. → **feito (30 testes)**

## Conteúdo e docs (T)

- D1 T: `data/prompt/cores/coding.md`, o core enxuto que devolve o essencial cortado (uso de tools, cuidado com ações, git). → **feito**
- D2 T: `data/prompt/souls/<nome>/` com o SOUL.md de um agente de código de alto nível, com voz própria, e o `settings.json` dele. → **feito (Rhizo)**
- D3 T: `docs/`: como funciona (o caminho do prompt), instalação, perfil e catálogo, medição e auditoria, possibilidades. → **feito**
- D4 T: README curto com a primeira execução em três comandos. → **feito**

## POC de reconstrução (T, na máquina do Brenner)

- R1 T: teste completo num `MANDRAKE_HOME` e `HOME` temporários (setup, links, proxy em outra porta, doctor), antes de tocar no real. → **feito (tool/tests/e2e/setup.sh, 18 checks)**
- R2 T: `mandrake capture` do real vira `~/.config/mandrake/mandrake.toml`, conferido contra os 22 links. → **feito (23 links)**
- R3 T: zip de backup fora do git (`~/mandrake-backup-<ts>.zip`, 600) com o `~/.claude` (links e arquivos de config sem dados de sessão), as linhas do `~/.zshrc`, o plist do proxy, o `settings.json` e o mapa de links. → **feito (apps/mandrake/.backup/20261024-185330.zip)**
- R4 T: numa única chamada, para nenhuma sessão aberta ficar sem API nem transcript: manda para a lixeira os links do `~/.claude`, o plist e a linha do zshrc; desliga o proxy antigo; roda `install.sh` com o perfil; sobe o proxy novo na 8787. → **feito (lixeira: ~/.Trash/mandrake-rebuild-20261024-191618)**
- R5 T: confere que o `doctor` passa, que os links batem com o mapa de antes e que uma sessão nova (`claude -p`) responde pelo proxy novo com o trim. → **feito (doctor 11 ok; claude invoke tiamati respondeu pelo proxy novo)**
- R6 T: demonstração com `watch` e `hex` ao vivo, o bench com antes e depois e o log de auditoria guardado. → **feito (ab.sh: 20.449 → 12.680 tokens, 38%)**

## Entrega

- E1 T: `cargo fmt`, `clippy -D warnings` e `test` no workspace, e build no Linux (hf-cabo) se o Brenner liberar o acesso. → **parcial: fmt, clippy e test limpos; build no Linux pendente**
- E2 T: PR `tiamati/v0` para `main` com `[POCS]` e a validação em lista. → **feito**
- E3 T: atualizar este plano marcando cada item feito, com a evidência. → **feito**

## Em aberto depois da v0

- O1: Em sessões longas, o `count_tokens` do corpo cortado veio maior que o do original (252.326 → 264.888), enquanto num `claude -p` curto veio menor (12.730 → 12.686). A hipótese ainda não testada é que a API trate de outro jeito, na autenticação por assinatura, um `system` sem o bloco de identidade. Próximo passo: auditar uma requisição longa e repetir o `count_tokens` só com o bloco de identidade de volta.
- O2: Build e `setup` no Linux, com systemd `--user`.
- O3: Percorrer o wizard até o fim num terminal real, e testar o `watch --embed` num painel do tui-embed.
- O4: Traduzir o `about` do catálogo, que hoje aparece em inglês no wizard.
