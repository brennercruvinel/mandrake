# Measure and audit

Every view reads the same data: `~/.local/state/mandrake/events.jsonl`, one line per request, and,
with `[proxy] audit = true`, the bodies in `audit/<id>.before.json` and `.after.json`.

## Live: `mandrake watch`

Split screen. The left side shows requests through a proxy in `pass` mode (no Mandrake), the
right side requests through the trimming proxy. Each side has totals (requests, tokens before and
after, bytes, cache read and create) and the latest requests. The footer shows the savings so far.
`Enter` on an audited request opens its hex.

To fill both sides, run a second proxy without the cut and point one session at it:

```sh
mandrake proxy run --pass --port 8788 &
ANTHROPIC_BASE_URL=http://127.0.0.1:8788 claude -p "same prompt"   # left side
claude -p "same prompt"                                             # right side, through 8787
```

The plugin's cut happens before the request exists, so for the left side to show Claude Code
untouched, start that session without the plugin as well (`CLAUDE_CODE_PLUGIN_DIRS=`).

`mandrake watch --embed` draws without the alternate screen, for a tui-embed pane inside Claude Code.

## Hex: `mandrake hex <id>`

The request before and after the cut, side by side in hex and ASCII. Removed bytes are red,
inserted bytes green; `n` and `p` jump between changed regions, and the slider shows where you are
in the body. `mandrake audit` lists the audited ids.

## Bench: `mandrake bench`

O A/B real. Três prompts curtos, cada um pelo `claude -p` duas vezes, com o seu login:

- **sem Mandrake:** sem prompt-trim, por um proxy em modo `pass` que só mede;
- **com Mandrake:** com o prompt-trim instalado, por um proxy que corta.

Os tokens são o que o `count_tokens` da API conta para cada requisição como ela foi enviada.
Custa 6 requisições curtas. Escreve `~/.local/state/mandrake/bench/latest.json` e `.md`, e guarda
as 6 requisições em `bench/run-<data>/` para o `mandrake hex`. `mandrake bench --view` desenha o
último relatório.

`mandrake bench --fixtures <pasta>` mede só o corte do proxy sobre corpos de requisição salvos,
sem rodar o Claude Code; os de `tool/tests/fixtures` são sintéticos, feitos para os testes.

## Tarefas reais: `tool/bench/tasks.py`

O `bench` mede o que cada requisição custa. As tarefas medem se o corte economiza **sem piorar o
trabalho**. São 10 tarefas pequenas em `tool/bench/tasks/`: corrigir um bug, implementar pela
docstring, renomear em vários arquivos, consertar aspas num script, migrar um formato, fazer um
`cargo test` compilar, commitar só o que foi pedido, limpar o build sem apagar `data/`, responder a
partir da documentação e escrever testes que pegam versões quebradas. Cada uma roda pelo `claude -p`
sem e com o Mandrake, e o relatório compara:

| | De onde vem |
| - | - |
| sucesso | o primeiro item do `check.sh` da tarefa, quase sempre testes que o modelo não viu |
| qualidade | os outros itens da rubrica (não mexeu no que não devia, os testes pegam os mutantes…), contados só onde a tarefa deu certo |
| ferramentas, turnos | o stream do `claude -p` |
| tokens, custo | o que o Claude Code informa no fim da sessão |

```sh
tool/bench/tasks.py verify            # não gasta: cada check falha no repo intocado e passa na solução
tool/bench/smoke.sh                   # não gasta: tudo contra uma API local; mostra o relatório e os tokens
tool/bench/tasks.py run --yes -n 3    # gasta 60 sessões reais (10 tarefas x 3 x 2 lados), com o seu login
```

O relatório fica em `~/.local/state/mandrake/bench/tasks-<data>/report.md` e `.json`. As sessões
rodam numa pasta temporária, com só as ferramentas que a tarefa permite (`allow` no `task.toml`) e um
git que não lê a sua configuração nem assina com a sua chave. Uma tarefa nova é uma pasta com `repo/`,
`task.toml`, `check.sh` e uma solução de referência (`solution/`, `solution =` ou `answer =`); o
`verify` roda no `tool/fullcheck.sh`.

## Audit

Audit bodies never contain headers. Text that looks like a secret (`sk-ant-…`, `ghp_…`,
`github_pat_…`, `AKIA…`, private key blocks, Slack tokens) becomes `[redacted]`. Files are mode
600, and those older than `audit_keep_days` are removed when the proxy starts. They still hold your
conversation and code: keep audit on while you are measuring, not by habit.
