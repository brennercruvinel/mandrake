# Como funciona

Toda vez que você manda uma mensagem, o Claude Code envia para a API muito mais do que ela: um
system prompt próprio de alguns milhares de tokens, a descrição longa de cada tool, lembretes
automáticos, data, pasta, plataforma. Você paga esse pacote em toda requisição. O Mandrake corta
esse pacote em duas camadas e põe no lugar um core curto que devolve só as regras de trabalho.

```mermaid
flowchart LR
  S["shell: claude / claude invoke soul"] --> C[Claude Code]
  C <-->|hooks| T["1. prompt-trim"]
  C -->|se o proxy responde| P["2. proxy 127.0.0.1:8787"]
  C -.->|se não responde| A
  P --> A[api.anthropic.com]
  P --> E[(events.jsonl)]
```

## 1. prompt-trim, dentro do Claude Code

Um plugin carregado por `CLAUDE_CODE_PLUGIN_DIRS`. Age antes de a requisição existir:

| Tira | Põe no lugar |
| - | - |
| as seções do prompt padrão do Claude Code (`intro`, `doing_tasks`, `tools`, `tone`…) | o core, `data/prompt/cores/coding.md`, nas sessões sem soul |
| 64 tipos de lembretes e anexos (data, ambiente, todo, modelo, idioma…) | nada; o core diz ao modelo que pergunte à máquina (`date`, `pwd`) |
| a descrição de cada tool (~16 mil caracteres) | o core resume os contratos que importam (Read antes de Edit, cuidados do Bash) |
| a lista de tipos de subagente | nada |

Reescreve os textos fixos dos anexos que ficam: `Contents of X (…):` vira `# X`, o cabeçalho da lista
de skills vira `Skills:`.
O que o corte nunca muda (tools, instruções do usuário, hooks, modos como plan mode) está em
[prompt-trim-invariants.md](prompt-trim-invariants.md), com um teste para cada regra.

## 2. proxy local, entre o Claude Code e a API

Alcança o que o plugin não alcança, no corpo JSON da requisição:

- tira as tags `<system-reminder>` em volta dos textos, mantendo o conteúdo;
- tira os campos do topo listados em `[proxy] strip_fields` (padrão: `metadata`);
- mede: conta os tokens antes e depois (`count_tokens`, com a credencial da própria requisição),
  e guarda o `usage` que a API cobrou. Uma linha por requisição em `~/.local/state/mandrake/events.jsonl`.

O corte da identidade do CLI no `system` existe (`cut_identity`), mas vem desligado: medido numa
sessão longa com login por assinatura, ele fez a requisição custar mais (269.338 → 291.637 tokens).

O proxy é opcional para funcionar. O `settings.json` não aponta para ele: o launcher do shell
define `ANTHROPIC_BASE_URL` por sessão, só quando o proxy responde. Proxy fora do ar = sessão
direto na API, com um aviso. Sessões abertas fora desse shell (IDE, app) não passam pelo proxy,
mas continuam com o prompt-trim.

## Ver o corte

`mandrake compare` roda um prompt pelo seu Claude Code duas vezes, sem e com o Mandrake, e mostra lado a lado o que cada um enviaria: o system prompt, as ferramentas e as mensagens, com o tamanho de cada parte. As duas requisições vão para uma API falsa local, que responde na hora: nada chega à Anthropic e nada é gasto. `--print` imprime a tabela. Os dois corpos ficam em `state/compare/run-<data>` por um dia; o compare seguinte apaga as pastas mais velhas (as do bench, em `state/bench`, ficam sete dias).

## Quanto economiza, medido

`mandrake bench` roda os mesmos prompts pelo `claude -p`, com o seu login, sem Mandrake (sem
prompt-trim, proxy só medindo) e com ele. Em 2026-10-24, nesta máquina:

| prompt | tokens sem | tokens com | economia |
| - | -: | -: | -: |
| resposta-curta | 20.784 | 14.152 | 31,9% |
| pergunta | 20.791 | 14.159 | 31,9% |
| codigo | 20.801 | 14.169 | 31,9% |

Onde estão os tokens que sobram, na requisição com Mandrake: o `CLAUDE.md` do usuário (~18,8 mil
caracteres, o maior pedaço), o core (~4,4 mil), os schemas das tools sem descrição. O Mandrake não
corta o `CLAUDE.md`: ele é seu. O proxy sozinho corta pouco (17 tokens); quase toda a economia é do
prompt-trim.

## Souls

`claude invoke <soul>` abre uma sessão com o prompt da soul como system prompt inteiro, o
`settings.json` dela e o gh dela (`~/.config/gh-<soul>`). O core vai antes da soul, a menos que o
perfil diga `core_only_plain = true` (souls que já se sustentam sozinhas). `Rhizo` é o exemplo
que vem com o Mandrake.

A mesma soul abre em outras CLIs, com o mesmo env e o `/invoke` como primeira mensagem:

| CLI | O prompt da soul vai em | Servidor |
| - | - | - |
| `claude invoke <soul>` | `--system-prompt-file`, com `--settings` | o do processo |
| `codex invoke <soul>` | `-c developer_instructions`; o env em `shell_environment_policy.set`, que o Codex não filtra | `--no-daemon` |
| `opencode invoke <soul>` | `instructions` em `OPENCODE_CONFIG_CONTENT` | `--standalone` |

Codex e OpenCode rodam por padrão num servidor de fundo compartilhado, aberto sem o env da soul: por
isso cada sessão de soul sobe o seu. `--yolo` vira a flag de cada CLI para rodar sem aprovações.

Uma sessão aberta sem soul (pelo IDE, pelo app, por `claude` sozinho) ganha uma com `/invoke <soul>`:
o `soul bind` guarda o par sessão → soul em `~/.local/state/mandrake/souls/` e devolve o prompt da
soul para o modelo. O env da soul não entra no processo que já está rodando; comandos que dependem
dele (git, gh) rodam com o prefixo que o bind mostra. Uma sessão que já é uma soul recusa outra.

Isso vale dentro de qualquer CLI. O `mandrake setup` roda `soul install`, que dá a cada CLI encontrada
as skills que só uma pessoa inicia (`disable-model-invocation: true`), como arquivos finos que chamam
`soul skill <nome>`:

| CLI | Onde fica | Como chama |
| - | - | - |
| OpenCode | `~/.config/opencode/commands/`, e o agente em `agents/` | `/invoke` |
| Codex | `~/.agents/skills/` | `$invoke` |

`soul skill` renderiza a skill na hora, então esses arquivos não envelhecem, e `soul session` diz qual
CLI está rodando e o id da sessão, pela variável que cada uma define para seus comandos. Uma sessão de
subagente do OpenCode responde pela raiz, lida do banco dele em modo leitura. O porquê está no
[ADR-0005](../adr/installer/0005-any-cli-reads-its-own-store-read-only.md).

O resto do que o Claude Code tem chega junto:

| O quê | OpenCode | Codex |
| - | - | - |
| `CLAUDE.md` | link em `~/.config/opencode/AGENTS.md` | link em `~/.codex/AGENTS.md` |
| Skills que o modelo inicia | lê a pasta do Claude Code | link em `~/.agents/skills/` |
| Agentes | `agents/`, e o que `tools` deixa de fora vira `deny` | não |
| MCP do catálogo (`mcps` no perfil) | `mcp.servers` em `opencode.json` | `codex mcp add` |

Os dois só leem `AGENTS.md`: sem o link, a sessão roda sem as instruções. `soul mcp <catálogo>` escreve
os servidores em cada CLI encontrada, e um servidor com `enabled: false` sai delas. Os plugins de tela
(fancy, mermaid, tui-embed) desenham dentro do Claude Code e ficam nele.

Uma soul aberta com `opencode invoke` passa pelo proxy: os modelos Claude do OpenCode rodam o Claude Code
debaixo do servidor privado da sessão, que herda a rota. Numa sessão sem soul, ligada ao serviço de fundo,
não passa.

Todo esse trabalho mora em `cfgs/shell/soul`; as funções `claude`, `codex` e `opencode` do
`mandrake.sh` têm uma linha cada, porque o Bash do Claude Code guarda as funções do shell, mas
descarta as que começam com `_`.

## A pasta ~/.claude

Um perfil diz, para cada entrada de `~/.claude`, se ela fica, vira link para uma pasta sua ou
recebe uma cópia. Nada é apagado: o que está no caminho vai para
`~/.local/state/mandrake/backups/<data>/`, cada passo vai para um journal, `mandrake link rollback`
desfaz. Quando o destino de um link ainda não existe (ou é uma pasta vazia), o conteúdo é copiado
para lá antes, e a entrada continua mostrando os mesmos arquivos pelo link; o rollback devolve o
original e guarda o destino preenchido em `backups/<data>/.seeded/`. `mandrake capture` escreve a pasta atual como perfil, para refazer outra máquina.

## Onde fica cada coisa

| | Padrão | Muda com |
| - | - | - |
| perfil | `~/.config/mandrake/mandrake.toml` | `XDG_CONFIG_HOME` |
| cópia instalada (`data/`, `cfgs/`) | `~/.local/share/mandrake/src` | `XDG_DATA_HOME` |
| eventos, bench, journal, backups, logs | `~/.local/state/mandrake` | `XDG_STATE_HOME` |
| tudo acima, para teste | | `MANDRAKE_HOME` |
