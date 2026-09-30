import { FileUp, Shield, Trash } from 'lucide-svelte'
import { ResourceService } from '$lib/gen'
import { sendUserToast } from '$lib/toast'
import type { Item } from '$lib/utils'
import { isDeployable } from '$lib/utils_deployable'
import { getDeployUiSettings } from '$lib/components/home/deploy_ui'

/** Deletes a saved agent, saying how it went. Resolves whether it was deleted. */
export async function deleteAgent(workspace: string, path: string): Promise<boolean> {
	try {
		await ResourceService.deleteResource({ workspace, path })
		sendUserToast(`Deleted agent ${path}`)
		return true
	} catch (err) {
		sendUserToast(`Could not delete agent ${path}: ${err}`, true)
		return false
	}
}

/** An agent's menu, as its home row and its page both show it. */
export async function agentMenuItems(agent: {
	path: string
	canWrite: boolean
	/** Never deployed: no resource yet to hold permissions or to deploy onward. */
	draftOnly?: boolean
	wsSpecific?: boolean
	onPermissions: () => void
	onDeploy: () => void
	onDelete: (event?: MouseEvent) => void
}): Promise<Item[]> {
	const deployable =
		!agent.wsSpecific &&
		!agent.draftOnly &&
		isDeployable('resource', agent.path, await getDeployUiSettings())
	return [
		{
			displayName: 'Permissions',
			icon: Shield,
			disabled: !agent.canWrite || Boolean(agent.draftOnly),
			action: agent.onPermissions
		},
		...(deployable
			? [{ displayName: 'Deploy to prod/staging', icon: FileUp, action: agent.onDeploy }]
			: []),
		{
			displayName: 'Delete',
			icon: Trash,
			type: 'delete' as const,
			disabled: !agent.canWrite,
			action: agent.onDelete
		}
	]
}
