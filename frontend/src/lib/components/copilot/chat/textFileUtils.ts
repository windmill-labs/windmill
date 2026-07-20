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

/** Read a file for message attachment. Returns null when the sniff says binary. */
export async function fileToAttachedTextFile(file: File): Promise<AttachedTextFile | null> {
	if (!(await isTextFile(file))) return null
	return { name: file.name, content: await file.text() }
}
