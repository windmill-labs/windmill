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
	// Toast renders via {@html}, so escape unconditionally — server error
	// bodies can contain arbitrary content (e.g. `<` from a SQL fragment in
	// an error message), and the path regex below only ever inserts a safe
	// `<span>` around a `u/...` or `f/...` capture that itself cannot match
	// any HTML metacharacter. Convert `\n` to `<br />` so multi-line errors
	// stay readable in the toast (without this, HTML collapses newlines).
	let html = escapeHtml(msg).replaceAll('\n', '<br />')
	return html.replaceAll(pathRegex, (path) => {
		return `<span class="bg-surface-secondary p-1 text-xs font-mono whitespace-nowrap rounded-md">${path}</span>`
	})
}
