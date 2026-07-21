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
	/** Original filename before a courtesy rename (set only when one happened).
	 * Lets duplicate detection recognize a re-drop of the same source file without
	 * inferring provenance from the display name — a user's real `report (2).md`
	 * must never be mistaken for a rename of `report.md`. */
	sourceName?: string
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

/** Read a file for message attachment. Returns null when the sniff says binary
 * or the content exceeds MAX_TEXT_FILE_BYTES — the cap is enforced here at the
 * reader, not only at callers' pre-checks, so no ingestion path can persist an
 * oversized attachment (decoding can also grow past the raw size when malformed
 * UTF-8 expands to replacement characters).
 * The id is minted by the composer after name finalization (a same-name clash in
 * one draft gets a courtesy rename first, and the id hashes the final name). */
export async function fileToAttachedTextFile(file: File): Promise<AttachedTextFile | null> {
	if (file.size > MAX_TEXT_FILE_BYTES) return null
	if (!(await isTextFile(file))) return null
	const content = await file.text()
	if (textByteLength(content) > MAX_TEXT_FILE_BYTES) return null
	return { name: file.name, content }
}

/** Line count as the file tools report it (fileEngine.buildLineIndex): an empty
 * file has 0 lines and a trailing newline is not an extra line. The prompt must
 * advertise the same number or the model requests invalid read_file ranges. */
export function textLineCount(content: string): number {
	if (content === '') return 0
	return content.split('\n').length - (content.endsWith('\n') ? 1 : 0)
}

/** Admit files in order while their DECODED byte size fits `budget`. Admission
 * pre-checks use raw File.size, but the committed charge is the decoded UTF-8
 * length, which malformed input inflates (an invalid byte decodes to a 3-byte
 * replacement character) — so the commit step must re-check against what will
 * actually be charged. */
export function admitWithinByteBudget(
	files: AttachedTextFile[],
	budget: number
): { admitted: AttachedTextFile[]; dropped: number } {
	const admitted: AttachedTextFile[] = []
	let dropped = 0
	for (const f of files) {
		const bytes = textByteLength(f.content)
		if (bytes <= budget) {
			admitted.push(f)
			budget -= bytes
		} else {
			dropped++
		}
	}
	return { admitted, dropped }
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

/**
 * Fold freshly-read files into a draft's attachment list: identical
 * (name, content) duplicates are dropped, same-name-different-content clashes
 * get the courtesy rename, and ids are minted from the final name. Must run
 * against the LIVE list in the synchronous commit step — attach batches overlap
 * (each awaits its file reads), so dedupe/rename decisions made mid-read would
 * be stale by commit time.
 */
export function foldIntoDraft(
	current: AttachedTextFile[],
	reads: { name: string; content: string }[]
): AttachedTextFile[] {
	const commit: AttachedTextFile[] = []
	for (const f of reads) {
		const draft = [...current, ...commit]
		// "Same file dropped twice" means same original (name, content) — a
		// committed copy may have been courtesy-renamed, so its original name is
		// carried in sourceName rather than inferred from the display name (a
		// user's real `report (2).md` is not a rename of `report.md`).
		if (
			draft.some((x) => x.content === f.content && (x.name === f.name || x.sourceName === f.name))
		) {
			continue
		}
		const name = uniqueDraftFileName(
			f.name,
			draft.map((x) => x.name)
		)
		commit.push({
			name,
			content: f.content,
			id: attachedTextFileId(name, f.content),
			...(name !== f.name ? { sourceName: f.name } : {})
		})
	}
	return commit
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
