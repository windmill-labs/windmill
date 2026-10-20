import { Copy, Eye, FileUp, History, List, Shield, Trash } from 'lucide-svelte'
import { ResourceService, type AiAgent, type OpenFlow } from '$lib/gen'
import BarsStaggered from '$lib/components/icons/BarsStaggered.svelte'
import { keepsManagedMemory } from './agentFormFields'
import { AGENT_CHAT_SCHEMA } from './conversations/agentEditorChat'
import { workspaceStore } from '$lib/stores'
import { get } from 'svelte/store'
import { sendUserToast } from '$lib/toast'
import { copyToClipboard, type Item } from '$lib/utils'
import { isDeployable } from '$lib/utils_deployable'
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

/**
 * Opens a new flow at the agent's own path whose one step links the agent, so pre- and
 * post-processing can be added around it while the agent stays the one place it is configured.
 * The flow chats when the agent keeps the conversation, as the agent's own page does.
 */
async function useAgentInFlow(workspace: string, path: string): Promise<void> {
	let chat: boolean
	try {
		const value = await ResourceService.getResourceValue({ workspace, path })
		chat = keepsManagedMemory((value as { memory?: unknown } | undefined)?.memory)
	} catch (err) {
		sendUserToast(`Could not read agent ${path}: ${err}`, true)
		return
	}
	const flow: OpenFlow = {
		summary: '',
		value: {
			...(chat ? { chat_input_enabled: true } : {}),
			modules: [
				{
					id: 'a',
					value: {
						type: 'aiagent',
						agent: path,
						tools: [],
						input_transforms: {
							user_message: { type: 'javascript', expr: 'flow_input.user_message' },
							user_attachments: { type: 'javascript', expr: 'flow_input.user_attachments' }
						}
					} as AiAgent
				}
			]
		},
		schema: AGENT_CHAT_SCHEMA
	}
	// The new flow editor's fork handoff, which seeds the flow and its path. A full load: on a
	// client-side one the agent page's own URL hash would be read as the flow to seed instead.
	localStorage.setItem('fork_flow', JSON.stringify({ flow, path }))
	window.location.assign('/flows/add?fork=true')
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
	onHistory: () => void
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
						// An agent run is a flow preview, which the runs page hides by default. Chat turns
						// are filed under `<path>.chat` (agent_runs.rs) and listed by the agent's own chat.
						displayName: 'View runs',
						icon: List,
						action: () => goto(`/runs/${agent.path}?job_kinds=previews`)
					},
					...(agent.operator
						? []
						: [
								{
									displayName: 'Audit logs',
									icon: Eye,
									action: () => goto(`/audit_logs?resource=${agent.path}`)
								}
							]),
					{ displayName: 'Copy path', icon: Copy, action: () => copyToClipboard(agent.path) },
					...(agent.operator
						? []
						: [
								{
									displayName: 'Use in a flow',
									description: 'A new flow at the same path, with this agent as its step',
									icon: BarsStaggered,
									iconProps: { style: 'fill: currentColor;' },
									action: () => {
										const workspace = get(workspaceStore)
										if (workspace) useAgentInFlow(workspace, agent.path)
									}
								}
							])
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
		...(agent.draftOnly || agent.operator
			? []
			: [{ displayName: 'Version history', icon: History, action: agent.onHistory }]),
		{
			displayName: 'Delete',
			icon: Trash,
			type: 'delete' as const,
			disabled: !agent.canWrite,
			action: agent.onDelete
		}
	]
}
