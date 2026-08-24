import { readFileSync } from 'node:fs'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { describe, expect, it } from 'vitest'

const TAGS = [
	'SchemaFormDnd',
	'SchemaForm',
	'PasswordArgInput',
	'ArgInput',
	'FlowPropertyEditor',
	'PropertyEditor',
	'EditableSchemaForm',
	'EditableSchemaDrawer',
	'ArrayTypeNarrowing',
	'InputTransformSchemaForm',
	'InputTransformForm',
	'ScriptSchema'
]
// `SchemaFormDnd` precedes `SchemaForm` so the longer tag is not matched as the shorter one.
const OPENING = new RegExp(`<(${TAGS.join('|')})(?=[\\s/>]|$)`, 'g')

function formMounts(source: string): { tag: string; line: number; block: string }[] {
	const lines = source.split('\n')
	const mounts: { tag: string; line: number; block: string }[] = []
	for (let i = 0; i < lines.length; i++) {
		const open = lines[i].trim().match(new RegExp(`^${OPENING.source}`))
		if (!open) continue
		// Requiring a lone `>` would run past a mount whose last prop shares the closing line, into
		// the next component, and read its `workspace` as this one's — a false pass on exactly the
		// regression this guards.
		let end = i
		while (end < lines.length && !lines[end].trim().endsWith('>')) end++
		// Running off the end means the props were never delimited, so the block would swallow the
		// rest of the file and match any `workspace` in it — a false pass, not a failure.
		if (end >= lines.length) {
			throw new Error(`unterminated <${open[1]}> mount at line ${i + 1}`)
		}
		mounts.push({ tag: open[1], line: i + 1, block: lines.slice(i, end + 1).join('\n') })
		i = end
	}
	return mounts
}

// `workspace={$workspaceStore}` and `workspace={undefined}` are the nav-workspace fallback this
// guards against, so presence of the prop is not enough.
function forwardsWorkspace(block: string): boolean {
	const m = block.match(/\{workspace\}|workspace=\{([^{}]*)\}/)
	if (!m) return false
	const expr = m[1]?.trim()
	return expr === undefined || (expr !== 'undefined' && expr !== '$workspaceStore')
}

function read(relPath: string): string {
	return readFileSync(resolve(dirname(fileURLToPath(import.meta.url)), relPath), 'utf-8')
}

// Every hop between the form a caller mounts and the PasswordArgInput that mints the secret, plus
// the entry points that supply the workspace in the first place. A hop that drops `workspace` falls
// back to the navigation workspace, so the secret lands where the job will not run — while the
// top-level case keeps passing.
describe.each([
	['ArgInput.svelte', 7],
	['schema/SchemaFormDND.svelte', 1],
	['SchemaForm.svelte', 2],
	['EditableSchemaForm.svelte', 3],
	['schema/FlowPropertyEditor.svelte', 3],
	['schema/EditableSchemaDrawer.svelte', 2],
	['schema/PropertyEditor.svelte', 3],
	['ArrayTypeNarrowing.svelte', 1],
	['InputTransformSchemaForm.svelte', 1],
	['InputTransformForm.svelte', 1],
	['ScriptSchema.svelte', 1],
	['ScriptBuilder.svelte', 1],
	['flows/content/FlowInput.svelte', 2],
	['flows/content/FlowModuleComponent.svelte', 1],
	['flows/content/AgentToolBindings.svelte', 1],
	['ModulePreviewForm.svelte', 1],
	['dbt/DbtEditor.svelte', 1]
	// `flows/content/FlowModuleSuspend.svelte` stays out: its two unthreaded mounts render the
	// locally built `groups` schema and a preview whose args stay empty, so neither can mint.
])('%s nested forms', (relPath, minMounts) => {
	it('forwards workspace to every nested form', () => {
		const source = read(relPath)
		const mounts = formMounts(source)
		expect(mounts.length).toBeGreaterThanOrEqual(minMounts)
		// The scan only recognises a mount opening its own line, so one written inline would be
		// skipped and silently unguarded. Every opening tag in the file has to be accounted for.
		expect(mounts.length).toBe(source.match(OPENING)?.length ?? 0)
		expect(
			mounts.filter((m) => !forwardsWorkspace(m.block)).map((m) => `${m.tag}:${m.line}`)
		).toEqual([])
	})
})
