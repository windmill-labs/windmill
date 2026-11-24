import { resource } from 'runed'
import { AiService } from '$lib/gen'

export type SharedSessionsFilter = 'show' | 'only' | 'hide'

export const SHARED_SESSIONS_FILTER_OPTIONS: {
	value: SharedSessionsFilter
	label: string
	hint: string
}[] = [
	{ value: 'show', label: 'Show', hint: 'Show' },
	{ value: 'only', label: 'Only shared', hint: 'Only' },
	{ value: 'hide', label: 'Hide', hint: 'Hide' }
]

// Bumped when this browser creates, changes, removes or opens a share, so a mounted list refetches.
const versions: Record<string, number> = $state({})

export function invalidateSharedSessions(workspace: string | undefined): void {
	if (!workspace) return
	versions[workspace] = (versions[workspace] ?? 0) + 1
}

/** The workspace's shares, as `listSharedAiSessions` returns them. Empty until it answers. */
export function useSharedSessionsList(workspace: () => string | undefined) {
	const list = resource(
		() => {
			const ws = workspace()
			return { ws, v: ws ? (versions[ws] ?? 0) : 0 }
		},
		async ({ ws }) => {
			if (!ws) return []
			try {
				return await AiService.listSharedAiSessions({ workspace: ws })
			} catch (err) {
				// The list still works without them: they are an addition to the reader's own.
				console.error('Could not list the shared sessions', err)
				return []
			}
		}
	)
	return {
		get items() {
			return list.current ?? []
		}
	}
}
