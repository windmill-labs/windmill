/** Deep-clones reactive state into plain values.
 *
 * A leaf on purpose: `$lib/svelte5Utils.svelte` re-exports this but also reaches
 * `$lib/utils`, and through it the icon and store modules. Anything that only
 * needs a snapshot (the `sharedUtils` entry point, most of all) imports it from
 * here so it doesn't drag the UI graph along.
 *
 * Return type annotated because the inferred `Snapshot<T>` is a deep conditional
 * type TS refuses to serialize into a declaration file (TS7056). It is not
 * exported from the `$state` namespace, and only differs from `T` for
 * Date/Map/Set, which no caller snapshots.
 */
export function stateSnapshot<T>(state: T): T {
	return $state.snapshot(state) as T
}
