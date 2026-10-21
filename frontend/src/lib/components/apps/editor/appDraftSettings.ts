import type { Policy } from '$lib/gen'

/** Policy fields `updatePolicy` rebuilds from the app value on every deploy. */
const DERIVED_POLICY_KEYS = ['triggerables', 'triggerables_v2', 's3_inputs', 'allowed_s3_keys']

/** The part of a policy the user sets (execution mode, sandbox, on-behalf-of, SDK scopes),
 *  which an app draft carries. The derived part is left out: it describes the deployed value,
 *  and a draft holding it would differ from the deployed row after every runnable edit. */
export function policySettings(policy: Policy | Partial<Policy> | undefined): Partial<Policy> {
	const settings: Record<string, unknown> = { ...(policy ?? {}) }
	for (const k of DERIVED_POLICY_KEYS) delete settings[k]
	return settings as Partial<Policy>
}

export type AppDraftSettings = {
	policy: Partial<Policy>
	labels: string[]
	custom_path: string | null
}

/** The settings an app draft mirrors onto the App value, normalized so the editor's mirror and
 *  the deployed baseline it is compared against agree on "unchanged". */
export function appDraftSettings(s: {
	policy?: Policy | Partial<Policy>
	labels?: string[] | null
	custom_path?: string | null
}): AppDraftSettings {
	return {
		policy: policySettings(s.policy),
		labels: s.labels ?? [],
		custom_path: s.custom_path ?? null
	}
}

/** A drafted policy's settings over the deployed policy's derived fields. Every setting comes
 *  from the draft, absent ones included: a setting turned off is absent from the draft's JSON,
 *  and spreading the draft over the deployed policy would turn it back on. */
export function withDraftPolicySettings(
	deployed: Policy | undefined,
	draftSettings: Partial<Policy>
): Policy {
	const d = (deployed ?? {}) as Record<string, unknown>
	const derived = Object.fromEntries(
		DERIVED_POLICY_KEYS.filter((k) => d[k] !== undefined).map((k) => [k, d[k]])
	)
	return { ...policySettings(draftSettings), ...derived } as Policy
}

/** A classic app's settings as the draft states them, falling back field by field to the
 *  deployed row for a draft that predates mirroring them. */
export function draftOrDeployedSettings(
	parts: {
		policy?: Record<string, unknown>
		labels?: string[]
		customPath?: string | null
	},
	row: { policy?: Policy; labels?: string[] | null; custom_path?: string | null }
): AppDraftSettings {
	const deployed = appDraftSettings(row)
	return {
		policy: parts.policy ? policySettings(parts.policy as Partial<Policy>) : deployed.policy,
		labels: parts.labels ?? deployed.labels,
		custom_path: parts.customPath !== undefined ? parts.customPath : deployed.custom_path
	}
}
