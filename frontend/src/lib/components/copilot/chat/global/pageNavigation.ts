import { buildFilterUrl } from '$lib/navigation'
import { buildRunsFilterSearchbarSchema } from '$lib/components/runs/runsFilter'
import { buildSchedulesFilterSchema } from '$lib/components/schedules/schedulesFilter'

// In-app paths for the deep-linkable preview pages the AI chat can open.
export const RUNS_PATH = '/runs'
export const SCHEDULES_PATH = '/schedules'

// Valid query-param keys are derived from the real filter schemas (option arrays are
// irrelevant to the key set), so a renamed filter key propagates here for free.
const RUNS_FILTER_KEYS = Object.keys(
	buildRunsFilterSearchbarSchema({
		paths: [],
		usernames: [],
		folders: [],
		jobTriggerKinds: [],
		isSuperAdminOrDevops: false,
		isAdminsWorkspace: false
	})
)
const SCHEDULES_FILTER_KEYS = Object.keys(
	buildSchedulesFilterSchema({ paths: [], scriptPaths: [] })
)

/** Deep-link to the Runs page with the given filters (keys must match `runsFilter`). */
export function buildRunsUrl(filters: Record<string, unknown>): string {
	return buildFilterUrl(RUNS_PATH, filters, { validKeys: RUNS_FILTER_KEYS })
}

/**
 * Deep-link to the Schedules page. When `open` is set, the schedule at that exact path
 * is opened in the edit drawer via the `#<schedule_path>` hash the page already handles.
 */
export function buildSchedulesUrl({
	open,
	filters
}: {
	open?: string
	filters?: Record<string, unknown>
}): string {
	return buildFilterUrl(SCHEDULES_PATH, filters ?? {}, {
		validKeys: SCHEDULES_FILTER_KEYS,
		hash: open
	})
}
