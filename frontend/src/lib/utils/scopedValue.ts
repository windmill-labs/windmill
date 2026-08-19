/**
 * Holds the newest fetched value that describes `key`, ignoring any other.
 *
 * `resource` does not order responses: it cancels through an `AbortSignal` the generated
 * client cannot consume, and assigns `current` unconditionally once a fetch resolves. So
 * a late answer for a workspace we have left still lands in `current`, and a publish site
 * trusting `current` alone would erase the value on screen for the workspace we are on.
 * Tagging each value with what it describes makes both cases inert.
 *
 * `loading` cannot do this job: it is also true during a `refetch()`, when `current` is
 * still the right value, so gating on it blanks the display on every re-read.
 */
export function scopedValue<T>() {
	let held: { key: string; value: T } | undefined = undefined
	return (key: string | undefined, fetched: { key: string; value: T } | undefined) => {
		if (fetched && fetched.key === key) held = fetched
		return held && held.key === key ? held.value : undefined
	}
}
