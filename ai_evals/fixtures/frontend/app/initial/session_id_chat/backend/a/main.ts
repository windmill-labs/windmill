export async function main({
	session_id,
	message,
	system_prompt
}: {
	session_id: string
	message: string
	system_prompt: string
}) {
	return {
		content: `Session ${session_id}: ${message}`,
		tool_calls: [],
		system_prompt_length: system_prompt.length
	}
}
