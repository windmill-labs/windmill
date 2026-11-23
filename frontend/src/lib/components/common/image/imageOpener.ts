import { getContext, setContext } from 'svelte'
import type { ImageSource } from '$lib/components/sessions/previewRouter'

/** Opens an image in a viewer beside the current surface: a session's preview panel. */
export type ImageOpener = (source: ImageSource, label: string) => void

const KEY = 'imageOpener'

/** Provided by a session for everything it renders, the chat and the preview tabs. */
export function setImageOpener(opener: ImageOpener): void {
	setContext(KEY, opener)
}

/** Undefined outside a session: an image then expands in place. */
export function getImageOpener(): ImageOpener | undefined {
	return getContext<ImageOpener | undefined>(KEY)
}
