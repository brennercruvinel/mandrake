import { expect, test } from 'claude-code/testing'

const SURFACES = ['terminal', 'desktop'] as const

// Badges over the engine's markdown: fancy with its markdown engine off.
const NATIVE_MARKDOWN = { options: { markdown: false } }

// Stands for the engine beneath: draws a markdown run as a reply does.
const engine = (on: any) =>
  on('ui.render', { component: 'AssistantMessage' }, ($: any, e: any) => {
    const { Markdown } = $.ui.resolve(e)
    return <Markdown text={e.props.text} />
  })

const reply = (text: string) => ({
  plugin: 'fancy',
  component: 'AssistantMessage' as const,
  props: { text, isFirstOfReply: true },
})

for (const surface of SURFACES) {
  test(`${surface}: draws a configured tag as a badge`, NATIVE_MARKDOWN, async ($, on) => {
    engine(on)
    const ui = await $.ui.mount({ ...reply('`[DONE]` shipped\n\n- checked'), surface })
    const badge = await ui.find({ type: 'Text', text: ' DONE ' })

    expect(badge?.props).toMatchObject({ backgroundColor: '#13651A', color: '#FFFFFF', bold: true })
    expect(await ui.find({ type: 'Markdown', text: 'shipped' })).toBeDefined()
  })

  test(`${surface}: leaves a reply without tags to the engine`, NATIVE_MARKDOWN, async ($, on) => {
    on('ui.render', { component: 'AssistantMessage' }, ($, e) => {
      const { Text } = $.ui.resolve(e)
      return <Text>engine</Text>
    })
    const ui = await $.ui.mount({ ...reply('plain reply with [DONE] mid-sentence'), surface })

    expect(await ui.find({ type: 'Text', text: 'engine' })).toBeDefined()
  })

  test(`${surface}: uses the tags and style from the options`, { options: { markdown: false, tags: 'OK:green', style: 'text' } }, async ($, on) => {
    engine(on)
    const ui = await $.ui.mount({ ...reply('[OK] works\n[DONE] not configured'), surface })

    expect((await ui.find({ type: 'Text', text: '[OK]' }))?.props).toMatchObject({ color: 'green' })
    expect(await ui.find({ type: 'Text', text: /DONE/ })).toBeUndefined()
  })
}

for (const surface of SURFACES) {
  test(`${surface}: hands each markdown run to the plugins beneath, first run with the bullet`, NATIVE_MARKDOWN, async ($, on) => {
    const seen: Array<{ text: string; isFirstOfReply: boolean }> = []
    on('ui.render', { component: 'AssistantMessage' }, ($, e) => {
      seen.push({ text: e.props.text, isFirstOfReply: e.props.isFirstOfReply })
      const { Text } = $.ui.resolve(e)
      return <Text>{`beneath:${e.props.text}`}</Text>
    })
    await $.ui.mount({ ...reply('intro\n\n`[DONE]` shipped\n\n```mermaid\nflowchart LR\n  A --> B\n```'), surface })

    expect(seen.map(s => s.isFirstOfReply)).toEqual([true, false])
    expect(seen[0]?.text).toContain('intro')
    expect(seen[1]?.text).toContain('```mermaid')
  })
}

for (const surface of SURFACES) {
  test(`${surface}: paints a gradient tag one cell at a time`, NATIVE_MARKDOWN, async $ => {
    const ui = await $.ui.mount({ ...reply('`[AUTO]` decided'), surface })
    const first = await ui.find({ type: 'Text', text: 'A' })

    expect(first?.props).toMatchObject({ color: '#FFFFFF', bold: true })
    expect((await ui.findAll({ type: 'Text' })).length).toBeGreaterThanOrEqual(6)
  })
}
