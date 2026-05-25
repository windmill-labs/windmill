const pathRegex = /\b(u|f)(\/[^\/\s]+){2,}\b/g

function escapeHtml(s: string): string {
	return s
		.replaceAll('&', '&amp;')
		.replaceAll('<', '&lt;')
		.replaceAll('>', '&gt;')
		.replaceAll('"', '&quot;')
		.replaceAll("'", '&#39;')
}

export function processMessage(message: string | undefined): string {
	let msg = !message
		? 'Error without message'
		: typeof message != 'string'
			? JSON.stringify(message, null, 2)
			: message
	// Preserve newlines: the toast renders via {@html}, where consecutive
	// whitespace (including \n) collapses to a single space. Escape the raw
	// message first to avoid injecting HTML from server-side error bodies,
	// then convert \n to <br /> so multi-line errors stay readable.
	const hasNewline = msg.includes('\n')
	let html = hasNewline ? escapeHtml(msg).replaceAll('\n', '<br />') : msg
	return html.replaceAll(pathRegex, (path) => {
		return `<span class="bg-surface-secondary p-1 text-xs font-mono whitespace-nowrap rounded-md">${path}</span>`
	})
}
