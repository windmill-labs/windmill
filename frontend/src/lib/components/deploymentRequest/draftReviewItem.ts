import { getDraftDiffValues } from '$lib/utils_draft_deploy'
import type { UserDraftItemKind } from '$lib/gen'
import type { ReviewItem } from './aiReview'

/** A draft to review against its deployed version, read the way its diff is shown. */
export function draftReviewItem(
	kind: UserDraftItemKind,
	path: string,
	workspaceId: string,
	draftOnly: boolean
): ReviewItem {
	return {
		kind,
		path,
		change: draftOnly ? 'added' : 'modified',
		load: async () => {
			const { deployed, draft } = await getDraftDiffValues(kind, path, workspaceId, draftOnly)
			return { before: deployed, after: draft }
		}
	}
}
