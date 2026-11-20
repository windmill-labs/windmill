import type { Policy, UserDraftItemKind } from '$lib/gen'
import { accessModeOf, type AppAccessMode } from '$lib/components/itemSettings/appAccess'
import type { DeployCardIdentity } from './DeployCardView.svelte'
import type { OnBehalfOf } from './global/deployEdits'

/** Where each kind keeps the user it runs on behalf of. */
function identityOf(kind: UserDraftItemKind, value: unknown): OnBehalfOf {
	const v = (value ?? {}) as Record<string, any>
	const src = kind === 'raw_app' || kind === 'app' ? (v.policy ?? {}) : v
	return { email: src.on_behalf_of_email, permissionedAs: src.on_behalf_of }
}

/** The run-as setting of a deploy, for the kinds that carry one. Others run as their caller, or
 *  keep an identity the chat does not read. */
export function deployIdentity(
	kind: UserDraftItemKind,
	deploy: { deployed: unknown; draft: unknown; isNew: boolean; folderDefault: string | undefined }
): DeployCardIdentity | undefined {
	if (kind !== 'script' && kind !== 'flow' && kind !== 'raw_app' && kind !== 'app') return undefined
	return {
		kind: kind === 'raw_app' ? 'app' : kind,
		held: identityOf(kind, deploy.draft),
		saved: deploy.isNew
			? { email: undefined, permissionedAs: undefined }
			: identityOf(kind, deploy.deployed),
		folderDefault: deploy.isNew ? deploy.folderDefault : undefined,
		rawApp: kind === 'raw_app'
	}
}

/** Who can open a new app as its draft sets it. Undefined for an update, which keeps the deployed
 *  access that only the Share dialog changes. */
export function newAppAccessMode(
	kind: UserDraftItemKind,
	draft: unknown,
	isNew: boolean
): AppAccessMode | undefined {
	if (!isNew || (kind !== 'raw_app' && kind !== 'app')) return undefined
	return accessModeOf((draft as { policy?: Policy } | undefined)?.policy)
}
