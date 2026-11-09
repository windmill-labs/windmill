// The MCP server renders the same templates in Rust (`ai_session_tools/instructions.rs`,
// `render_template`); `mcpSessionGen.test.ts` hashes every profile's render so the two cannot
// drift. Keep the grammar to what both implement: `{{#flag}}…{{/flag}}` keeps its body when
// the flag is set, `{{^flag}}…{{/flag}}` when it is not, `{{name}}` takes a variable, and a
// variable not given stays as written for a later pass to fill.
const TAG = /\{\{([#^/]?)([a-z_]+)\}\}/g

export function renderPromptTemplate(
	template: string,
	flags: ReadonlySet<string>,
	vars: Readonly<Record<string, string>>
): string {
	let out = ''
	let last = 0
	const open: { name: string; keep: boolean }[] = []
	const keeping = () => open.every((s) => s.keep)
	for (const m of template.matchAll(TAG)) {
		const [tag, kind, name] = m
		if (keeping()) out += template.slice(last, m.index)
		last = m.index + tag.length
		if (kind === '#' || kind === '^') {
			open.push({ name, keep: flags.has(name) === (kind === '#') })
		} else if (kind === '/') {
			if (open.pop()?.name !== name) throw new Error(`unbalanced {{/${name}}} in prompt template`)
		} else if (keeping()) {
			out += Object.hasOwn(vars, name) ? vars[name] : tag
		}
	}
	if (open.length) throw new Error(`unclosed {{#${open[0].name}}} in prompt template`)
	return out + template.slice(last)
}
