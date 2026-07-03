import { resource } from 'runed'
import { JobService, type ListDispatchEventsResponse } from '$lib/gen'

export type DispatchEvent = ListDispatchEventsResponse[number]

// Shared loader for "the jobs this run dispatched". Both the inline button and
// the run-detail panel render the same list, so keep the fetch in one place.
// Returns a getter for the (never-undefined) event list — empty while loading,
// when there's nothing to load, or before workspace/jobId are known.
export function useDispatchEvents(
	workspace: () => string,
	jobId: () => string
): { readonly list: DispatchEvent[] } {
	const events = resource(
		() => ({ workspace: workspace(), jobId: jobId() }),
		async ({ workspace, jobId }) =>
			workspace && jobId ? await JobService.listDispatchEvents({ workspace, id: jobId }) : []
	)
	return {
		get list() {
			return events.current ?? []
		}
	}
}
