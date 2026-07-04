import { describe, expect, it, vi } from 'vitest'
import type { ChatCompletionMessageParam } from 'openai/resources/index.mjs'

vi.mock('monaco-editor', () => ({
	editor: {}
}))

vi.mock('$lib/stores', () => ({
	workspaceStore: { subscribe: () => () => undefined }
}))

vi.mock('$lib/components/flows/flowTree', () => ({
	findModuleInModules: () => undefined
}))

vi.mock('$lib/gen', () => ({
	OpenAPI: {},
	ResourceService: {},
	ScriptService: {},
	FlowService: {},
	JobService: {},
	ScheduleService: {},
	HttpTriggerService: {},
	WebsocketTriggerService: {},
	KafkaTriggerService: {},
	NatsTriggerService: {},
	PostgresTriggerService: {},
	MqttTriggerService: {},
	SqsTriggerService: {},
	GcpTriggerService: {},
	AzureTriggerService: {}
}))

vi.mock('$lib/utils', () => ({
	emptyString: (value: string | undefined | null) => !value,
	generateRandomString: () => 'generated_id'
}))

vi.mock('$lib/scripts', () => ({
	scriptLangToEditorLang: (language: string) => language
}))

vi.mock('$lib/aiStore', () => ({
	getCurrentModel: () => undefined,
	getMetadataModel: () => undefined,
	copilotInfo: {
		subscribe: (run: (value: unknown) => void) => {
			run({})
			return () => undefined
		}
	}
}))

vi.mock('@leeoniya/ufuzzy', () => ({
	default: class {
		search() {
			return [[], [], []]
		}
	}
}))

function streamOf(chunks: unknown[]): any {
	return (async function* () {
		for (const chunk of chunks) {
			yield chunk
		}
	})()
}

function toolCallChunk(delta: Record<string, unknown>) {
	return { choices: [{ delta: { tool_calls: [{ index: 0, ...delta }] } }] }
}

function createCallbacks() {
	return {
		onNewToken: vi.fn(),
		onMessageEnd: vi.fn(),
		setToolStatus: vi.fn(),
		removeToolStatus: vi.fn()
	}
}

function createTool(fn = vi.fn().mockResolvedValue('tool ok')) {
	return {
		def: {
			type: 'function' as const,
			function: { name: 'patch_app_file', parameters: { type: 'object' } }
		},
		fn
	}
}

describe('parseOpenAICompletion tool call arguments', () => {
	it('does not execute nor persist a tool call whose streamed arguments are truncated', async () => {
		const { parseOpenAICompletion } = await import('./lib')
		const fn = vi.fn()
		const callbacks = createCallbacks()
		const messages: ChatCompletionMessageParam[] = []
		const addedMessages: ChatCompletionMessageParam[] = []

		const result = await parseOpenAICompletion(
			streamOf([
				toolCallChunk({
					id: 'call_1',
					function: { name: 'patch_app_file', arguments: '{"path": "u/admin/app", ' }
				}),
				// The stream ends mid-arguments (e.g. output token limit or dropped connection)
				toolCallChunk({ function: { arguments: '"old_string": "setMessages(prev' } })
			]),
			callbacks,
			messages,
			addedMessages,
			[createTool(fn)] as any,
			{},
			undefined,
			{ workspace: 'test' }
		)

		expect(fn).not.toHaveBeenCalled()
		expect(result.shouldContinue).toBe(true)

		const assistant = messages.find((m) => m.role === 'assistant') as any
		// Persisting the truncated arguments string would make every follow-up
		// request fail provider-side JSON parsing, bricking the session.
		expect(assistant.tool_calls[0].function.arguments).toBe('{}')

		const toolResult = messages.find((m) => m.role === 'tool') as any
		expect(toolResult.tool_call_id).toBe('call_1')
		expect(toolResult.content).toContain('NOT executed')
		expect(callbacks.setToolStatus).toHaveBeenCalledWith(
			'call_1',
			expect.objectContaining({ error: expect.stringContaining('invalid or truncated') })
		)
		expect(addedMessages).toEqual(messages)
	})

	it('executes a tool call with valid streamed arguments and keeps them verbatim', async () => {
		const { parseOpenAICompletion } = await import('./lib')
		const fn = vi.fn().mockResolvedValue('tool ok')
		const messages: ChatCompletionMessageParam[] = []

		const result = await parseOpenAICompletion(
			streamOf([
				toolCallChunk({
					id: 'call_1',
					function: { name: 'patch_app_file', arguments: '{"path": ' }
				}),
				toolCallChunk({ function: { arguments: '"u/admin/app"}' } })
			]),
			createCallbacks(),
			messages,
			[],
			[createTool(fn)] as any,
			{},
			undefined,
			{ workspace: 'test' }
		)

		expect(fn).toHaveBeenCalledWith(expect.objectContaining({ args: { path: 'u/admin/app' } }))
		expect(result.shouldContinue).toBe(true)

		const assistant = messages.find((m) => m.role === 'assistant') as any
		expect(assistant.tool_calls[0].function.arguments).toBe('{"path": "u/admin/app"}')
		const toolResult = messages.find((m) => m.role === 'tool') as any
		expect(toolResult.content).toBe('tool ok')
	})
})
