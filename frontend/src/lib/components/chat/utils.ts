export function parseStreamDeltas(streamData: string): {
	content: string
	type?: string
	success?: boolean
} {
	const lines = streamData.trim().split('\n')
	let content = ''
	let type = 'message'
	let success = true

	for (const line of lines) {
		if (!line.trim()) continue
		try {
			const parsed = JSON.parse(line)
			if (parsed.type === 'tool_result') {
				type = 'tool_result'
				success = parsed.success
				const toolName = parsed.function_name
				content = success ? `Used ${toolName} tool` : `Failed to use ${toolName} tool`
			}
			if (parsed.type === 'token_delta' && parsed.content) {
				content += parsed.content
			}
		} catch (e) {
			console.error('Failed to parse stream line:', line, e)
		}
	}

	return { content, type, success }
}
