// A folder's data-pipeline drafts all live in ONE bundle stored at
// `f/<folder>/data_pipeline`, while the editor for it is the folder's pipeline
// view. Every surface that routes between the two — the drafts compare page,
// the home list, the session preview panel — needs that layout, so it is stated
// here once rather than re-parsed per call site.
export const PIPELINE_DRAFT_KIND = 'data_pipeline' as const

export function pipelineBundlePath(folder: string): string {
	return `f/${folder}/${PIPELINE_DRAFT_KIND}`
}

const BUNDLE_PATH_RE = new RegExp(`^f/([^/]+)/${PIPELINE_DRAFT_KIND}$`)

/** The folder a pipeline bundle belongs to, or `undefined` if the path isn't one. */
export function pipelineFolderFromBundlePath(path: string): string | undefined {
	return path.match(BUNDLE_PATH_RE)?.[1]
}
