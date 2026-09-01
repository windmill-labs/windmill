/**
 * The inputs a chat message ran with, recovered from its job.
 *
 * A conversation message stores only its text, so what the user attached and which
 * settings the turn used exist nowhere but the run's arguments. The user row carries
 * the flow job id, whose args are the raw run arguments — `user_message` plus every
 * other flow input.
 *
 * Fetched lazily and kept in memory only: a purged job leaves a dangling id and the
 * turn simply shows no inputs, which is honest — the arguments are gone.
 */
import { JobService } from '$lib/gen'
import { base } from '$lib/base'
import {
	createAttachedFileContextElement,
	type ContextElement
} from '$lib/components/copilot/chat/context'
import type { AttachedImage } from '$lib/components/copilot/chat/imageUtils'

type S3Ref = { s3: string; filename?: string; storage?: string }

const IMAGE_EXTENSIONS = ['.png', '.jpg', '.jpeg', '.gif', '.webp', '.bmp', '.svg', '.avif']

function isS3Ref(value: any): value is S3Ref {
	return !!value && typeof value === 'object' && typeof value.s3 === 'string' && value.s3 !== ''
}

function s3Refs(value: any): S3Ref[] {
	if (isS3Ref(value)) return [value]
	if (Array.isArray(value)) return value.filter(isS3Ref)
	return []
}

function displayName(ref: S3Ref): string {
	return ref.filename ?? ref.s3.split('/').pop() ?? ref.s3
}

function looksLikeImage(ref: S3Ref): boolean {
	const name = displayName(ref).toLowerCase()
	return IMAGE_EXTENSIONS.some((ext) => name.endsWith(ext))
}

/** Same-origin, cookie-authed GET — usable directly as an <img src>, no blob fetch. */
function downloadUrl(workspace: string, ref: S3Ref): string {
	const params = new URLSearchParams({ file_key: ref.s3 })
	if (ref.storage) params.set('storage', ref.storage)
	return `${base}/api/w/${workspace}/job_helpers/download_s3_file?${params.toString()}`
}

/** A scalar input, summarised for a chip. Objects are left to the file/JSON branches. */
function scalarSummary(value: any): string | undefined {
	if (value === undefined || value === null || value === '') return undefined
	if (typeof value === 'string') return value
	if (typeof value === 'number' || typeof value === 'boolean') return String(value)
	return undefined
}

export type MessageInputs = { images: AttachedImage[]; contextElements: ContextElement[] }

const EMPTY: MessageInputs = { images: [], contextElements: [] }

/**
 * Split a turn's run arguments into the lanes a user message renders: image
 * thumbnails, and a chip per remaining input. `user_message` is the bubble itself.
 */
export function argsToMessageInputs(
	workspace: string,
	args: Record<string, any> | undefined
): MessageInputs {
	if (!args) return EMPTY
	const images: AttachedImage[] = []
	const contextElements: ContextElement[] = []
	for (const [name, value] of Object.entries(args)) {
		if (name === 'user_message') continue
		const files = s3Refs(value)
		if (files.length > 0) {
			for (const ref of files) {
				if (looksLikeImage(ref)) {
					images.push({
						dataUrl: downloadUrl(workspace, ref),
						mediaType: 'image/png',
						name: displayName(ref)
					})
				} else {
					contextElements.push(
						createAttachedFileContextElement(displayName(ref), `Attached file · ${ref.s3}`)
					)
				}
			}
			continue
		}
		const summary = scalarSummary(value)
		if (summary !== undefined) {
			contextElements.push(createAttachedFileContextElement(name, summary))
		}
	}
	return images.length > 0 || contextElements.length > 0 ? { images, contextElements } : EMPTY
}

/**
 * Per-conversation cache of run arguments by job id. One fetch per turn, kept only
 * for as long as the chat is mounted.
 */
export class MessageInputsStore {
	#workspace: () => string | undefined
	#byJob = $state<Record<string, MessageInputs>>({})
	#inFlight = new Set<string>()

	constructor(workspace: () => string | undefined) {
		this.#workspace = workspace
	}

	/** What the turn ran with, fetching on first ask. Empty until the args land. */
	get(jobId: string | null | undefined): MessageInputs {
		if (!jobId) return EMPTY
		const cached = this.#byJob[jobId]
		if (cached) return cached
		void this.#load(jobId)
		return EMPTY
	}

	async #load(jobId: string) {
		const workspace = this.#workspace()
		if (!workspace || this.#inFlight.has(jobId)) return
		this.#inFlight.add(jobId)
		try {
			const args = await JobService.getJobArgs({ workspace, id: jobId })
			this.#byJob = { ...this.#byJob, [jobId]: argsToMessageInputs(workspace, args as any) }
		} catch {
			// A purged job, or one this user cannot read: the turn shows no inputs.
			this.#byJob = { ...this.#byJob, [jobId]: EMPTY }
		} finally {
			this.#inFlight.delete(jobId)
		}
	}
}
