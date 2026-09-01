<!--
@component
Read-only inventory of what the chat carries into every turn: its tools, the
skills and MCP servers the user turned on, the custom instructions in force, and
the files and folders linked to the session.

Each of those is configured elsewhere — the `+` menu, the skills and MCP drawers,
the prompts modal, the attachment bar — and each outlives the moment it was set:
a skill turned on last week and an instruction written once both go on steering
every answer. This is where a user accounts for them: a glance on the trigger, a
breakdown per section on expand, and a shortcut back to the manager that owns
each one. It reports, and never changes what the chat carries.
-->
<script lang="ts">
	import { BookOpen, Boxes, FileText, Folder, Plug, ScrollText, Settings2 } from 'lucide-svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import Tooltip from '$lib/components/meltComponents/Tooltip.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import { copilotInfo, getCustomPromptParts } from '$lib/aiStore'
	import { logFeatureUsage } from '$lib/utils/featureUsage'
	import { getAiChatManager } from './aiChatManagerContext'
	import { AIMode } from './AIChatManager.svelte'
	import {
		attachmentStatusLabel,
		attachmentValue,
		contextGlanceLine,
		countReadyAttachments,
		filterTools,
		summarizeTools
	} from './agentContext'
	import ChatCollapsibleCard from './ChatCollapsibleCard.svelte'
	import type { AttachedFileStatus } from './files/attachedFiles.svelte'

	let {
		onManageSkills,
		onManageMcp
	}: {
		onManageSkills: () => void
		onManageMcp: () => void
	} = $props()

	const aiChatManager = getAiChatManager()

	type SectionKey = 'tools' | 'skills' | 'mcp' | 'instructions' | 'files'

	// One section at a time: the popover is a glance surface, and two long lists
	// open at once turn it into a page.
	let expanded = $state<SectionKey | undefined>(undefined)
	let toolFilter = $state('')

	// Exactly what the chat loop sends — plan mode's transition tool is registered
	// alongside `tools` there, so listing only `tools` would under-report.
	let tools = $derived(summarizeTools([...aiChatManager.tools, ...aiChatManager.planMode.tools]))
	let shownTools = $derived(filterTools(tools, toolFilter))
	// Already the enabled-and-still-readable sets, and the same lists the system
	// prompt carries — so the panel cannot claim more than the model was told.
	let skills = $derived(aiChatManager.globalSkills)
	let servers = $derived(aiChatManager.mcpServers)
	let folders = $derived(aiChatManager.attachedFiles.folders)
	// Files dropped on a past message are counted and listed alongside the session's
	// own: the attachment bar hides them because their chip lives on that message,
	// but the file tools go on reading them, which is what this panel reports.
	let attachedFiles = $derived([
		...aiChatManager.attachedFiles.standalone,
		...aiChatManager.attachedFiles.messageAttached
	])
	// The count is what is usable; the total says what is merely attached.
	let readyAttachments = $derived(countReadyAttachments(folders, attachedFiles))
	let attachmentCount = $derived(folders.length + attachedFiles.length)
	let attachmentLabel = $derived(attachmentValue(readyAttachments, attachmentCount))

	// The three bare reads below are this derived's dependencies, since the getter
	// takes none of them. Between them they keep the panel at least as current as
	// the prompt it reports on: `copilotInfo` backs the workspace half and is the
	// same store the system message is built from; `systemMessage` is reassigned
	// whenever the manager re-reads the prompts (what `update_user_instructions`
	// does), which is the only signal the localStorage-backed user half has; and
	// `promptsSeq` re-reads it on open for the writes that go through neither.
	let promptsSeq = $state(0)
	let instructions = $derived.by(() => {
		promptsSeq
		$copilotInfo
		aiChatManager.systemMessage
		return getCustomPromptParts(AIMode.GLOBAL)
	})
	let instructionSources = $derived(
		[
			instructions.workspace ? 'Workspace' : undefined,
			instructions.user ? 'You' : undefined
		].filter((s): s is string => s !== undefined)
	)

	let glance = $derived(
		contextGlanceLine({
			tools: tools.length,
			skills: skills.length,
			mcpServers: servers.length,
			attachments: readyAttachments,
			instructions: instructionSources.length > 0
		})
	)

	// The key vocabulary is the closed set in this signature and nothing else — a
	// skill path or server path here would be workspace-authored text.
	function record(key: 'open' | SectionKey) {
		logFeatureUsage('ai_session', 'context_panel', {
			key,
			workspace: aiChatManager.operatingWorkspace
		})
	}

	function onOpenChange(open: boolean) {
		if (!open) return
		promptsSeq++
		// Refresh the per-manager skill/MCP snapshots, which a toggle in another
		// session leaves behind — but never mid-turn: these carry generation counters,
		// so one started here invalidates the send's own, whose `Promise.all` then
		// returns without applying results and leaves this one rewriting a live turn.
		if (!aiChatManager.loading && !aiChatManager.sendInFlight) {
			void aiChatManager.refreshGlobalSkills()
			void aiChatManager.refreshMcpServers()
		}
		record('open')
	}

	function toggle(key: SectionKey) {
		if (expanded === key) {
			expanded = undefined
			return
		}
		expanded = key
		record(key)
	}

	function manage(action: () => void, close: () => void) {
		close()
		action()
	}
</script>

{#snippet attachmentStatus(status: AttachedFileStatus)}
	{@const label = attachmentStatusLabel(status)}
	{#if label}
		<span class="shrink-0 text-2xs text-hint">{label}</span>
	{/if}
{/snippet}

{#snippet section(p: {
	key: SectionKey
	/** A lucide icon component; typed loosely the way `Item.icon` is. */
	icon: any
	label: string
	value: string
	empty: boolean
	manageTitle?: string
	onManage?: () => void
	children?: import('svelte').Snippet
})}
	<ChatCollapsibleCard
		label={p.label}
		icon={p.icon}
		expanded={expanded === p.key}
		onToggle={() => toggle(p.key)}
		toggleable={!p.empty}
		class="font-sans px-1.5"
		headerClass="grow min-w-0 px-1.5 gap-1.5"
		labelClass="text-xs text-primary"
		contentClass="mt-0.5 mb-1 px-2 py-1.5 border-0 bg-surface-secondary max-h-56 overflow-y-auto"
	>
		{#snippet headerRight()}
			<div class="flex items-center gap-1 shrink-0">
				<span class="text-2xs tabular-nums {p.empty ? 'text-hint' : 'text-secondary'}">
					{p.value}
				</span>
				{#if p.onManage}
					<Button
						unifiedSize="2xs"
						variant="subtle"
						iconOnly
						startIcon={{ icon: Settings2 }}
						title={p.manageTitle}
						onClick={p.onManage}
					/>
				{/if}
			</div>
		{/snippet}
		{@render p.children?.()}
	</ChatCollapsibleCard>
{/snippet}

<Popover
	placement="top-end"
	contentClasses="w-80 max-w-[calc(100vw-2rem)]"
	triggerAttrs={{ 'aria-label': 'What this chat can use' }}
	on:openChange={(e) => onOpenChange(e.detail)}
>
	{#snippet trigger()}
		<Tooltip small placement="top">
			<!-- Popover renders the real trigger button around this one and carries the
			     label; this is here for the design-system styling alone, so it is taken
			     out of the tab order and the accessibility tree. -->
			<Button
				nonCaptureEvent
				unifiedSize="2xs"
				variant="default"
				iconOnly
				startIcon={{ icon: Boxes }}
				tabindex={-1}
				aria-hidden="true"
			/>
			{#snippet text()}
				<div class="max-w-64 text-xs">
					<p class="font-semibold">What this chat can use</p>
					<p class="mt-1">{glance}</p>
				</div>
			{/snippet}
		</Tooltip>
	{/snippet}

	{#snippet content({ close })}
		<div class="px-3 pt-2.5 pb-2 border-b border-border-light">
			<p class="text-xs font-semibold text-emphasis">Available to the assistant</p>
			<p class="mt-0.5 text-2xs text-secondary">{glance}</p>
		</div>
		<div class="py-1.5 flex flex-col">
			{#snippet toolList()}
				{#if tools.length > 12}
					<div class="pb-1.5">
						<TextInput
							bind:value={toolFilter}
							size="2xs"
							inputProps={{ placeholder: 'Filter tools' }}
						/>
					</div>
				{/if}
				{#each shownTools as tool (tool.name)}
					<div class="py-1 border-b border-border-light last:border-b-0">
						<div class="font-mono text-2xs text-emphasis break-all">{tool.name}</div>
						{#if tool.description}
							<div class="mt-0.5 text-2xs text-secondary line-clamp-2">{tool.description}</div>
						{/if}
					</div>
				{/each}
				{#if shownTools.length === 0}
					<div class="py-1 text-2xs text-hint">No tool matches this filter.</div>
				{/if}
			{/snippet}
			{@render section({
				key: 'tools',
				icon: Boxes,
				label: 'Tools',
				value: `${tools.length}`,
				empty: tools.length === 0,
				children: toolList
			})}

			{#snippet skillList()}
				{#each skills as skill (skill.path)}
					<div class="py-1 border-b border-border-light last:border-b-0">
						<div class="text-2xs font-semibold text-emphasis truncate">{skill.name}</div>
						<div class="font-mono text-2xs text-tertiary truncate">{skill.path}</div>
						{#if skill.description}
							<div class="mt-0.5 text-2xs text-secondary line-clamp-2">{skill.description}</div>
						{/if}
					</div>
				{/each}
			{/snippet}
			{@render section({
				key: 'skills',
				icon: BookOpen,
				label: 'Skills',
				value: skills.length > 0 ? `${skills.length}` : 'None',
				empty: skills.length === 0,
				manageTitle: 'Manage skills',
				onManage: () => manage(onManageSkills, close),
				children: skillList
			})}

			{#snippet serverList()}
				{#each servers as server (server.path)}
					<div class="py-1 font-mono text-2xs text-emphasis truncate">{server.path}</div>
				{/each}
				<p class="pt-1 text-2xs text-secondary">
					Their tools are discovered on demand and run with your own credentials.
				</p>
			{/snippet}
			{@render section({
				key: 'mcp',
				icon: Plug,
				label: 'MCP connections',
				value: servers.length > 0 ? `${servers.length}` : 'None',
				empty: servers.length === 0,
				manageTitle: 'Manage MCP connections',
				onManage: () => manage(onManageMcp, close),
				children: serverList
			})}

			{#snippet instructionBlocks()}
				{#if instructions.workspace}
					<div class="py-1 border-b border-border-light last:border-b-0">
						<div class="text-2xs font-semibold text-emphasis">Workspace</div>
						<p class="mt-0.5 text-2xs text-secondary whitespace-pre-wrap break-words">
							{instructions.workspace}
						</p>
					</div>
				{/if}
				{#if instructions.user}
					<div class="py-1">
						<div class="text-2xs font-semibold text-emphasis">You</div>
						<p class="mt-0.5 text-2xs text-secondary whitespace-pre-wrap break-words">
							{instructions.user}
						</p>
					</div>
				{/if}
			{/snippet}
			{@render section({
				key: 'instructions',
				icon: ScrollText,
				label: 'Instructions',
				value: instructionSources.length > 0 ? instructionSources.join(' · ') : 'None',
				empty: instructionSources.length === 0,
				children: instructionBlocks
			})}

			{#snippet attachmentList()}
				{#each folders as folder (folder.name)}
					<div class="py-1 flex items-center gap-1.5">
						<Folder size={12} class="shrink-0 text-tertiary" />
						<span class="grow min-w-0 text-2xs text-emphasis truncate">{folder.name}</span>
						{@render attachmentStatus(folder.status)}
						<span class="shrink-0 text-2xs text-secondary tabular-nums">
							{folder.files.length}
						</span>
					</div>
				{/each}
				{#each attachedFiles as file (file.id ?? file.name)}
					<div class="py-1 flex items-center gap-1.5">
						<FileText size={12} class="shrink-0 text-tertiary" />
						<span class="grow min-w-0 text-2xs text-emphasis truncate">{file.name}</span>
						{@render attachmentStatus(file.status)}
					</div>
				{/each}
			{/snippet}
			{@render section({
				key: 'files',
				icon: FileText,
				label: 'Files & folders',
				value: attachmentLabel,
				empty: attachmentCount === 0,
				children: attachmentList
			})}
		</div>
	{/snippet}
</Popover>
