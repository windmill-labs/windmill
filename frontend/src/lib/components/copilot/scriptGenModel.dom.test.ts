import { afterEach, expect, it, vi } from 'vitest'
import { flushSync, mount, tick, unmount } from 'svelte'
import { get } from 'svelte/store'
import {
	copilotSessionModel,
	setCopilotInfo,
	setLastModelPick,
	setScriptGenModelPick
} from '$lib/aiStore'
import ScriptGen from './ScriptGen.svelte'
import { workspaceAIClients } from './lib'

vi.mock('$app/navigation', () => ({
	goto: vi.fn(),
	afterNavigate: vi.fn(),
	beforeNavigate: vi.fn()
}))
// jsdom has no Element.animate.
vi.mock('svelte/transition', async (importOriginal) => ({
	...(await importOriginal<typeof import('svelte/transition')>()),
	fade: () => ({ duration: 0 })
}))
vi.mock('monaco-editor', () => ({ Selection: class Selection {}, editor: {} }))

let component: ReturnType<typeof mount> | undefined
afterEach(() => component && unmount(component))

it('sends its own model and effort, whatever the last pick is', async () => {
	setCopilotInfo({
		providers: {
			anthropic: {
				resource_path: 'u/admin/a',
				models: ['claude-sonnet-4-6', 'claude-opus-4-6', 'claude-opus-4-8']
			}
		},
		default_model: { provider: 'anthropic', model: 'claude-sonnet-4-6' }
	})
	const stream = vi.fn((_body: any) =>
		(async function* () {
			yield {
				type: 'content_block_delta',
				delta: { type: 'text_delta', text: '```bash\necho\n```' }
			}
		})()
	)
	vi.spyOn(workspaceAIClients, 'getAnthropicClient').mockReturnValue({
		messages: { stream }
	} as any)

	component = mount(ScriptGen, {
		target: document.body,
		props: { lang: 'bash', editor: undefined, diffEditor: undefined, args: {} }
	})
	const lastPick = { provider: 'anthropic', model: 'claude-opus-4-8' } as const
	setLastModelPick(lastPick)
	setScriptGenModelPick({ provider: 'anthropic', model: 'claude-opus-4-6', reasoning: 'low' })
	flushSync()

	document.querySelector<HTMLButtonElement>('button[title="Generate code from prompt"]')!.click()
	await vi.waitFor(() => expect(document.querySelector('textarea')).toBeTruthy())
	const input = document.querySelector('textarea')!
	input.value = 'say hi'
	input.dispatchEvent(new Event('input', { bubbles: true }))
	await tick()
	input.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }))
	await vi.waitFor(() => expect(stream).toHaveBeenCalled())
	const body = stream.mock.calls[0][0]
	expect(body.model).toBe('claude-opus-4-6')
	expect(body.output_config?.effort).toBe('low')
	expect(get(copilotSessionModel)).toEqual(lastPick)
})
