import { describe, expect, it, vi } from 'vitest'
import { toResponsesContent } from './openai-responses'

// openai-responses.ts pulls in the chat client/registry layer at import time; the
// helper under test is pure, so stub those side-effecting modules away.
vi.mock('../lib', () => ({
	createOpenAIProxyClient: vi.fn(),
	getAiProxyBaseURL: vi.fn(),
	getProviderAndCompletionConfig: vi.fn(),
	providerSupportsWebSearch: vi.fn(),
	workspaceAIClients: {}
}))

vi.mock('../reasoningRegistry', () => ({
	applyReasoningToConfig: vi.fn()
}))

vi.mock('./shared', () => ({
	processToolCall: vi.fn(),
	appendPendingToolImages: vi.fn()
}))

describe('toResponsesContent', () => {
	it('passes a plain string through unchanged', () => {
		expect(toResponsesContent('hello')).toBe('hello')
	})

	it('maps text parts to input_text and image_url parts to input_image (string url)', () => {
		const out = toResponsesContent([
			{ type: 'text', text: 'describe this' },
			{ type: 'image_url', image_url: { url: 'data:image/png;base64,ZZZZ' } }
		]) as any[]

		expect(out).toEqual([
			{ type: 'input_text', text: 'describe this' },
			{ type: 'input_image', image_url: 'data:image/png;base64,ZZZZ' }
		])
	})

	it('leaves unrecognized parts untouched', () => {
		const part = { type: 'input_audio', input_audio: {} }
		expect((toResponsesContent([part]) as any[])[0]).toBe(part)
	})
})
