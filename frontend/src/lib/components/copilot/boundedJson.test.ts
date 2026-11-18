import { describe, expect, it } from 'vitest'
import { readJsonWithLimit } from './boundedJson'

function streamed(body: string): Response {
	const bytes = new TextEncoder().encode(body)
	return new Response(
		new ReadableStream({
			start(controller) {
				// Two chunks, so the cap is checked mid-stream rather than only on the whole body.
				controller.enqueue(bytes.slice(0, Math.ceil(bytes.length / 2)))
				controller.enqueue(bytes.slice(Math.ceil(bytes.length / 2)))
				controller.close()
			}
		}),
		{ status: 200 }
	)
}

const listing = (count: number) =>
	JSON.stringify({ data: Array.from({ length: count }, (_, i) => ({ id: `model-${i}` })) })

describe('readJsonWithLimit', () => {
	it('parses a body within the cap', async () => {
		await expect(readJsonWithLimit(streamed(listing(3)), 1_000_000)).resolves.toEqual({
			data: [{ id: 'model-0' }, { id: 'model-1' }, { id: 'model-2' }]
		})
	})

	it('refuses one over the cap instead of parsing it', async () => {
		// The listing endpoint may be a gateway the workspace does not control, and the catalog
		// calls it without anyone asking.
		await expect(readJsonWithLimit(streamed(listing(5000)), 500)).rejects.toThrow(
			/exceeded 500 bytes/
		)
	})
})
