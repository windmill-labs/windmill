<!--
@component
Everything the chat carries into a turn beyond the conversation itself: its tools, the
skills and MCP servers turned on for it, and the custom instructions in force. Each of
those outlives the moment it was set — a skill turned on last week and an instruction
written once both go on steering every answer — so this is the one place they are all
accounted for and changed.

The trigger sits in the composer next to the model pill; `open(section)` is for the
callers that already know which section they mean, such as the "+" menu's Manage entries.
-->
<script lang="ts" module>
	export type AssistantSettingsSection = 'tools' | 'skills' | 'instructions' | 'mcp' | 'files'
</script>

<script lang="ts">
	import { BookOpen, Boxes, Paperclip, Plug, ScrollText, SlidersHorizontal } from 'lucide-svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import Modal2 from '$lib/components/common/modal/Modal2.svelte'
	import SidebarNavigation from '$lib/components/common/sidebar/SidebarNavigation.svelte'
	import Tooltip from '$lib/components/meltComponents/Tooltip.svelte'
	import { workspaceStore } from '$lib/stores'
	import { logFeatureUsage } from '$lib/utils/featureUsage'
	import { getAiChatManager } from './aiChatManagerContext'
	import { summarizeTools } from './agentContext'
	import AssistantToolsSection from './AssistantToolsSection.svelte'
	import AssistantSkillsSection from './AssistantSkillsSection.svelte'
	import AssistantInstructionsSection from './AssistantInstructionsSection.svelte'
	import AssistantMcpSection from './AssistantMcpSection.svelte'
	import AssistantFilesSection from './AssistantFilesSection.svelte'

	const aiChatManager = getAiChatManager()

	let isOpen = $state(false)
	let section = $state<AssistantSettingsSection>('tools')
	let skillCount = $state(0)
	let mcpCount = $state(0)
	let fileCount = $state(0)
	// A section is mid-something the modal must not close under: a confirmation or a
	// popover portaled to the body (where a click inside reads as a click outside this
	// modal), or an editor holding text nobody has saved yet. While one is up, Escape
	// and an outside click belong to the section, which answers them itself.
	let toolsBusy = $state(false)
	let skillsBusy = $state(false)
	let instructionsBusy = $state(false)
	let mcpBusy = $state(false)
	let filesBusy = $state(false)
	let blocksClose = $derived(toolsBusy || skillsBusy || instructionsBusy || mcpBusy || filesBusy)
	// Which section is holding the modal open. The section on screen wins whenever it is
	// itself blocking: it is the one that will answer the key, and preferring any other
	// would take Escape away from the surface the user is looking at. Otherwise it is
	// whichever section blocks from behind — in practice Instructions holding unsaved
	// text, since the page-based sections park themselves when navigated away from.
	let busyBySection = $derived<Record<AssistantSettingsSection, boolean>>({
		tools: toolsBusy,
		skills: skillsBusy,
		instructions: instructionsBusy,
		mcp: mcpBusy,
		files: filesBusy
	})
	let blockingSection = $derived<AssistantSettingsSection | undefined>(
		busyBySection[section]
			? section
			: (Object.keys(busyBySection) as AssistantSettingsSection[]).find((k) => busyBySection[k])
	)

	/** Escape with something blocking behind another section would look like a dead key:
	 * `Modal2` ignores it, and the section that answers it is not on screen. Show that
	 * section instead, so the reason the modal will not close is in front of the user.
	 *
	 * `stopImmediatePropagation` because this listener is the parent's and runs before
	 * the sections': showing a section makes it `active` within this same dispatch, and
	 * it would then answer the key it was revealed by — reverting the very draft the
	 * user was brought here to see. */
	function onKeydown(event: KeyboardEvent) {
		if (!isOpen || event.key !== 'Escape') return
		const blocker = blockingSection
		if (!blocker || blocker === section) return
		event.preventDefault()
		event.stopImmediatePropagation()
		select(blocker)
	}

	// A session chat operates on its own (possibly forked) workspace without switching
	// `workspaceStore`, and that is the workspace every list here is read under.
	//
	// `operatingWorkspace` is a plain getter over untracked state, so the store is read
	// unconditionally rather than behind `??`: short-circuiting it would leave this
	// derived with no dependency at all, frozen on the workspace it first saw.
	let ws = $derived.by(() => {
		const active = $workspaceStore
		return aiChatManager.operatingWorkspace ?? active ?? ''
	})

	// Exactly what the chat loop sends — plan mode's transition tool is registered
	// alongside `tools` there, so listing only `tools` would under-report.
	let tools = $derived(summarizeTools([...aiChatManager.tools, ...aiChatManager.planMode.tools]))

	// `SidebarNavigation`'s item shape, which is what the instance and workspace settings
	// navs are built from too.
	let sections = $derived([
		{ id: 'tools', label: 'Tools', icon: Boxes, count: tools.length },
		{ id: 'skills', label: 'Skills', icon: BookOpen, count: skillCount },
		{ id: 'instructions', label: 'Instructions', icon: ScrollText },
		{ id: 'mcp', label: 'MCP connections', icon: Plug, count: mcpCount },
		// The count is what is readable rather than what is attached: a locked folder under
		// a heading about what the assistant can use would say the opposite of the truth.
		{ id: 'files', label: 'Files & folders', icon: Paperclip, count: fileCount }
	])

	/** Opens on `target`, or back on whichever section was last read. */
	export function open(target: AssistantSettingsSection = section) {
		section = target
		isOpen = true
		refresh()
		record('open')
		record(target)
	}

	// `context_panel` is the kind registered in `feature_usage_ee.rs`; an unregistered
	// pair is dropped silently, so the name is fixed there rather than here.
	//
	// The key vocabulary is the closed set in this signature and nothing else — a
	// skill path or server path here would be workspace-authored text.
	function record(key: 'open' | AssistantSettingsSection) {
		logFeatureUsage('ai_session', 'context_panel', { key, workspace: ws })
	}

	function select(target: AssistantSettingsSection) {
		if (target === section) return
		section = target
		record(target)
	}

	function refresh() {
		// `globalSkills` and `mcpServers` are per-manager snapshots, and the enabled
		// sets they derive from are shared: toggling a skill in one session leaves
		// every other warm session's copy behind until its next send refreshes it.
		// Opening this modal is the one moment the tool list has to be true, so it
		// refreshes the same way the send path does — the active chat only.
		//
		// Never while a turn is in flight, though. These refreshes carry generation
		// counters, so starting one invalidates the send's own: its `Promise.all`
		// would return without applying results, and this one would then rewrite the
		// tools and system prompt underneath a running turn. Mid-turn it shows the
		// values the turn was actually given, which is the honest answer anyway.
		if (!aiChatManager.loading && !aiChatManager.sendInFlight) {
			void aiChatManager.refreshGlobalSkills()
			void aiChatManager.refreshMcpServers()
		}
	}
</script>

<svelte:window onkeydown={onKeydown} />

<Tooltip small placement="top">
	<Button
		unifiedSize="2xs"
		variant="subtle"
		iconOnly
		startIcon={{ icon: SlidersHorizontal }}
		aria-label="Assistant settings"
		onClick={() => open()}
	/>
	{#snippet text()}
		<div class="max-w-64 text-xs">
			<p class="font-semibold">Assistant settings</p>
			<p class="mt-1">The tools, skills, instructions and MCP servers this chat can use.</p>
		</div>
	{/snippet}
</Tooltip>

<Modal2
	bind:isOpen
	title="Assistant settings"
	fixedWidth="md"
	fixedHeight="lg"
	closeOnOutsideClick={!blocksClose}
	closeOnEscape={!blocksClose}
>
	{#snippet headerLeft()}
		<p class="pl-3 pt-1 text-xs text-secondary truncate">
			What the assistant can see and use in this session.
		</p>
	{/snippet}

	<div class="w-full flex min-h-0 gap-4">
		<div class="w-52 shrink-0 flex flex-col border-r border-border-light pr-3">
			<SidebarNavigation
				groups={[{ items: sections }]}
				selectedId={section}
				onNavigate={(id) => select(id as AssistantSettingsSection)}
			/>
		</div>

		<div class="grow min-w-0 flex flex-col min-h-0">
			<!-- Every section stays mounted while the modal is open: the sidebar badges
			     count what each one loaded, so hiding is display-only. Tools, Skills and MCP
			     own their own scrolling — each is a list and a detail page laid over each
			     other — so only Instructions scrolls here. -->
			<div class="{section === 'tools' ? 'flex' : 'hidden'} grow min-h-0 flex-col overflow-hidden">
				<AssistantToolsSection {tools} active={section === 'tools'} bind:blocksClose={toolsBusy} />
			</div>
			<!-- Skills owns its own scrolling: its list and its editor are PagedContent pages
			     laid over each other, and each keeps a scroll position of its own. -->
			<div class="{section === 'skills' ? 'flex' : 'hidden'} grow min-h-0 flex-col overflow-hidden">
				<AssistantSkillsSection
					{ws}
					active={section === 'skills'}
					bind:count={skillCount}
					bind:blocksClose={skillsBusy}
				/>
			</div>
			<div
				class="{section === 'instructions' ? 'block' : 'hidden'} grow min-h-0 overflow-y-auto pr-2"
			>
				<AssistantInstructionsSection
					{ws}
					active={section === 'instructions'}
					bind:blocksClose={instructionsBusy}
				/>
			</div>
			<!-- Like Skills, MCP owns its own scrolling: its list and its connect form are
			     PagedContent pages laid over each other. -->
			<div class="{section === 'mcp' ? 'flex' : 'hidden'} grow min-h-0 flex-col overflow-hidden">
				<AssistantMcpSection
					{ws}
					active={section === 'mcp'}
					bind:count={mcpCount}
					bind:blocksClose={mcpBusy}
				/>
			</div>
			<div class="{section === 'files' ? 'block' : 'hidden'} grow min-h-0 overflow-y-auto pr-2">
				<AssistantFilesSection bind:count={fileCount} bind:blocksClose={filesBusy} />
			</div>
		</div>
	</div>
</Modal2>
