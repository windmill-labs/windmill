const MAX_TAG_LEN = 50
const HASH_SUFFIX_LEN = 16

// IMPORTANT: This implementation must stay in sync with the Rust backend
// `dedicated_worker_tag()` in backend/windmill-common/src/worker.rs.
// The frontend version is used for display; the backend version is authoritative.
export async function dedicatedWorkerTag(workspaceId: string, path: string): Promise<string> {
	const fullTag = `${workspaceId}:${path}`
	if (fullTag.length <= MAX_TAG_LEN) return fullTag

	const encoded = new TextEncoder().encode(fullTag)
	const hashBuffer = await crypto.subtle.digest('SHA-256', encoded)
	const hashArray = Array.from(new Uint8Array(hashBuffer))
	const hexHash = hashArray.map((b) => b.toString(16).padStart(2, '0')).join('')

	const prefixLen = MAX_TAG_LEN - 1 - HASH_SUFFIX_LEN
	return `${fullTag.substring(0, prefixLen)}#${hexHash.substring(0, HASH_SUFFIX_LEN)}`
}

export function parseTag(
	tag: string
): { workspace: string; type: 'script' | 'flow'; path: string } | null {
	const colonIndex = tag.indexOf(':')
	if (colonIndex === -1) return null

	const workspace = tag.substring(0, colonIndex)
	const rest = tag.substring(colonIndex + 1)

	if (rest.startsWith('flow/')) {
		return { workspace, type: 'flow', path: rest.substring(5) }
	} else {
		return { workspace, type: 'script', path: rest }
	}
}

export async function computeHashedTag(rawTag: string): Promise<string> {
	const parsed = parseTag(rawTag)
	if (!parsed) return rawTag
	const fullPath = parsed.type === 'flow' ? `flow/${parsed.path}` : parsed.path
	return dedicatedWorkerTag(parsed.workspace, fullPath)
}
