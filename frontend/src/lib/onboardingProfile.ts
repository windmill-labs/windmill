import { UserService } from './gen'
import { WORKSPACE_NAME_MAX_LENGTH } from './utils/workspaceId'

/**
 * What an invited cloud account arrives knowing about itself: the context the invite
 * carried, recorded by the provisioning script before the person ever signs in. Every key
 * is optional and the whole thing is absent for a self-signup, so each consumer must read
 * it as a hint and fall back to its default.
 *
 * The blob is written by an outbound pipeline that may grow new keys at any time; this is
 * the only place that decides which of them mean anything to the frontend, and how far a
 * value is trusted before it is shown.
 */
export interface OnboardingProfile {
	/** Answers onboarding's source question; `outbound:<campaign>`. */
	touch_point?: string
	company?: string
	/** What to call the first workspace; `company` is the fallback. */
	workspace_name?: string
	/** Hub project slugs to surface first on an empty workspace. */
	hub_projects?: string[]
	/** Replace the home page's example prompts with ones written for this person. */
	starter_prompts?: StarterPrompt[]
	/** Integrations they are known to use, for picking hub projects when none are named. */
	tools?: string[]
}

export interface StarterPrompt {
	label: string
	prompt: string
}

const MAX_LIST = 12
const MAX_LABEL = 40
const MAX_PROMPT = 500

function str(v: unknown, max: number): string | undefined {
	if (typeof v !== 'string') return undefined
	const t = v.trim()
	return t && t.length <= max ? t : undefined
}

function strList(v: unknown, max: number): string[] | undefined {
	if (!Array.isArray(v)) return undefined
	const out = v.map((x) => str(x, max)).filter((x): x is string => !!x)
	return out.length ? out.slice(0, MAX_LIST) : undefined
}

/**
 * The keys the frontend acts on, shape-checked one by one. A malformed key is dropped, not
 * the whole profile: the touch point that skips a survey question must survive a pipeline
 * that produced a bad prompt list, and vice versa.
 */
export function parseOnboardingProfile(raw: unknown): OnboardingProfile | null {
	if (!raw || typeof raw !== 'object' || Array.isArray(raw)) return null
	const r = raw as Record<string, unknown>
	// Labels are what the home page keys its tags by, so two prompts sharing one would
	// break its list: the first wins.
	const seen = new Set<string>()
	const prompts = Array.isArray(r.starter_prompts)
		? r.starter_prompts
				.map((p) => {
					if (!p || typeof p !== 'object') return undefined
					const label = str((p as Record<string, unknown>).label, MAX_LABEL)
					const prompt = str((p as Record<string, unknown>).prompt, MAX_PROMPT)
					return label && prompt ? { label, prompt } : undefined
				})
				.filter((p): p is StarterPrompt => !!p && !seen.has(p.label) && !!seen.add(p.label))
				.slice(0, MAX_LIST)
		: []
	const profile: OnboardingProfile = {
		touch_point: str(r.touch_point, 200),
		company: str(r.company, WORKSPACE_NAME_MAX_LENGTH),
		workspace_name: str(r.workspace_name, WORKSPACE_NAME_MAX_LENGTH),
		hub_projects: strList(r.hub_projects, 100),
		starter_prompts: prompts.length ? prompts : undefined,
		tools: strList(r.tools, 50)?.map((t) => t.toLowerCase())
	}
	return Object.values(profile).some((v) => v !== undefined) ? profile : null
}

let cached: Promise<OnboardingProfile | null> | undefined
let cachedFor: string | undefined

/**
 * The profile belongs to a session, not to the page: sign-in and sign-out are client-side
 * navigations, so a cache keyed on nothing would hand account A's invite context to
 * account B signing in on the same tab. The root layout reports the session's address
 * whenever it learns it, and logout clears; a change drops what was cached.
 */
export function noteSessionEmail(email: string | undefined) {
	if (email !== cachedFor) {
		cached = undefined
		cachedFor = email
	}
}

/**
 * The signed-in account's profile, fetched once per session. Off cloud the server answers
 * `null` without a lookup, so the cost is one request per session. A failed fetch reads as
 * "no profile" and is retried on the next call rather than cached.
 */
export function onboardingProfile(): Promise<OnboardingProfile | null> {
	if (!cached) {
		const p = UserService.getOnboardingProfile()
			.then((r) => parseOnboardingProfile(r.profile))
			.catch((e) => {
				console.error('Could not read the onboarding profile:', e)
				if (cached === p) cached = undefined
				return null
			})
		cached = p
	}
	return cached
}
