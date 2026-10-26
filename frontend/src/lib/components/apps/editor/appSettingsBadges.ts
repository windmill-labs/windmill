import { Globe, Link, PlugZap, ShieldCheck, Tag, UserCog, Users } from 'lucide-svelte'
import type { Policy } from '$lib/gen'
import type { ScriptSettingsBadge } from '$lib/components/scriptSettings'

/** The badges of an app's Metadata tab. */
export function getAppMetadataBadges(labels: string[] | undefined): ScriptSettingsBadge[] {
	return labels?.length
		? [{ key: 'labels', label: 'Labels', icon: Tag, detail: labels.join(', ') }]
		: []
}

/** The badges of an app's Permissions tab. `sandboxed` also holds when the role forces it. */
export function getAppPermissionBadges(
	policy: Pick<Policy, 'on_behalf_of' | 'on_behalf_of_email' | 'frontend_sdk_scopes'>,
	sandboxed: boolean
): ScriptSettingsBadge[] {
	const badges: ScriptSettingsBadge[] = []
	const runsAs = policy.on_behalf_of ?? policy.on_behalf_of_email
	if (runsAs) {
		badges.push({ key: 'on_behalf_of', label: 'Run on behalf of', icon: UserCog, detail: runsAs })
	}
	if (sandboxed) {
		badges.push({
			key: 'sandbox',
			label: 'Sandbox isolation',
			icon: ShieldCheck,
			detail: "The app's browser-side code gets a narrowly scoped token"
		})
		if (policy.frontend_sdk_scopes?.length) {
			badges.push({
				key: 'frontend_sdk_scopes',
				label: 'Frontend API access',
				icon: PlugZap,
				detail: policy.frontend_sdk_scopes.join(', ')
			})
		}
	}
	return badges
}

/** The badges of an app's Sharing tab: who beyond members can open it, and its custom URL. */
export function getAppSharingBadges(
	executionMode: Policy['execution_mode'],
	customPath: string | undefined
): ScriptSettingsBadge[] {
	const badges: ScriptSettingsBadge[] = []
	if (executionMode === 'anonymous') {
		badges.push({
			key: 'access',
			label: 'Public',
			icon: Globe,
			detail: 'Anyone with the secret URL, no login'
		})
	} else if (executionMode === 'guest') {
		badges.push({
			key: 'access',
			label: 'Guests',
			icon: Users,
			detail: 'Guests your identity provider authenticates, plus members'
		})
	}
	if (customPath) {
		badges.push({ key: 'custom_path', label: 'Custom URL', icon: Link, detail: customPath })
	}
	return badges
}
