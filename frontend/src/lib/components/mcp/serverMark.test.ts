import { beforeEach, describe, expect, it, vi } from 'vitest'

const { getResourceMock } = vi.hoisted(() => ({ getResourceMock: vi.fn() }))

vi.mock('$lib/gen', () => ({ ResourceService: { getResource: getResourceMock } }))
vi.mock('./iconCache', () => ({ cachedProviderMark: () => undefined }))
vi.mock('./providerIcon', () => ({
	loadProviderIcon: async (key: string | null) => (key ? `icon:${key}` : undefined),
	providerKey: (url: unknown) =>
		typeof url === 'string' && url.includes('linear') ? 'linear' : null
}))

import { forgetMcpServerMarks, resolveMcpServerMark } from './serverMark'

describe('resolveMcpServerMark', () => {
	beforeEach(() => {
		forgetMcpServerMarks()
		getResourceMock.mockReset()
	})

	it('reads a server once and shares the answer', async () => {
		getResourceMock.mockResolvedValue({ value: { url: 'https://mcp.linear.app/mcp' } })

		const marks = await Promise.all([
			resolveMcpServerMark('ws', 'u/admin/linear_mcp'),
			resolveMcpServerMark('ws', 'u/admin/linear_mcp')
		])

		expect(marks.map((m) => m.icon)).toEqual(['icon:linear', 'icon:linear'])
		expect(getResourceMock).toHaveBeenCalledTimes(1)
	})

	// A failure must not be memoized: one offline blip would otherwise leave the
	// server unmarked in every later row until the page reloads.
	it('retries after a failed read', async () => {
		getResourceMock.mockRejectedValueOnce(new Error('offline'))
		expect(await resolveMcpServerMark('ws', 'u/admin/linear_mcp')).toEqual({})

		getResourceMock.mockResolvedValue({ value: { url: 'https://mcp.linear.app/mcp' } })
		const mark = await resolveMcpServerMark('ws', 'u/admin/linear_mcp')

		expect(mark.icon).toBe('icon:linear')
		expect(getResourceMock).toHaveBeenCalledTimes(2)
	})
})
