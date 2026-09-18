<script lang="ts">
	// What the session's agent is carrying, shown while the session has no messages.
	// Everything here is read from the same places the system prompt is built from.
	import { workspaceStore } from '$lib/stores'
	import { ResourceService } from '$lib/gen'
	import { getCustomPromptParts } from '$lib/aiStore'
	import { sendUserToast } from '$lib/toast'
	import { isMcpEnabled, setMcpEnabled } from '$lib/components/mcp/enabledServers'
	import {
		isOwnOrSharedMcpPath,
		MCP_LIST_PER_PAGE,
		mcpViewer
	} from '$lib/components/mcp/ownServers'
	import { AIMode } from '../AIChatManager.svelte'
	import { getAiChatManager } from '../aiChatManagerContext'
	import { summarizeTools } from '../agentContext'
	import { isSkillEnabled, setSkillEnabled } from '../skills/enabledSkills'
	import { listSkillResources } from '../skills/skillResources'
	import ContextSummaryRows from './ContextSummaryRows.svelte'
	import type {
		AgentContext,
		ContextMcpServer,
		ContextSectionId,
		ContextSkill,
		ContextSummaryActions
	} from './contextSummary'

	let { onManage }: { onManage: (section: ContextSectionId) => void } = $props()

	const aiChatManager = getAiChatManager()

	// The workspace every list here is read under — a session operates on its own
	// (possibly forked) workspace without switching `workspaceStore`.
	let ws = $derived(aiChatManager.operatingWorkspace ?? $workspaceStore ?? '')

	// A session whose fork is still staged has no workspace of its own yet, so `ws` is
	// the PARENT: a switch flipped now would be stored under it and stop applying the
	// moment the first send commits the fork.
	let toggleBlockedReason = $derived(
		aiChatManager.sessionContextResolver?.()?.pendingForkOf
			? 'This session creates its workspace on the first message. Send one before changing this.'
			: undefined
	)

	// The full lists, disabled entries included — `aiChatManager` keeps only what is on,
	// and a switch needs the rows it can turn back on. One promise per workspace, so a
	// workspace change replaces it rather than racing it.
	let listsPromise = $derived(loadLists(ws))
	// Flips applied since the lists were read, so a switch lands without a reload. One map
	// per kind: a skill and a server can hold the same resource path.
	let skillOverrides = $state<Record<string, boolean>>({})
	let mcpOverrides = $state<Record<string, boolean>>({})

	async function loadLists(
		workspace: string
	): Promise<{ skills: ContextSkill[]; mcpServers: ContextMcpServer[] }> {
		if (!workspace) return { skills: [], mcpServers: [] }
		const [skills, mcpServers] = await Promise.all([
			loadSkills(workspace),
			loadMcpServers(workspace)
		])
		return { skills, mcpServers }
	}

	async function loadSkills(workspace: string): Promise<ContextSkill[]> {
		try {
			return (await listSkillResources(workspace)).skills.map(({ path, name, description }) => ({
				path,
				name,
				description: description ?? '',
				enabled: isSkillEnabled(workspace, path)
			}))
		} catch (e) {
			console.error('Failed to load AI skills', e)
			return []
		}
	}

	async function loadMcpServers(workspace: string): Promise<ContextMcpServer[]> {
		try {
			const [resources, viewer] = await Promise.all([
				ResourceService.listResource({
					workspace,
					resourceType: 'mcp',
					perPage: MCP_LIST_PER_PAGE
				}),
				mcpViewer(workspace)
			])
			return resources
				.filter((r) => isOwnOrSharedMcpPath(r.path, r.extra_perms, viewer))
				.map((r) => ({
					path: r.path,
					name: r.path,
					description: r.description,
					enabled: isMcpEnabled(workspace, r.path)
				}))
		} catch (e) {
			console.error('Failed to load MCP connections', e)
			return []
		}
	}

	function applyOverrides<T extends { path: string; enabled: boolean }>(
		rows: T[],
		overrides: Record<string, boolean>
	): T[] {
		return rows.map((row) => ({ ...row, enabled: overrides[row.path] ?? row.enabled }))
	}

	function buildContext(lists: {
		skills: ContextSkill[]
		mcpServers: ContextMcpServer[]
	}): AgentContext {
		return {
			workspace: ws,
			// Exactly what the chat loop sends: plan mode's transition tool is registered
			// alongside `tools` there, so listing only `tools` would under-report.
			tools: summarizeTools([...aiChatManager.tools, ...aiChatManager.planMode.tools]).map(
				({ name, description, readOnly }) => ({ name, description, readOnly })
			),
			skills: applyOverrides(lists.skills, skillOverrides),
			mcpServers: applyOverrides(lists.mcpServers, mcpOverrides),
			instructions: getCustomPromptParts(AIMode.GLOBAL)
		}
	}

	const actions: ContextSummaryActions = {
		onToggleSkill: async (path, enabled) => {
			if (!setSkillEnabled(ws, path, enabled)) {
				sendUserToast('Could not save this choice for this account.', true)
				return
			}
			skillOverrides[path] = enabled
			// The prompt lists exactly the skills that are on, so it has to be rebuilt
			// before the next message rather than on the next mode change.
			await aiChatManager.refreshGlobalSkills(ws)
		},
		onToggleMcp: async (path, enabled) => {
			const target = ws
			if (!setMcpEnabled(target, path, enabled)) {
				sendUserToast('Could not save the selection for this account.', true)
				return
			}
			mcpOverrides[path] = enabled
			if (target !== ws) return
			await aiChatManager.refreshMcpServers(target)
		},
		onManage
	}
</script>

{#await listsPromise then lists}
	<ContextSummaryRows ctx={buildContext(lists)} {actions} {toggleBlockedReason} />
{/await}
