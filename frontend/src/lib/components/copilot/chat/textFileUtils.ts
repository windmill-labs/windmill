/**
 * Message-scoped text-file attachments for the GLOBAL chat composer.
 *
 * Text files attach like images: chips in the composer, riding the next
 * message, cleared on send. Unlike images, only a *reference* (name + size)
 * enters the prompt — the content is registered into the session file store at
 * send and the model reads it on demand via the file tools, same as a DOM pick
 * is inspected via the DOM tools. The full content stays on the message for
 * the bubble preview and for re-registration on edit/retry.
 */
import { isTextFile } from './files/fileEngine'

export type AttachedTextFile = {
	name: string
	/** Stable reference the transcript, prompt, and file tools join on — a content
	 * hash of (name, content), see attachedTextFileId. The name is display-only
	 * and may collide freely. Absent only on transcripts persisted before ids
	 * existed; hydrated (deterministically, from the same hash) on chat load. */
	id?: string
	content: string
}

/**
 * Files one message may carry. Enforced wherever a message is assembled, not
 * just at the composer: queuing clears the composer, so its own count would
 * reset and let repeated sends stack an unbounded batch into a single message.
 */
export const MAX_ATTACHED_FILES = 8

/**
 * Per-file byte cap. The model reads content on demand (never inlined), so
 * this only bounds what rides the message state and the chat history's
 * persisted snapshot — a sanity ceiling, not a context-window one. Larger
 * files can be linked via their folder instead.
 */
export const MAX_TEXT_FILE_BYTES = 1_000_000

/**
 * Cumulative cap across a conversation. Message-file content lives inside the
 * transcript (DisplayMessage.files) and is rewritten with every history save,
 * so without a conversation-level bound repeated attachments would grow the
 * in-memory record and its IndexedDB copy without limit. Enforced at attach
 * time against transcript + queue + composer bytes.
 */
export const MAX_CONVERSATION_FILE_BYTES = 5_000_000

/** Read a file for message attachment. Returns null when the sniff says binary.
 * The id is minted by the composer after name finalization (a same-name clash in
 * one draft gets a courtesy rename first, and the id hashes the final name). */
export async function fileToAttachedTextFile(file: File): Promise<AttachedTextFile | null> {
	if (!(await isTextFile(file))) return null
	return { name: file.name, content: await file.text() }
}

// cyrb53 (public-domain hash by bryc) — chosen over crypto.subtle because it is
// synchronous and works on plain-HTTP deployments where SubtleCrypto is absent.
function cyrb53(str: string, seed: number): number {
	let h1 = 0xdeadbeef ^ seed
	let h2 = 0x41c6ce57 ^ seed
	for (let i = 0; i < str.length; i++) {
		const ch = str.charCodeAt(i)
		h1 = Math.imul(h1 ^ ch, 2654435761)
		h2 = Math.imul(h2 ^ ch, 1597334677)
	}
	h1 = Math.imul(h1 ^ (h1 >>> 16), 2246822507)
	h1 ^= Math.imul(h2 ^ (h2 >>> 13), 3266489909)
	h2 = Math.imul(h2 ^ (h2 >>> 16), 2246822507)
	h2 ^= Math.imul(h1 ^ (h1 >>> 13), 3266489909)
	return 4294967296 * (2097151 & h2) + (h1 >>> 0)
}

/**
 * Deterministic content-hash id for a message attachment. Identity derived from
 * the file itself: re-registration after reload/rollback lands on the same id by
 * construction, identical attaches dedupe, and legacy transcripts hydrate their
 * ids without migration state. Two seeded cyrb53 passes (~106 bits) — collision
 * odds are negligible at conversation scale, and the context is not adversarial
 * (a user's own attachments).
 */
export function attachedTextFileId(name: string, content: string): string {
	// NUL separator: unambiguous split (filenames cannot contain it), so
	// two (name, content) pairs never hash alike across the boundary.
	const input = `${name}\u0000${content}`
	return `f${cyrb53(input, 1).toString(36)}${cyrb53(input, 2).toString(36)}`
}

/** Return `files` with every entry carrying its id (legacy rows hydrated). */
export function withAttachedTextFileIds(files: AttachedTextFile[]): AttachedTextFile[] {
	return files.map((f) => (f.id ? f : { ...f, id: attachedTextFileId(f.name, f.content) }))
}

/** Courtesy rename for a same-name clash within one message draft: `notes.md` →
 * `notes (2).md`. Display-only — identity is the id, and names may collide
 * across messages — but two identical labels inside one draft would be
 * indistinguishable to the user and the model alike. */
export function uniqueDraftFileName(original: string, taken: Iterable<string>): string {
	const names = new Set(taken)
	if (!names.has(original)) return original
	const dot = original.lastIndexOf('.')
	const base = dot > 0 ? original.slice(0, dot) : original
	const ext = dot > 0 ? original.slice(dot) : ''
	let n = 2
	while (names.has(`${base} (${n})${ext}`)) n++
	return `${base} (${n})${ext}`
}

/** UTF-8 byte length of attachment content — budget math must match the byte
 * caps, and string length undercounts multibyte text. */
export function textByteLength(content: string): number {
	return new TextEncoder().encode(content).length
}
