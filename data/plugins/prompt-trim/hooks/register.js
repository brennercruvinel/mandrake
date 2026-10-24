// Cuts what Claude Code adds around the system prompt, and puts Mandrake's core prompt in its place.
// Each name below is one piece; delete a name to let that piece through again.

// Messages Claude Code attaches to the conversation on its own.
// Kept, with their fixed English wording rewritten below: instructions, output_style_instructions,
// queued_command, file, at_mention_reference, edited_text_file, deferred_tools_delta,
// skill_listing, hook_additional_context, hook_blocking_error, poll_events, task_status.
const ATTACHMENTS = new Set([
  'account_memory_recall',
  'advisor_tool',
  'agent_listing_delta',
  'agent_mention',
  'artifact_opening_prefetch',
  'async_hook_response',
  'auto_mode',
  'auto_mode_exit',
  'bash_output_audience_note',
  'batching_reminder',
  'budget_usd',
  'compact_file_reference',
  'context_sections',
  'coordinator_context',
  'cowork_memory_context',
  'critical_system_reminder',
  'date',
  'date_change',
  'diagnostics',
  'dir_sync_notice',
  'directory',
  'dynamic_skill',
  'elapsed_time_reminder',
  'environment',
  'fork_briefing',
  'hook_stopped_continuation',
  'hook_success',
  'inlined_image_paths',
  'invoked_skills',
  'language',
  'mcp_dropped_tools_delta',
  'mcp_instructions_delta',
  'mcp_resource',
  'memory_update',
  'model',
  'nested_memory',
  'opened_file_in_ide',
  'output_limit_note',
  'output_style',
  'output_token_usage',
  'pdf_reference',
  'peer_mention',
  'plan_file_reference',
  'plan_mode',
  'plan_mode_exit',
  'plan_mode_reentry',
  'prefix_delta',
  'proactivity',
  'read_truncation_notice',
  'relevant_memories',
  'remote_session_change',
  'sandbox_instructions',
  'secondary_reminder',
  'selected_lines_in_diff',
  'selected_lines_in_ide',
  'session_context',
  'silent_turn_reminder',
  'skill_mention',
  'task_reminder',
  'team_context',
  'teammate_mailbox',
  'thread_state',
  'todo_reminder',
  'token_usage',
  'tool_hosts_correction',
  'tool_hosts_notice',
  'tool_search_usage_reminder',
  'total_tokens_reminder',
  'ultra_effort_enter',
  'ultra_effort_exit',
  'ultrathink_effort',
  'unknown_command_fallback',
  'workflow_keyword_request',
  'workflow_size_guideline_change',
])

// System prompt sections, matched by the name before any ':' suffix.
// Only present in sessions without a monster profile (--system-prompt-file replaces them).
const SECTIONS = new Set([
  'act_dont_rederive',
  'action_caution',
  'autonomy_append',
  'bg-session',
  'brief',
  'brook_heron',
  'communication',
  'context_management',
  'delivering_work_max',
  'endconv_deferred_hint',
  'env_info_model',
  'env_info_simple',
  'env_info_static',
  'fable_identity',
  'focus_mode',
  'heron_brook',
  'memory',
  'opus5_reduced_delegation',
  'overcorrection',
  'pronouns',
  'session_guidance',
  'subagent_steer_delegation',
  'surface',
  'system_messages',
  'task_continuity',
  'tool_param_json',
  'willow_tern',
])

// Composed sections that never pass through prompt.section.
const COMPOSED = new Set([
  'actions', 'doing_tasks', 'intro', 'lean_body', 'memory_static',
  'repl_working', 'system', 'tone', 'tools', 'total_tokens', 'user_pays',
])

// Fixed English wording Claude Code wraps around the kept attachments.
const REWRITE = {
  instructions: [
    ['Codebase and user instructions are shown below. Be sure to adhere to these instructions. IMPORTANT: These instructions OVERRIDE any default behavior and you MUST follow them exactly as written.', ''],
    [/^Contents of (.+?) \([^)\n]*\):$/gm, '# $1'],
  ],
  output_style_instructions: [[/^# Output Style: [^\n]*\n/, '']],
  hook_additional_context: [[/^\S+ hook additional context: /, '']],
  hook_blocking_error: [[/^(\S+) hook blocking error from command: "(.*?)": /, 'hook $1 bloqueou "$2": ']],
  edited_text_file: [
    [/^Note: (.+?) changed on disk since you last read it\. .*? Here are the relevant changes \(shown with line numbers\):\n/s, '$1 mudou no disco desde a última leitura:\n'],
    [/^Note: (.+?) changed on disk since you last read it\. .*? The changes are not shown here; use \S+ if you need the current content\./s, '$1 mudou no disco desde a última leitura.'],
  ],
  at_mention_reference: [
    ['The user @-mentioned ', 'citado: '],
    [/:? ?if these are files or directories in your working directory, read them with your file tools before responding\./, ''],
  ],
  file: [
    [/^Called the \S+ tool with the following input: /m, ''],
    [/^Result of calling the \S+ tool:\n/m, ''],
  ],
  deferred_tools_delta: [
    [/The following deferred tools are now available via (\S+)\. .*?before calling them:/s, 'Carregar com $1 (select:<nome>):'],
    ['The following tools just became available and are ready to use:', 'Disponíveis:'],
    ['The following tools have updated definitions, which replace the earlier ones from here on:', 'Definições atualizadas:'],
    ['The following tools are no longer available. Do not call them:', 'Indisponíveis:'],
    ['The following tools are now available:', 'Disponíveis:'],
  ],
  skill_listing: [['The following skills are available for use with the Skill tool:\n', 'Skills:\n']],
  task_status: [[/^Task "(.*?)" \((.*?)\) was stopped by the user\./, 'Tarefa "$1" ($2) parada.']],
}

const rewrite = (type, text) =>
  (REWRITE[type] ?? []).reduce((t, [from, to]) => t.replace(from, to), String(text)).replace(/^\s+/, '')

// Subagent types offered to Claude: none.
const OFFER_AGENTS = false

// Tool descriptions: removed.
const TOOL_DESCRIPTIONS = false

// The core prompt, written by `mandrake setup` into ./core.js from the profile (souls.core).
// Without it the default prompt is cut and nothing replaces it, so a plain `claude` would run with an
// empty system prompt. A soul session already carries the core in its --system-prompt-file.
import { CORE } from './core.js'

export function register(on) {
  on('prompt.attachment', async ($, e, next) => {
    if (ATTACHMENTS.has(e.type)) return { text: null }
    const r = await next(e)
    return r?.text == null || !REWRITE[e.type] ? r : { ...r, text: rewrite(e.type, r.text) }
  })
  on('prompt.section', async ($, e, next) =>
    SECTIONS.has(String(e.name).split(':')[0]) ? { text: null } : next(e))
  on('prompt.compose', async ($, e, next) => {
    const r = await next(e)
    const kept = r.sections.filter(s => !COMPOSED.has(s.id))
    if (!CORE || globalThis.process?.env?.MANDRAKE_SOUL) return { ...r, sections: kept }
    return { ...r, sections: [...kept, { id: 'prompt-trim:core', text: CORE, scope: 'session' }] }
  })
  on('agent.offer', async ($, e, next) =>
    OFFER_AGENTS ? next(e) : { isOffered: false })
  on('tool.describe', async ($, e, next) =>
    TOOL_DESCRIPTIONS ? next(e) : next({ ...e, description: '' }))
}
