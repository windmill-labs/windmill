export interface PreviewFailureDetailsInput {
	workspaceId: string
	jobId: string
	logs?: string | null
	result?: unknown
	maxCharacters?: number
}

export function formatPreviewFailureDetails(input: PreviewFailureDetailsInput): string {
	const parts = [`workspace=${input.workspaceId}`, `job=${input.jobId}`]
	const maxCharacters = input.maxCharacters ?? 1200
	const logExcerpt = normalizeExcerpt(input.logs, maxCharacters)
	const resultExcerpt = normalizeExcerpt(formatResult(input.result), maxCharacters)

	if (logExcerpt) {
		parts.push(`logs=${logExcerpt}`)
	}
	if (resultExcerpt) {
		parts.push(`result=${resultExcerpt}`)
	}

	return parts.join('; ')
}

function formatResult(result: unknown): string | null {
	if (result == null) {
		return null
	}
	if (typeof result === 'string') {
		return result
	}
	try {
		return JSON.stringify(result)
	} catch {
		return String(result)
	}
}

function normalizeExcerpt(value: string | null | undefined, maxCharacters: number): string | null {
	const normalized = value?.replace(/\s+/g, ' ').trim()
	if (!normalized) {
		return null
	}
	if (normalized.length <= maxCharacters) {
		return normalized
	}
	return `${normalized.slice(0, maxCharacters - 1)}…`
}
