# Auditoria visual dos comandos do Mandrake no Linux, 2026-10-25

Escrita pelo Tiamati a pedido do Brenner no #3: cada comando e cada tela gravados pelo anina, um por
linha, olhando para quem começa (vibe coder) sem perder quem vive no terminal. A Quimera faz a mesma
passada no macOS com as mesmas tapes; os dois lados ficam em `docs/img/e2e/cli/<plataforma>/`.

- Máquina: Linux x86_64 (Ubuntu), `main` em `7b82f23`, anina compilado lá.
- Cada tape roda em `tool/tests/e2e/anina/cli/env.sh`: HOME, `MANDRAKE_HOME` e `CLAUDE_CONFIG_DIR`
  temporários, `claude` substituído por um stub que imprime o comando, e, quando a tela precisa de
  tráfego, uma API falsa local (`fake-upstream.py`, cerca de 1 token a cada 4 bytes). Nenhuma
  requisição real, nenhum `mandrake bench` de verdade.
- Por causa do `MANDRAKE_HOME`, o serviço do proxy nunca é carregado: `FAIL proxy` no doctor e
  "DESLIGADO" no status são efeito do isolamento, não defeito.
- Texto de cada tela: `tool/tests/e2e/anina/logs/cli/linux/<tape>.snaps.txt`. Rodar de novo:
  `tool/tests/e2e/anina/cli/record.sh linux` (precisa de `anina` e `python3` no PATH).

Severidade: **bug** quebra ou engana; **ux** funciona, mas quem começa se perde; **polish** acabamento.

## Por comando

### `mandrake` (sem argumentos) · ux · `status.svg`

- Vê: antes de instalar, duas linhas e "Rode: mandrake setup". Depois, as duas camadas explicadas.
- Quebra: nada.
- Confunde: o próximo passo sugerido é `mandrake bench`, que gasta requisições da conta ("Ainda não
  mediu: rode `mandrake bench` (2 requisições curtas por prompt, pela sua sessão)"), mesmo depois de
  requisições terem passado pelo proxy. `hex` falta na linha "Comandos". "passaram 1 requisições".
- Sugestão: sugerir `mandrake doctor` ou uma medição que não gasta, e listar todos os comandos.

### `mandrake --help` · polish · `help.svg`

- Vê: descrições em português, mas "Usage", "Commands", "Options", "Print help" em inglês. Em 100
  colunas as descrições quebram no meio da palavra ("se / m perguntar", "d / octor").
- Confunde: nove comandos no mesmo nível, sem separar o que se usa no primeiro dia do avançado.
- Sugestão: agrupar (começar: `setup`, `doctor` · ver: `watch`, `bench` · avançado: `hex`, `audit`,
  `link`, `proxy`, `capture`) e traduzir os rótulos.

### `mandrake setup` (tela cheia) · bug · `setup-wizard.svg` (80x24), `setup-wizard-wide.svg` (140x40)

- Vê: seis passos com a trilha "1 Pasta › 2 Itens › … › 6 Resumo"; Esc sai com "setup: nada mudou".
- Quebra:
  - o passo 3 descreve o tui-embed como "painel dentro do Claude Code para o mandrake watch --embed",
    o texto que o #5 corrigiu no marketplace (ele abre `/abtop` e `/gitlogue`);
  - numa máquina nova, o Resumo diz "Nenhuma mudança em relação ao perfil atual" e o Enter, em
    seguida, grava perfil, settings e o bloco do shell;
  - em 80x24 o texto corta em vez de quebrar: o passo 1 perde uma linha ("resolvidos a partir" pula
    para "então o mesmo perfil"), o 3 corta "obri", o 5 corta "a skill invoke tr", o rodapé do 2
    corta "Ente".
- Confunde: os 23 itens do `~/.claude` aparecem todos com "manter", inclusive os que não existem; a
  descrição de cada item vem em inglês ("Instructions read at the start of every session"); em 80x24
  cabem 8 itens e a caixa de detalhe cobre o resto; o item selecionado só se distingue pela cor.
- Sugestão: um modo simples, padrão, que mostra só o que existe e o que vai mudar; o resumo dizer o
  que a primeira instalação escreve.

### `mandrake setup --cli` · bug · `setup-cli.svg`

- Vê: 33 perguntas, 23 delas sobre itens do `~/.claude`, inclusive "hoje: não existe".
- Quebra: linkar `agents`, que tinha `reviewer.md`, para um destino que ainda não existe move a pasta
  para o backup e cria um destino vazio. Depois do setup, `ls -A ~/.claude/agents` não mostra nada; o
  arquivo só está em `state/backups/<stamp>/agents/reviewer.md`. Nada se perde, mas para quem começa
  os agentes sumiram.
- Confunde: "backuplink" no resumo; "1 links no perfil, 1 mudam agora".
- Sugestão: com o destino vazio ou inexistente, levar o conteúdo para o destino (ou perguntar), e o
  resumo dizer "agents: 1 arquivo vai para ~/dev/claude/agents"; pular itens que não existem.

### `mandrake setup --profile f --yes` · ux · `setup-yes.svg`

- Vê: aplica os dois links; a segunda execução diz "sem mudança" em tudo (idempotente).
- Quebra: o mesmo `backuplink agents` do `--cli`: `reviewer.md` vai para o backup.
- Confunde: uma linha solta, em inglês e fora do alinhamento, no meio da lista ("wrote
  …/mandrake-proxy.service (MANDRAKE_HOME is set: not loaded)"); "settings: settings env…" e "shell:
  shell: block added" repetem o nome do passo; caminhos completos quebram em 100 colunas.
- Sugestão: uma linha por passo, no mesmo idioma, com `~` no lugar do caminho completo.

### `mandrake capture` · polish · `capture.svg`

- Vê: o perfil em TOML cru, sem dizer o que é nem como salvar; `--out` responde "captured 2 links
  into my-profile.toml", em inglês.
- Confunde: o perfil capturado tem outra forma que o aplicado (`dev_root` ".../work/claude" e
  destinos "agents", contra ".../work" e "claude/agents"): mesmo resultado, comparação manual.
- Sugestão: sem `--out`, gravar num arquivo padrão e mostrar o caminho; manter o `dev_root` do perfil.

### `mandrake link plan|apply|rollback` · polish · `link.svg`

- Vê: com o link `commands` perdido, `plan` mostra "links: link commands -> …", `apply` o recoloca e
  `rollback` o desfaz. A recuperação funciona.
- Confunde: `plan` não diz que nada mudou nem qual comando aplica; "rolled back 1 steps"; o caminho do
  journal aparece para quem não sabe o que é journal.
- Sugestão: `plan` terminar com "para aplicar: mandrake link apply"; mensagens em português.

### `mandrake proxy status|run` · polish · `proxy.svg`

- Vê: `status` numa linha longa em inglês ("proxy: service file at … (MANDRAKE_HOME is set: not
  loaded); 127.0.0.1:… not answering"); `run` imprime "mandrake proxy on http://127.0.0.1:… ->
  http://127.0.0.1:… (trim)" e mais nada.
- Confunde: `run` não diz como parar nem como uma sessão passa a usá-lo.
- Sugestão: `status` em duas linhas curtas (serviço, porta) e o próximo passo.

### `mandrake watch` · bug · `watch.svg` (80x24), `watch-wide.svg` (140x40)

- Vê: dois painéis; o da direita atualiza ao vivo (3, depois 6 requisições); o da esquerda, "Sem
  Mandrake · pass", fica em 0 a sessão inteira.
- Quebra: o foco começa no painel vazio. ↓ e Enter não fazem nada, embora o rodapé prometa "Enter
  hex"; nas duas gravações a tela continua igual depois do Enter (`05-hex`, `02-hex`). Em 80x24 a
  hora vira "02:29:" e o caminho do `events.jsonl` e a linha de economia são cortados.
- Confunde: "corte do proxy: +98 tokens (3,5%)" usa "+" para uma economia; nada explica que o lado
  esquerdo só enche com `mandrake proxy run --pass`. O anina mede o rótulo da economia em 4,0:1 (abaixo
  de AA) e a barra verde sobre cinza em 1,86:1.
- Sugestão: foco inicial no lado com dados; o lado vazio dizer como enchê-lo ou sumir; "economizou 98
  tokens". É a tela natural para o lado a lado que o Brenner pediu.

### `mandrake hex <id>` · ux · `hex.svg` (80x24), `hex-wide.svg` (160x40)

- Vê: em 160 colunas, antes e depois lado a lado em hex, com a guia dos trechos cortados; `n` anda
  pelos trechos ("trecho 2/16 offset 0x510"). Em 80x24 os dois lados empilham e o rodapé corta em
  "Tab guia antes/de", escondendo "trecho N/16".
- Quebra: nada no caminho feliz.
- Confunde: exige um id de 24 caracteres tirado do `audit` (`mandrake hex $(mandrake audit | head -1 |
  cut -f1)`); "−423 B +0 B".
- Sugestão: `mandrake hex` sem id abre a última requisição auditada.

### `mandrake audit` · bug · `audit.svg`

- Vê: TSV cru com tabulações, sem cabeçalho, quebrando em 100 colunas; "saved 69 tokens" em inglês.
- Quebra: sem os corpos (apagados à mão aqui; a retenção de `audit_keep_days` deve levar ao mesmo
  estado, não conferido), a lista continua com os três ids e `hex` falha: "corpo de auditoria não
  encontrado: …/state/audit/<id>.before.json · Caused by: No such file or directory (os error 2)".
- Sugestão: listar só o que ainda tem corpo, ou marcar "corpo removido"; tabela com cabeçalho.

### `mandrake bench --view` · bug · `bench-view.svg` (80x24), `bench-view-wide.svg` (120x36)

- Vê: tabela por fixture e barras de economia; em 120 colunas, legível.
- Quebra: em 80x24 os cabeçalhos perdem as primeiras letras ("ytes antes", "tes depois", "ens antes",
  "ns depois"). Sem relatório, o erro é cru: "Error: lendo o relatório …/latest.json · Caused by: No
  such file or directory (os error 2)".
- Confunde: só números; não mostra o que foi cortado.
- Sugestão: cabeçalhos curtos; sem relatório, dizer como medir sem gastar (`--fixtures`); um painel com
  o system prompt antes e depois do corte.

### `mandrake doctor` · ux · `doctor.svg`

- Vê: uma linha `ok`/`FAIL` por conferência; com um link perdido e o bloco tirado do `~/.bashrc`, dois
  FAIL novos; `mandrake setup --yes` conserta os dois.
- Quebra: nada (o `FAIL proxy` é do isolamento).
- Confunde: nenhum FAIL diz o comando que conserta ("FAIL links: 1 as the profile says, differ:
  agents", "FAIL shell block in …/.bashrc"); português e inglês na mesma lista. A linha `souls` confere
  a pasta do checkout (`MANDRAKE_SRC`) enquanto o launcher usa a cópia instalada em `data/src`: com
  `MANDRAKE_SRC` ou um binário de dentro do checkout, o doctor confere outra pasta que a do shell.
- Sugestão: cada FAIL com "conserte com: <comando>", e talvez `mandrake doctor --fix`.

### `claude invoke <soul>` · polish · `invoke.svg`

- Vê: o aviso do proxy em português e o comando que o launcher monta (`claude --system-prompt-file
  /tmp/mandrake-soul-rhizo-<pid>.md --name Rhizo`); `claude` puro chega ao Claude Code sem
  argumentos a mais (o core das sessões sem soul entra pelo prompt-trim, que o stub não mostra).
- Confunde: os erros vêm em inglês ("claude invoke: which soul? (rhizo)", "'nobody' is not a soul");
  o aviso "proxy fora do ar" aparece em toda sessão de quem não instalou o serviço.

## Medição do anina (`anina audit --quick`, 4 terminais: 120x40, 60x20, sem cor, 16 cores)

| Tape | Estabilidade | Resposta | Legibilidade | Compatibilidade | Higiene |
| --- | ---: | ---: | ---: | ---: | ---: |
| `bench-view-wide` | 100 | 100 | 100 | 100 | 100 |
| `setup-wizard-wide` | 100 | 100 | 100 | 100 | 100 |
| `hex-wide` | 100 | 80 | 100 | 100 | 100 |
| `watch-wide` | 100 | 63 | 96 | 100 | 100 |

A resposta de `hex` e `watch` inclui o tempo da preparação (setup, API falsa e proxy antes do prompt),
não só o do Mandrake. A legibilidade de `watch` é a linha de economia citada acima. Relatórios em
`tool/tests/e2e/anina/logs/cli/linux/<tape>.audit.txt`.

## O que isso diz sobre os pedidos do Brenner no #3

- **Passo a passo de instalação.** O `mandrake` numa máquina nova já diz "Rode: mandrake setup", mas o
  `--help` não tem ordem de uso. Três passos cabem nessa mesma tela: `mandrake setup`, abrir um shell
  novo, `mandrake doctor`.
- **Recuperação.** Os dois casos pedidos se recuperam hoje: link perdido com `mandrake link apply`
  (`link.svg`), bloco do shell apagado com `mandrake setup --yes` (`doctor.svg`). Nenhuma tela diz
  isso; o doctor é o lugar.
- **Bench lado a lado, sem trim contra com trim.** O `watch` já tem dois lados, mas o esquerdo fica
  vazio; o `bench --view` só mostra números; o `hex` mostra antes e depois lado a lado em bytes a
  partir de 160 colunas. Falta o texto do system prompt antes e depois, que os fixtures já têm.
- **Bench e teste automáticos para quem começa.** O `mandrake bench` padrão gasta requisições e é o
  que o status recomenda; a medição que não gasta (`--fixtures`) está escondida numa flag.
- **Hex para quem vive no terminal.** Funciona bem a partir de 160 colunas; em 80x24 precisa do
  rodapé inteiro e de abrir sem id.

## Limites

- Os números de token vêm da API falsa (cerca de bytes/4), não de `count_tokens` real.
- O stub de `claude` não abre o Claude Code: o `invoke` foi visto até o comando que o launcher monta.
- `proxy install|uninstall` não rodaram: carregariam um serviço systemd na conta da máquina.
- O tui-embed (`/abtop`, `/gitlogue`) fica para a prova do #5, num Claude Code real.
