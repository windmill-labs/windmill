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

// Same rule as `dedicated_worker_pattern_prefix()` in backend/windmill-common/src/worker.rs:
// `f/dw/*` stands for every dedicated script under `f/dw/`. Flows are listed one by one.
export function isDedicatedPattern(path: string): boolean {
	return path.endsWith('/*') && path.length > 2 && !path.startsWith('flow/') && !path.includes(':')
}

// One leading `/` is dropped; `//` and a lone `/` are kept as written, as
// `WorkspacedPath::dedicated` in backend/windmill-common/src/worker.rs reads an entry.
function canonicalPath(path: string): string {
	return path.length > 1 && path[0] === '/' && path[1] !== '/' ? path.substring(1) : path
}

export function canonicalTag(tag: string): string {
	const parsed = parseTag(tag)
	if (!parsed) return tag
	return `${parsed.workspace}:${parsed.type === 'flow' ? 'flow/' : ''}${parsed.path}`
}

export function parseTag(
	tag: string
): { workspace: string; type: 'script' | 'flow' | 'pattern'; path: string } | null {
	const colonIndex = tag.indexOf(':')
	if (colonIndex === -1) return null

	const workspace = tag.substring(0, colonIndex)
	const raw = tag.substring(colonIndex + 1)
	const rest = canonicalPath(raw)

	if (rest.startsWith('flow/')) {
		// Only an entry written `flow/…` has its flow path read; `/flow/…` is a path read whole
		const path = rest.substring(5)
		return { workspace, type: 'flow', path: raw.startsWith('flow/') ? canonicalPath(path) : path }
	} else if (isDedicatedPattern(rest)) {
		return { workspace, type: 'pattern', path: rest }
	} else {
		return { workspace, type: 'script', path: rest }
	}
}

export async function computeHashedTag(rawTag: string): Promise<string> {
	const parsed = parseTag(rawTag)
	if (!parsed || parsed.type === 'pattern') return canonicalTag(rawTag)
	const fullPath = parsed.type === 'flow' ? `flow/${parsed.path}` : parsed.path
	return dedicatedWorkerTag(parsed.workspace, fullPath)
}
