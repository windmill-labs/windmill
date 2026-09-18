/** The files a user message carried, as the lanes its bubble renders: image thumbnails and file chips. */
import type { ChatAttachment } from 'windmill-chat'
import { base } from '$lib/base'
import {
	createAttachedFileContextElement,
	type ContextElement
} from '$lib/components/copilot/chat/context'
import type { AttachedImage } from '$lib/components/copilot/chat/imageUtils'

const IMAGE_EXTENSIONS = ['.png', '.jpg', '.jpeg', '.gif', '.webp', '.bmp', '.svg', '.avif']

export type MessageAttachments = { images: AttachedImage[]; contextElements: ContextElement[] }

function displayName(attachment: ChatAttachment): string {
	return attachment.filename || attachment.s3.split('/').pop() || attachment.s3
}

function looksLikeImage(attachment: ChatAttachment): boolean {
	const name = displayName(attachment).toLowerCase()
	return IMAGE_EXTENSIONS.some((ext) => name.endsWith(ext))
}

/** Same-origin, cookie-authed GET, usable directly as an <img src>. */
function downloadUrl(workspace: string, attachment: ChatAttachment): string {
	const params = new URLSearchParams({ file_key: attachment.s3 })
	if (attachment.storage) params.set('storage', attachment.storage)
	return `${base}/api/w/${workspace}/job_helpers/download_s3_file?${params.toString()}`
}

/** Without a workspace there is no link to build, so an image falls back to a file chip. */
export function attachmentLanes(
	workspace: string | undefined,
	attachments: readonly ChatAttachment[] | undefined
): MessageAttachments {
	const images: AttachedImage[] = []
	const contextElements: ContextElement[] = []
	for (const attachment of attachments ?? []) {
		const name = displayName(attachment)
		if (workspace && looksLikeImage(attachment)) {
			images.push({ dataUrl: downloadUrl(workspace, attachment), mediaType: 'image/png', name })
		} else {
			contextElements.push(
				createAttachedFileContextElement(name, `Attached file · ${attachment.s3}`)
			)
		}
	}
	return { images, contextElements }
}
