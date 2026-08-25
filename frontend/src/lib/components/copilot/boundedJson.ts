/**
 * Read a JSON body, refusing one larger than `maxBytes`.
 *
 * A provider resource can point at any endpoint, including a gateway the workspace does not
 * control, and callers that list its models without a user asking (the AI agent step catalog)
 * would otherwise parse and sort whatever it sends. An abort signal bounds how long that takes,
 * not how much arrives.
 *
 * A leaf module of its own so it is testable: `copilot/lib` reaches Monaco through its imports.
 */
export async function readJsonWithLimit(response: Response, maxBytes: number): Promise<unknown> {
	const reader = response.body?.getReader()
	if (!reader) {
		return response.json()
	}
	const chunks: Uint8Array[] = []
	let received = 0
	while (true) {
		const { done, value } = await reader.read()
		if (done) break
		received += value.byteLength
		if (received > maxBytes) {
			await reader.cancel()
			throw new Error(`Model listing exceeded ${maxBytes} bytes`)
		}
		chunks.push(value)
	}
	const body = new Uint8Array(received)
	let offset = 0
	for (const chunk of chunks) {
		body.set(chunk, offset)
		offset += chunk.byteLength
	}
	return JSON.parse(new TextDecoder().decode(body))
}
