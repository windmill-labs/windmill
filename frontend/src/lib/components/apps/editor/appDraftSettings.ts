import type { Policy } from '$lib/gen'

/** Policy fields `updatePolicy` rebuilds from the app value on every deploy. */
const DERIVED_POLICY_KEYS = ['triggerables', 'triggerables_v2', 's3_inputs', 'allowed_s3_keys']

/** Policy fields the Share dialog writes straight to a deployed app, as `custom_path` is. A draft
 *  of a deployed app does not carry them and a deploy takes them from the deployed row, so a
 *  draft saved before a Share change cannot revert it. A never-deployed app drafts them, and its
 *  first deploy creates the app with them. */
const SHARED_POLICY_KEYS = ['execution_mode']

/** The part of a policy the user sets (execution mode, sandbox, on-behalf-of, SDK scopes),
 *  which an app draft carries. The derived part is left out: it describes the deployed value,
 *  and a draft holding it would differ from the deployed row after every runnable edit. With
 *  `deployed`, the shared part is left out too. */
export function policySettings(
	policy: Policy | Partial<Policy> | undefined,
	deployed = false
): Partial<Policy> {
	const settings: Record<string, unknown> = { ...(policy ?? {}) }
	for (const k of DERIVED_POLICY_KEYS) delete settings[k]
	if (deployed) for (const k of SHARED_POLICY_KEYS) delete settings[k]
	return settings as Partial<Policy>
}

/** A policy without the fields the Share dialog sets, for a raw app's draft, which keeps the rest
 *  of the policy whole. */
export function withoutSharedPolicySettings<T>(policy: T): T {
	if (!policy || typeof policy !== 'object') return policy
	const rest: Record<string, unknown> = { ...(policy as Record<string, unknown>) }
	for (const k of SHARED_POLICY_KEYS) delete rest[k]
	return rest as T
}

export type AppDraftSettings = {
	policy: Partial<Policy>
	labels: string[]
	/** Null when not drafted, which is always the case for a deployed app. */
	custom_path: string | null
}

/** The settings an app draft mirrors onto the App value, normalized so the editor's mirror and
 *  the deployed baseline it is compared against agree on "unchanged". `deployed` says whether
 *  the app has a deployed row, whose shared settings the draft then leaves out. */
export function appDraftSettings(
	s: {
		policy?: Policy | Partial<Policy>
		labels?: string[] | null
		custom_path?: string | null
	},
	deployed: boolean
): AppDraftSettings {
	return {
		policy: policySettings(s.policy, deployed),
		labels: s.labels ?? [],
		custom_path: deployed ? null : (s.custom_path ?? null)
	}
}

/** A drafted policy's settings over the deployed policy's derived fields. Every setting comes
 *  from the draft, absent ones included: a setting turned off is absent from the draft's JSON,
 *  and spreading the draft over the deployed policy would turn it back on. For an app that is
 *  `deployed`, the shared settings come from the deployed policy instead. */
export function withDraftPolicySettings(
	deployed: Policy | undefined,
	draftSettings: Partial<Policy>,
	isDeployed: boolean
): Policy {
	const d = (deployed ?? {}) as Record<string, unknown>
	const fromDeployed = isDeployed
		? [...DERIVED_POLICY_KEYS, ...SHARED_POLICY_KEYS]
		: DERIVED_POLICY_KEYS
	const kept = Object.fromEntries(
		fromDeployed.filter((k) => d[k] !== undefined).map((k) => [k, d[k]])
	)
	return { ...policySettings(draftSettings, isDeployed), ...kept } as Policy
}

/** A classic app's settings as the draft states them, falling back field by field to `row` for a
 *  draft that predates mirroring them: the deployed row, or what a new app starts with. Whether
 *  the app is deployed is passed, not read from `row`, which carries a policy either way. */
export function draftOrDeployedSettings(
	parts: {
		policy?: Record<string, unknown>
		labels?: string[]
		customPath?: string | null
	},
	row: { policy?: Policy; labels?: string[] | null; custom_path?: string | null },
	isDeployed: boolean
): AppDraftSettings {
	const fallback = appDraftSettings(row, isDeployed)
	return {
		policy: parts.policy
			? policySettings(parts.policy as Partial<Policy>, isDeployed)
			: fallback.policy,
		labels: parts.labels ?? fallback.labels,
		custom_path:
			!isDeployed && parts.customPath !== undefined ? parts.customPath : fallback.custom_path
	}
}
