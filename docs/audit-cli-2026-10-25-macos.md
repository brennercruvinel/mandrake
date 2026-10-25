# Auditoria visual da CLI e da TUI, macOS, 2026-10-25

Escrita pela Quimera, a pedido do Brenner na #3 (01:56 e 02:08), em par com a auditoria do Linux
(`docs/audit-cli-2026-10-25-linux.md`). Cada bloco separa o que foi visto do que é sugestão.

Ambiente: macOS 27.0.1 arm64, `/bin/bash` 3.2.57, zsh como shell do usuário, rustc 1.99.0, `main`
em `7b82f23`, binário de debug. Terminal de 100x30 e 120x30, e 80x24 onde a tela muda. Tudo numa
HOME descartável (HOME, MANDRAKE_HOME, CLAUDE_CONFIG_DIR e XDG\_\* temporários, `claude` simulado,
nenhuma variável `MANDRAKE_*` herdada). A API foi substituída por um servidor local na porta 18788
(`count_tokens` ≈ bytes/4), então os números de tokens abaixo são só de forma, não de economia real.
Nenhuma requisição saiu da máquina, e o `bench` de verdade não rodou.

Severidade: **bug** quebra ou mente; **ux** funciona, mas um iniciante se perde; **polish** acabamento.

## install.sh

Gravação: pendente do tape compartilhado.

- Severidade: bug
- O que um iniciante vê: `installed ~/.local/bin/mandrake` e `note: add ~/.local/bin to PATH`,
  depois o wizard. A #3 manda rodar `exec $SHELL` e `mandrake`.
- O que quebra: o `/etc/paths` do macOS não tem `~/.local/bin`, e o bloco que o setup escreve no
  `.zshrc` só carrega `mandrake.sh` (`rust/layout/src/shell.rs:24`), sem mexer no PATH. Num Mac novo,
  `mandrake` e `mandrake doctor` dão `command not found` logo depois de uma instalação sem erros.
- Conferido no código (`install.sh:20`, `shell.rs:24`) e no `/etc/paths` deste Mac; o `install.sh` não
  rodou aqui, porque instala na HOME real.
- Sugestão: o bloco do shell acrescenta o prefixo do binário ao PATH quando ele não estiver lá, e o
  resumo final repete a linha se ainda faltar.

## mandrake

Gravação: pendente do tape compartilhado.

- Severidade: ux
- O que um iniciante vê: um texto claro sobre as duas camadas, o proxy `DESLIGADO` e
  `Ainda não mediu: rode mandrake bench (2 requisições curtas por prompt, pela sua sessão)`.
- O que quebra: depois de `mandrake bench --fixtures`, que gravou `state/bench/latest.json`, o
  resumo continua dizendo `Ainda não mediu`.
- O que confunde: o único próximo passo sugerido gasta requisições da conta.
- Sugestão: oferecer primeiro a medição sem custo (`bench --fixtures`) e contar o bench de fixtures
  no resumo.

## mandrake --help

Gravação: pendente do tape compartilhado.

- Severidade: polish
- O que um iniciante vê: descrições em português, boas e curtas.
- O que confunde: o que vem do clap fica em inglês (`Usage`, `Commands`, `Print help`); `hex
  --snapshot` não tem descrição.
- Sugestão: traduzir os textos do clap ou deixar a CLI toda num idioma só.

## mandrake setup (tela cheia)

Gravação: pendente do tape compartilhado.

- Severidade: bug
- O que um iniciante vê: seis passos com trilha no topo (`1 Pasta › 2 Itens › …`), uma ajuda curta
  em cada um e as teclas no rodapé. É a melhor tela para quem está começando.
- O que quebra: numa máquina sem perfil, o passo 6 diz `Nenhuma mudança em relação ao perfil
  atual.`, e o Enter em seguida escreve o perfil, `settings.json`, o bloco do `.zshrc` e o serviço
  do proxy. O resumo diz que nada vai acontecer, mas o setup aplica tudo isso.
- O que quebra: o passo 3 descreve o tui-embed como `painel dentro do Claude Code para o mandrake
  watch --embed`; o #11 tirou o `watch` da descrição do marketplace, mas o wizard continua com ela.
- O que confunde: no passo 2, os itens têm descrição em inglês (`Instructions read at the start of
  every session`) numa tela em português, e o quadro de detalhe esconde a seção `Sessão e
  histórico`. Em 100 e 80 colunas, o rodapé corta (`Shift+Ta`, `Ente`). No passo 4, a quebra de
  linha de `safeguards` volta para a coluna 0 (`mantê-lo`).
- Sugestão: o resumo lista o que vai ser escrito na primeira instalação; uma fonte única para a
  descrição do tui-embed.

## mandrake setup --cli

Gravação: pendente do tape compartilhado.

- Severidade: ux
- O que um iniciante vê: 32 perguntas e a confirmação. Vinte e três são sobre itens de `~/.claude`, um por um,
  mesmo os que `hoje: não existe`, com descrição em inglês.
- O que quebra: nada; Enter em tudo leva a `aplicar [s]:` com um resumo correto (`0 links no
  perfil`), e a entrada fechada não aplica (#6).
- O que confunde: `core (padrao, none ou um caminho) [padrao]` mistura idioma e falta o acento;
  `pasta [~/dev]` sugere uma pasta que pode não existir.
- Sugestão: perguntar só pelos itens que existem, e oferecer "manter todos" numa linha.

## mandrake setup --profile f --yes

Gravação: pendente do tape compartilhado.

- Severidade: polish
- O que um iniciante vê: uma linha `ok` por passo, `Erros: nenhum` e o próximo passo.
- O que confunde: rótulos repetidos (`ok   settings: settings env…`, `ok   shell: shell: block
  added`); a linha do proxy sai duas vezes, uma sem recuo e em inglês (`wrote …plist (MANDRAKE_HOME
  is set: not loaded)`) e outra em português; a saída do `claude plugin` aparece solta, sem recuo.
- Sugestão: um prefixo por passo e um idioma por linha.

## mandrake capture

Gravação: pendente do tape compartilhado.

- Severidade: polish
- O que um iniciante vê: o perfil em TOML no terminal, sem dizer o que fazer com ele.
- Sugestão: sem `--out` e com a saída num terminal, terminar com uma linha sobre `--out`.

## mandrake link plan | apply | rollback

Gravação: pendente do tape compartilhado.

- Severidade: ux
- O que um iniciante vê: `links: nothing to change`, `rolled back 1 steps`; sem perfil,
  `Error: no profile yet…` seguido de `Caused by: 0: reading … 1: No such file or directory (os
  error 2)`.
- O que confunde: inglês numa CLI em português; `1 steps`; o rollback não diz o que voltou nem de
  onde; a cadeia crua do erro.
- Sugestão: o rollback lista o que restaurou; erros sem `Caused by` para o caso comum.

## mandrake proxy status | run

Gravação: pendente do tape compartilhado.

- Severidade: ux
- O que um iniciante vê: `proxy: service not installed; 127.0.0.1:8787 answering`.
- O que quebra: o status olha só a porta. Numa HOME de teste, ele deu `answering` para o proxy de
  outra instalação na mesma porta; e sai 0 com o proxy fora do ar, então não serve em script.
- Sugestão: sair diferente de 0 quando não responde; dizer se quem responde é este proxy.

## mandrake watch

Gravação: pendente do tape compartilhado.

- Severidade: ux
- O que um iniciante vê: dois painéis. O da direita (`Com Mandrake · trim`) enche com as
  requisições; o da esquerda (`Sem Mandrake · pass`) fica em `aguardando requisições em` seguido de
  um caminho absoluto cortado.
- O que confunde: nada na tela diz como encher o lado esquerdo; o rodapé explica que
  `a diferença entre os lados aparece quando os dois tiverem requisições contadas`.
- Sugestão: é o lugar natural do "lado a lado" que o Brenner pediu. Um `mandrake bench` que mande
  o mesmo prompt pelos dois lados e mostre o `watch` cheio responde aos dois pedidos.

## mandrake hex <id>

Gravação: pendente do tape compartilhado.

- Severidade: ux
- O que um iniciante vê: um id de 24 caracteres copiado do `audit`, depois o dump hex de
  `before.json` em cima e de `after.json` embaixo, com um resumo útil no topo (`−189 B`, `cortes 1
  system-reminder(s) · campos metadata`).
- O que confunde: a ajuda diz `lado a lado`, mas os dois ficam empilhados e metade da largura fica
  vazia em 120 colunas; `mandrake hex` sem id não sugere os ids que existem.
- Sugestão: manter o hex para quem vive no terminal e abrir por padrão uma vista de texto do
  `system` antes e depois, que é o que o iniciante quer ver.

## mandrake bench --view

Gravação: pendente do tape compartilhado.

- Severidade: bug
- O que um iniciante vê: a tabela por fixture e as barras de economia.
- O que quebra: em 80x24, os cabeçalhos numéricos são cortados pela esquerda (`ytes antes`, `tes
  depois`, `ens antes`, `ns depois`), e a linha de aviso perde o fim (`…a economia abaixo é`).
- O que confunde: sem relatório, `Error: lendo o relatório …latest.json` com `Caused by`; o
  `bench --fixtures` só escreve `wrote …latest.json and …latest.md`, sem mostrar um número; o bench
  de fixtures mede só o corte do proxy (10,1% em bytes aqui), e não o do prompt-trim, onde fica
  quase toda a economia (31,9% medidos na #3).
- Sugestão: o `bench --fixtures` imprime o total e abre o `--view`; o aviso diz que a parte maior
  da economia não entra nessa medida.

## mandrake audit

Gravação: pendente do tape compartilhado.

- Severidade: polish
- O que um iniciante vê: sem auditoria, um texto claro sobre por que ela vem desligada. Com
  auditoria, linhas separadas por tab com ids longos e `saved 19 tokens` em inglês.
- Sugestão: colunas com cabeçalho, e o comando `mandrake hex <id>` pronto na última coluna.

## mandrake doctor

Gravação: pendente do tape compartilhado.

- Severidade: ux
- O que um iniciante vê: uma linha `ok` ou `FAIL` por checagem, e saída 1 quando algo falha.
- O que quebra: com o link de `rules` apagado e o bloco do `.zshrc` removido, o doctor acusa
  `FAIL links: 0 as the profile says, differ: rules` e `FAIL shell block in ~/.zshrc`, mas não diz
  como consertar. Só as linhas do tui-embed (#11) trazem o comando.
- Sugestão: cada `FAIL` termina com o comando que conserta (`mandrake link apply`, `mandrake setup
  --profile <perfil> --yes`); essa é a seção de recuperação que o Brenner pediu, dentro da própria
  ferramenta.

## claude invoke <soul>

Gravação: pendente do tape compartilhado.

- Severidade: polish
- O que um iniciante vê: `claude invoke` lista as souls (`rhizo`); um nome errado diz que não é soul
  e lista as que existem. O lançador monta `--system-prompt-file`, `--name Rhizo` e passa os flags.
- O que confunde: com o proxy fora do ar, toda chamada de `claude`, até `claude --version`, imprime
  `mandrake: proxy fora do ar em 127.0.0.1:<porta>…`. Quem escolheu não usar o proxy vê isso sempre.
- Sugestão: avisar só quando o serviço do proxy estiver instalado e não responder.

## Pedidos do Brenner (#3, 01:56)

- **Passo a passo de instalação:** depende do achado do `install.sh` (PATH) antes de tudo; os
  passos 1 a 6 do wizard já são um bom roteiro para escrever.
- **Recuperação (links perdidos, bloco do shell sumido):** `mandrake doctor` acha os dois, mas não
  diz o conserto; com o PATH no bloco, perder o bloco também faz o `mandrake` sumir.
- **Bench lado a lado (system prompt sem corte e cortado):** `watch` já tem os dois painéis e `hex`
  já tem antes e depois; falta uma vista de texto do `system` e um jeito de encher o lado esquerdo.
- **Bench e testes automáticos:** `bench --fixtures` roda sem custo e sem rede, mas mede só o
  proxy e não mostra o resultado na tela.
- **hex para quem vive no terminal:** está bom para esse público; o iniciante precisa da vista de
  texto acima.

## Limites desta passada

- Sem Claude Code de verdade: o `claude` foi simulado, então `FAIL plugin fancy: not enabled` logo
  depois do setup é efeito do stub, não um achado.
- Tokens contados por um servidor local (bytes/4), não pela API.
- `mandrake proxy install` e `uninstall` não rodaram: com `MANDRAKE_HOME`, o serviço é escrito e
  não carregado no launchd.
- O tui-embed (`/abtop`, `/gitlogue`) depende do `~/.claude` real e fica com a decisão do Brenner.
