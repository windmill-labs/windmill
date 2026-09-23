import { UserService } from '$lib/gen'

/**
 * The tour's slot in the `tutorial_progress` bitmask. Slot 6 is reserved for it across
 * versions: an operator who has already been through the tour must not meet it again, and
 * a slot that another tutorial writes would read as finished on day one.
 */
const OPERATOR_TOUR_BIT = 6

/** URL parameter the sidebar entry uses to ask the home page for a run. */
export const TOUR_PARAM = 'tour'
export const TOUR_PARAM_VALUE = 'operator'

/** Long enough for the home page's tabs to exist before the first step points at one. */
export const TOUR_START_DELAY_MS = 500
/** Time for the sidebar to open before the last step points into it. */
export const MENU_OPEN_DELAY_MS = 300

export async function hasSeenOperatorTour(): Promise<boolean> {
	// A failure answers "seen": the tour interrupts the page, and interrupting someone who
	// has already been through it is worse than never offering it, which the sidebar entry
	// covers anyway.
	try {
		const progress = (await UserService.getTutorialProgress()).progress ?? 0
		return (progress & (1 << OPERATOR_TOUR_BIT)) !== 0
	} catch (error) {
		console.error('Could not read tutorial progress:', error)
		return true
	}
}

export async function markOperatorTourSeen(): Promise<void> {
	try {
		// Read-modify-write, because the row is shared: it carries every slot's state, and a
		// write of this bit alone would clear the rest. `skipped_all` rides along for the same
		// reason — and the handler rejects a body without it, whatever the generated type says.
		const current = await UserService.getTutorialProgress()
		await UserService.updateTutorialProgress({
			requestBody: {
				progress: (current.progress ?? 0) | (1 << OPERATOR_TOUR_BIT),
				skipped_all: current.skipped_all ?? false
			}
		})
	} catch (error) {
		console.error('Could not record tutorial progress:', error)
	}
}
