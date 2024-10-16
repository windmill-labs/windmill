//regex that match path starting with u/ or f/ and with at least 2 /
const pathRegex = /\b(u|f)\/[^\/\s]+\/[^\/\s]+\b/g

export function processMessage(message: string): string {
	return message.replaceAll(pathRegex, (path) => {
		return `<span class="bg-surface-secondary p-1 text-xs font-mono whitespace-nowrap rounded-md">${path}</span>`
	})
}
