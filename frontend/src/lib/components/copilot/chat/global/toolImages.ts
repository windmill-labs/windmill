import { normalizeImageDataUrl, type AttachedImage } from '../imageUtils'
import { modelSupportsVision } from '../../modelConfig'
import { tryGetCurrentModel } from '$lib/aiStore'
import type { GlobalToolHelpers } from './core'

/** Whether the chat's model reads images. Re-read per call: a session's model can change
 * between turns, and a text-only model rejects the image message, failing the turn. */
export function chatModelSeesImages(helpers: unknown): boolean {
	const sessionAi = (helpers as GlobalToolHelpers | undefined)?.sessionAi
	const model = sessionAi ? sessionAi().model : tryGetCurrentModel()
	return !model || modelSupportsVision(model.provider, model.model)
}

/** A tool's base64 images, decoded and downscaled for the model; each one it cannot be shown
 * is named in `omitted`, for the tool's text to say so. */
export async function toolImagesFor(
	raw: { name: string; mimeType: string; data: string }[],
	canSeeImages: boolean
): Promise<{ images: AttachedImage[]; omitted: string[] }> {
	const images: AttachedImage[] = []
	const omitted: string[] = []
	for (const image of raw) {
		if (!canSeeImages) {
			omitted.push(`${image.name}: the current model cannot read images`)
			continue
		}
		try {
			images.push(await normalizeImageDataUrl(`data:${image.mimeType};base64,${image.data}`))
		} catch {
			omitted.push(`${image.name}: could not be decoded`)
		}
	}
	return { images, omitted }
}
