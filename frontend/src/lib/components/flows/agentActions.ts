import { Copy, Eye, FileUp, List, Shield, Trash } from 'lucide-svelte'
import { ResourceService } from '$lib/gen'
import { sendUserToast } from '$lib/toast'
import { copyToClipboard, type Item } from '$lib/utils'
import { isDeployable } from '$lib/utils_deployable'
import { base } from '$lib/base'
import { goto } from '$lib/navigation'
import type { WorkspaceDeployUISettings } from '$lib/gen'

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
export function agentMenuItems(agent: {
	path: string
	canWrite: boolean
	/** Never deployed: no resource yet to hold permissions or to deploy onward. */
	draftOnly?: boolean
	wsSpecific?: boolean
	/** Hides what an operator has no page for, as on a flow's menu. */
	operator?: boolean
	/** Undefined until loaded, when nothing is offered for deploy. */
	deployUiSettings: WorkspaceDeployUISettings | undefined
	onPermissions: () => void
	onDeploy: () => void
	onDelete: (event?: MouseEvent) => void
}): Item[] {
	const deployable =
		!agent.wsSpecific &&
		!agent.draftOnly &&
		isDeployable('resource', agent.path, agent.deployUiSettings)
	return [
		// A draft-only agent has no runs, no audit trail, and only a placeholder path yet.
		...(agent.draftOnly
			? []
			: [
					{
						// Chat turns are filed under `<path>.chat` (agent_runs.rs) and listed by the agent's
						// own chat, so this is the form runs.
						displayName: 'View runs',
						icon: List,
						action: () => goto(`${base}/runs/${agent.path}`)
					},
					...(agent.operator
						? []
						: [
								{
									displayName: 'Audit logs',
									icon: Eye,
									action: () => goto(`${base}/audit_logs?resource=${agent.path}`)
								}
							]),
					{ displayName: 'Copy path', icon: Copy, action: () => copyToClipboard(agent.path) }
				]),
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
