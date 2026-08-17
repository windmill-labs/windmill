/**
 * A message draft: the four lanes that ship together with one send — text,
 * pastes, images, text files. Every place a draft accumulates or moves
 * (composer attach, queue append, dequeue restore, failure restore) goes
 * through this type, so the draft rules — file dedupe by source identity,
 * courtesy rename, attachment slot caps, all-lanes-move-together — live here
 * once instead of being re-implemented at each aggregation point.
 *
 * Deliberately NOT owned here: the conversation byte budget (needs
 * manager-wide state — enforced at the composer until it moves into the
 * store) and @context/DOM picks (ContextManager owns their lifecycle).
 */
import { MAX_ATTACHED_IMAGES, type AttachedImage } from './imageUtils'
import type { PasteAttachment } from './pasteTokens'
import {
	admitWithinByteBudget,
	foldIntoDraft,
	MAX_ATTACHED_FILES,
	type AttachedTextFile
} from './textFileUtils'

/** A draft's four lanes as plain data — what moves between owners. */
export interface DraftSnapshot {
	text: string
	pastes: PasteAttachment[]
	images: AttachedImage[]
	files: AttachedTextFile[]
}

export class MessageDraft {
	text = $state('')
	pastes = $state<PasteAttachment[]>([])
	images = $state<AttachedImage[]>([])
	files = $state<AttachedTextFile[]>([])

	constructor(seed?: Partial<DraftSnapshot>) {
		if (seed?.text) this.text = seed.text
		if (seed?.pastes) this.pastes = [...seed.pastes]
		if (seed?.images) this.images = [...seed.images]
		if (seed?.files) this.files = [...seed.files]
	}

	get isEmpty(): boolean {
		return (
			this.text.trim() === '' &&
			this.pastes.length === 0 &&
			this.images.length === 0 &&
			this.files.length === 0
		)
	}

	/** Files joining a draft always fold (dedupe by source identity, courtesy
	 * rename) and respect the slot cap. `byteBudget`, when given, admits the
	 * folded entries by their decoded size — the fold must run first because
	 * dedupe changes what gets charged. Returns dropped counts so the caller can
	 * toast — the draft has no UI. */
	addFiles(
		reads: { name: string; content: string; sourceName?: string }[],
		byteBudget?: number
	): { droppedAtCap: number; droppedAtBudget: number } {
		let folded = foldIntoDraft(this.files, reads)
		let droppedAtBudget = 0
		if (byteBudget !== undefined) {
			const res = admitWithinByteBudget(folded, byteBudget)
			folded = res.admitted
			droppedAtBudget = res.dropped
		}
		const merged = [...this.files, ...folded]
		const droppedAtCap = Math.max(0, merged.length - MAX_ATTACHED_FILES)
		this.files = merged.slice(0, MAX_ATTACHED_FILES)
		return { droppedAtCap, droppedAtBudget }
	}

	/** Images join up to the slot cap. Returns the dropped count (caller toasts). */
	addImages(images: AttachedImage[]): number {
		const merged = [...this.images, ...images]
		const dropped = Math.max(0, merged.length - MAX_ATTACHED_IMAGES)
		this.images = merged.slice(0, MAX_ATTACHED_IMAGES)
		return dropped
	}

	/**
	 * Merge a restored draft on top of this one (queued-message delete, restore
	 * after a cancelled/errored turn): the restored draft was written FIRST, so
	 * its text lands above and its attachments ahead of the newer ones — at the
	 * caps it is the newest additions that drop, never the restored draft.
	 * Returns whether text merged onto a non-empty draft (the caller must then
	 * keep both drafts' context), plus dropped counts for toasts.
	 */
	prepend(restored: { text: string; images?: AttachedImage[]; files?: AttachedTextFile[] }): {
		mergedIntoDraft: boolean
		droppedImages: number
		droppedFiles: number
	} {
		const mergedIntoDraft = !!restored.text && !!this.text.trim()
		// An attachment-only restore has empty text; prepending would only add blank lines.
		if (restored.text) {
			this.text = this.text.trim() ? `${restored.text}\n\n${this.text}` : restored.text
		}
		let droppedImages = 0
		if (restored.images?.length) {
			const merged = [...restored.images, ...this.images]
			droppedImages = Math.max(0, merged.length - MAX_ATTACHED_IMAGES)
			this.images = merged.slice(0, MAX_ATTACHED_IMAGES)
		}
		let droppedFiles = 0
		if (restored.files?.length) {
			// The restored entries were already a normalized draft; the current
			// (newer) files fold against them so dedupe/rename still apply.
			const merged = [...restored.files, ...foldIntoDraft(restored.files, this.files)]
			droppedFiles = Math.max(0, merged.length - MAX_ATTACHED_FILES)
			this.files = merged.slice(0, MAX_ATTACHED_FILES)
		}
		return { mergedIntoDraft, droppedImages, droppedFiles }
	}

	/** Replace the draft with a snapshot, but only when it is empty — an occupied
	 * draft keeps what the user is writing. Returns whether the restore was taken. */
	replaceIfEmpty(snapshot: Partial<DraftSnapshot>): boolean {
		if (!this.isEmpty) return false
		this.replace(snapshot)
		return true
	}

	/** Unconditionally replace all lanes (put a taken queue back, etc.). */
	replace(snapshot: Partial<DraftSnapshot>): void {
		this.text = snapshot.text ?? ''
		this.pastes = [...(snapshot.pastes ?? [])]
		this.images = [...(snapshot.images ?? [])]
		this.files = [...(snapshot.files ?? [])]
	}

	/** Snapshot and clear atomically — the four lanes always move together, so no
	 * call site can take one and forget another. */
	take(): DraftSnapshot {
		const snapshot: DraftSnapshot = {
			text: this.text,
			pastes: this.pastes,
			images: this.images,
			files: this.files
		}
		this.clear()
		return snapshot
	}

	clear(): void {
		this.text = ''
		this.pastes = []
		this.images = []
		this.files = []
	}
}
