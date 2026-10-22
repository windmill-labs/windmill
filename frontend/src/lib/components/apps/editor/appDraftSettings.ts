import type { Policy } from '$lib/gen'

/** Policy fields a draft leaves to the deployed row: the ones `updatePolicy` rebuilds from the
 *  app value on every deploy, and who can open the app, which the editor holds until Deploy. */
const UNDRAFTED_POLICY_KEYS = [
	'triggerables',
	'triggerables_v2',
	's3_inputs',
	'allowed_s3_keys',
	'execution_mode'
]

/** The part of a policy an app draft carries (sandbox, on-behalf-of, SDK scopes). The derived
 *  part describes the deployed value, and a draft holding it would differ from the deployed row
 *  after every runnable edit. */
export function policySettings(policy: Policy | Partial<Policy> | undefined): Partial<Policy> {
	const settings: Record<string, unknown> = { ...(policy ?? {}) }
	for (const k of UNDRAFTED_POLICY_KEYS) delete settings[k]
	return settings as Partial<Policy>
}

/** A raw app's policy without who can open the app, which the Share dialog writes straight to a
 *  deployed app: a raw app drafts the rest of its policy whole. */
export function withoutSharedPolicySettings<T>(policy: T): T {
	if (!policy || typeof policy !== 'object') return policy
	const { execution_mode: _, ...rest } = policy as Record<string, unknown>
	return rest as T
}

export type AppDraftSettings = {
	policy: Partial<Policy>
	labels: string[]
}

/** The settings an app draft mirrors onto the App value, normalized so the editor's mirror and
 *  the deployed baseline it is compared against agree on "unchanged". */
export function appDraftSettings(s: {
	policy?: Policy | Partial<Policy>
	labels?: string[] | null
}): AppDraftSettings {
	return {
		policy: policySettings(s.policy),
		labels: s.labels ?? []
	}
}

/** A drafted policy's settings over the deployed policy's undrafted fields. Every setting comes
 *  from the draft, absent ones included: a setting turned off is absent from the draft's JSON,
 *  and spreading the draft over the deployed policy would turn it back on. */
export function withDraftPolicySettings(
	deployed: Policy | undefined,
	draftSettings: Partial<Policy>
): Policy {
	const d = (deployed ?? {}) as Record<string, unknown>
	const undrafted = Object.fromEntries(
		UNDRAFTED_POLICY_KEYS.filter((k) => d[k] !== undefined).map((k) => [k, d[k]])
	)
	return { ...policySettings(draftSettings), ...undrafted } as Policy
}

/** A classic app's settings as the draft states them, falling back field by field to the
 *  deployed row for a draft that predates mirroring them. */
export function draftOrDeployedSettings(
	parts: {
		policy?: Record<string, unknown>
		labels?: string[]
	},
	row: { policy?: Policy; labels?: string[] | null }
): AppDraftSettings {
	const deployed = appDraftSettings(row)
	return {
		policy: parts.policy ? policySettings(parts.policy as Partial<Policy>) : deployed.policy,
		labels: parts.labels ?? deployed.labels
	}
}
