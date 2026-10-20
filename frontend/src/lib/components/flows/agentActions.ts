import { Copy, Eye, FileUp, History, List, Shield, Trash } from 'lucide-svelte'
import { ResourceService, type AiAgent, type OpenFlow } from '$lib/gen'
import { keepsManagedMemory } from './agentFormFields'
import type { AIAgentConfig } from './agentResourceUtils'
import { AGENT_CHAT_SCHEMA } from './conversations/agentEditorChat'
import { inlineAgentDraft } from './linkedAgentDrafts'
import { nextId } from './flowModuleNextId'
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
 * Opens a new flow whose one step is the agent, reading the message and files from the flow's
 * inputs, and chatting when the agent keeps the conversation. The step links the deployed agent
 * at `path`, or with `inline` carries `config` itself, for an agent that was never deployed.
 * `beforeOpen` runs once the flow is handed over, and a throw from it keeps the page where it is.
 */
export async function openAgentInFlow(opts: {
	path: string
	config: AIAgentConfig
	inline: boolean
	beforeOpen?: () => Promise<unknown>
}): Promise<void> {
	const linked = {
		type: 'aiagent',
		agent: opts.path,
		tools: [],
		input_transforms: {
			user_message: { type: 'javascript', expr: 'flow_input.user_message' },
			user_attachments: { type: 'javascript', expr: 'flow_input.user_attachments' }
		}
	} as AiAgent & { agent: string }
	const value = opts.inline ? inlineAgentDraft(linked, opts.config) : linked
	// An inlined agent brings its tools' ids, which the step's own must not repeat.
	const id = nextId({}, { summary: '', value: { modules: [{ id: '', value }] } })
	const flow: OpenFlow = {
		summary: '',
		value: {
			...(keepsManagedMemory(opts.config.memory) ? { chat_input_enabled: true } : {}),
			modules: [{ id, value }]
		},
		schema: AGENT_CHAT_SCHEMA
	}
	// The new flow editor's fork handoff, which seeds the flow and its path. A full load: on a
	// client-side one the editor reads the current page's URL hash as the flow to seed instead.
	try {
		localStorage.setItem('fork_flow', JSON.stringify({ flow, path: opts.path }))
		await opts.beforeOpen?.()
	} catch (err) {
		localStorage.removeItem('fork_flow')
		sendUserToast(`Could not open the flow editor: ${err}`, true)
		return
	}
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
