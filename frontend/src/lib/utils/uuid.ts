export function randomUUID() {
	return 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, function (c) {
		const r = (Math.random() * 16) | 0
		const v = c === 'x' ? r : (r & 0x3) | 0x8
		return v.toString(16)
	})
}

/** Unguessable random token, for values whose unpredictability is load-bearing —
 * anything gating a credential. `randomUUID` above is `Math.random()`-based and
 * must not be used for those. `getRandomValues` (unlike `crypto.randomUUID`) is
 * also available in insecure contexts, so this needs no fallback. */
export function randomSecret(bytes = 32): string {
	const buf = new Uint8Array(bytes)
	crypto.getRandomValues(buf)
	return Array.from(buf, (b) => b.toString(16).padStart(2, '0')).join('')
}
