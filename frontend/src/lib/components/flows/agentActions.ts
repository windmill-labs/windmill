import {
	Copy,
	Eye,
	FileUp,
	History,
	LayoutDashboard,
	List,
	Trash,
	UserPlus
} from 'lucide-svelte'
import { DraftService, ResourceService, type AiAgent, type OpenFlow } from '$lib/gen'
import { mintDraftPath } from '$lib/mintDraftPath'
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
	workspace: string
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
	const flow: OpenFlow & { path: string; description: string; extra_perms: {} } = {
		path: opts.path,
		summary: '',
		description: '',
		extra_perms: {},
		value: {
			...(keepsManagedMemory(opts.config.memory) ? { chat_input_enabled: true } : {}),
			modules: [{ id, value }]
		},
		schema: AGENT_CHAT_SCHEMA
	}
	// Saved as a draft before anything is let go, so the flow outlives a refresh of its editor: a
	// new flow's editor writes nothing until its first edit. The editor opens it as it opens any
	// draft-only flow, in the workspace named here rather than the one navigation last stored.
	const draftPath = mintDraftPath()
	try {
		await DraftService.updateDraft({
			workspace: opts.workspace,
			kind: 'flow',
			path: draftPath,
			requestBody: { value: flow }
		})
		await opts.beforeOpen?.()
	} catch (err) {
		sendUserToast(`Could not open the flow editor: ${err}`, true)
		return
	}
	await goto(`/flows/edit/${draftPath}?workspace=${encodeURIComponent(opts.workspace)}`)
}

/** An agent's menu, as its home row and its page both show it. */
export function agentMenuItems(agent: {
	path: string
	canWrite: boolean
	/** Never deployed: no resource yet to hold permissions or to deploy onward. */
	draftOnly?: boolean
	wsSpecific?: boolean
	/** Hides what a role that cannot code has no page or right for, as on a flow's menu. */
	noCode?: boolean
	/** Undefined until loaded, when nothing is offered for deploy. */
	deployUiSettings: WorkspaceDeployUISettings | undefined
	onShare: () => void
	onDeploy: () => void
	onHistory: () => void
	onDelete: (event?: MouseEvent) => void
	/** Set where the user may build an app over the agent: a chat app when it keeps a chat, a form
	 *  over its message otherwise. */
	buildApp?: { chat: boolean; action: () => void }
}): Item[] {
	const deployable =
		!agent.noCode &&
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
					...(agent.noCode
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
		...(agent.buildApp
			? [
					{
						displayName: agent.buildApp.chat ? 'Build chat app' : 'Build app',
						icon: LayoutDashboard,
						action: agent.buildApp.action
					}
				]
			: []),
		{
			displayName: 'Share',
			icon: UserPlus,
			disabled: !agent.canWrite || Boolean(agent.draftOnly),
			action: agent.onShare
		},
		...(deployable
			? [{ displayName: 'Deploy to prod/staging', icon: FileUp, action: agent.onDeploy }]
			: []),
		...(agent.draftOnly || agent.noCode
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
