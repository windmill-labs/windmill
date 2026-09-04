import { logFeatureUsage } from '$lib/utils/featureUsage'

// Anonymous counters for the take rate of the alpha sandbox-isolation toggle. Same rules as
// every other `logFeatureUsage` caller: aggregated counts only, and the four keys below are
// the whole vocabulary — no app path, policy or scope ever reaches here.

/** Which editor the toggle was flipped in, since the two app kinds adopt it independently. */
export type AppSandboxKind = 'low_code' | 'raw'

/**
 * Counted where the user flips the toggle, not where the policy is persisted: a not-yet-deployed
 * app only mutates its policy locally, and dropping those would count the toggle as unused in
 * exactly the case where it is picked up front.
 */
export function logAppSandboxToggle(kind: AppSandboxKind, enabled: boolean): void {
	logFeatureUsage('app_sandbox', 'toggled', { key: `${kind}:${enabled ? 'on' : 'off'}` })
}
