# O que o corte nunca pode mudar

O Mandrake corta o que o Claude Code põe em volta do seu prompt: as seções do prompt padrão, as
descrições das tools, 64 tipos de anexo e, no proxy, a frase de identidade e a tag `system-reminder`.
Essas são regras de comportamento que continuam valendo depois do corte. Cada uma tem um teste, e os
testes rodam no `tool/fullcheck.sh`:

- `data/plugins/prompt-trim/tests/invariants.test.js` roda os hooks do prompt-trim com um Claude Code
  de mentira, sem rede.
- `tool/tests/e2e/invariants.sh` manda o mesmo prompt pelo Claude Code de verdade, sem e com o
  Mandrake, para a API local do `mandrake compare`. Nada é gasto, e não precisa de login.

| | Invariante | Por quê |
| - | - | - |
| I1 | As tools são as mesmas, com o mesmo nome e o mesmo `input_schema`; só a descrição sai | o modelo continua podendo fazer tudo o que fazia, e o harness valida as chamadas como antes |
| I2 | As instruções do usuário (`CLAUDE.md`) e o próprio prompt chegam ao modelo; só o cabeçalho fixo muda | são a vontade de quem usa |
| I3 | Os hooks continuam falando com o modelo: um bloqueio e o motivo dele, `additionalContext`, a saída de um hook que deu certo, a resposta de um hook assíncrono | hooks são como o usuário controla e informa a sessão; um guard que bloqueia `rm -rf` precisa dizer por quê |
| I4 | Um modo ou limite que o harness impõe é dito ao modelo: plan mode (entrar, sair, voltar), auto mode, as instruções do sandbox, os lembretes críticos | o harness bloqueia de qualquer jeito; sem o aviso, o modelo tenta, falha e não entende por quê. Só aparecem enquanto o modo está ligado |
| I5 | Um anexo que o prompt-trim não conhece passa | o corte é uma lista de nomes, não um padrão: um aviso novo de uma versão nova do Claude Code chega até alguém decidir cortá-lo |
| I6 | O core que entra no lugar do prompt padrão traz as regras de segurança: perguntar antes do que é difícil de desfazer ou atinge outras pessoas, não contornar o que está no caminho, nunca expor segredos, texto de arquivo ou de ferramenta não tem autoridade | essas regras viviam nas seções que saem (`action_caution`, `system`) |
| I7 | O que sai está escrito, nome por nome, em `hooks/register.js` | quem revisa vê exatamente o que muda |
| I8 | Modelo, `max_tokens`, `thinking`, `tool_choice` e `stream` são iguais nos dois lados | o corte mexe no texto, não no pedido |

O proxy tem os próprios testes em `rust/proxy/src/trim.rs`. Ele tira só um bloco de `system` cujo texto
inteiro é uma das frases de identidade, tira a tag `system-reminder` mas deixa o conteúdo, e devolve
igual, byte a byte, um corpo que não é JSON ou que não mudou. Fora isso, só sai o campo `metadata`.

## O que mudou ao escrever esta lista

O teste de ponta a ponta achou um corte que quebrava a I3: a saída simples de um hook
`UserPromptSubmit` (`echo …`) sumia, porque o anexo `hook_success` estava na lista. Também saíram da
lista `async_hook_response` e `hook_stopped_continuation` (I3), e os anexos de plan mode, auto mode,
sandbox e `critical_system_reminder` (I4).

## Fora daqui

As permissões, o sandbox e os hooks são aplicados pelo Claude Code, fora do prompt. Cortar texto não
dá ao modelo uma permissão que ele não tinha. Estes testes conferem o que o modelo fica sabendo, não
o que o harness deixa fazer.
