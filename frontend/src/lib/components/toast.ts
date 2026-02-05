const pathRegex = /\b(u|f)(\/[^\/\s]+){2,}\b/g

export function processMessage(message: string | undefined): string {
	let msg = !message
		? 'Error without message'
		: typeof message != 'string'
			? JSON.stringify(message, null, 2)
			: message
	return msg.replaceAll(pathRegex, (path) => {
		return `<span class="bg-surface-secondary p-1 text-xs font-mono whitespace-nowrap rounded-md">${path}</span>`
	})
}
