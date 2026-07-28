/** An input waiting to receive the next picked property. */
export type ArmedTarget = {
	/** Distinguishes targets within a panel — an argument name, or the input's own id. */
	id: string
	onSelect: (path: string) => void
}

/**
 * Arming is exclusive: a panel has at most one target, so a pick — from the picker or from
 * a step node — always has exactly one destination. Clicking the armed input again disarms.
 */
export function nextArmed(
	current: ArmedTarget | undefined,
	clicked: ArmedTarget
): ArmedTarget | undefined {
	return current?.id === clicked.id ? undefined : clicked
}

/**
 * Whether the graph offers its step outputs while this target is armed. The modal panel
 * covers the graph, so connecting there could never be completed; with nothing pickable
 * there is nothing for a node click to contribute.
 */
export function graphParticipates(
	armed: ArmedTarget | undefined,
	opts: { inModalPanel: boolean; hasPickableProperties: boolean }
): boolean {
	return armed != undefined && !opts.inModalPanel && opts.hasPickableProperties
}
