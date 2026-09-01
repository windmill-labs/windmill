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
	export type AssistantSettingsSection = 'tools' | 'skills' | 'instructions' | 'mcp'
</script>

<script lang="ts">
	import { BookOpen, Boxes, Plug, ScrollText, SlidersHorizontal } from 'lucide-svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import Modal2 from '$lib/components/common/modal/Modal2.svelte'
	import Tooltip from '$lib/components/meltComponents/Tooltip.svelte'
	import { workspaceStore } from '$lib/stores'
	import { logFeatureUsage } from '$lib/utils/featureUsage'
	import { getAiChatManager } from './aiChatManagerContext'
	import { summarizeTools } from './agentContext'
	import AssistantToolsSection from './AssistantToolsSection.svelte'
	import AssistantSkillsSection from './AssistantSkillsSection.svelte'
	import AssistantInstructionsSection from './AssistantInstructionsSection.svelte'
	import AssistantMcpSection from './AssistantMcpSection.svelte'

	const aiChatManager = getAiChatManager()

	let isOpen = $state(false)
	let section = $state<AssistantSettingsSection>('tools')
	let skillCount = $state(0)
	let mcpCount = $state(0)
	// A section is mid-something the modal must not close under: a confirmation or a
	// popover portaled to the body (where a click inside reads as a click outside this
	// modal), or an editor holding text nobody has saved yet. While one is up, Escape
	// and an outside click belong to the section, which answers them itself.
	let skillsBusy = $state(false)
	let instructionsBusy = $state(false)
	let mcpBusy = $state(false)
	let blocksClose = $derived(skillsBusy || instructionsBusy || mcpBusy)

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

	const sections: {
		key: AssistantSettingsSection
		label: string
		/** A lucide icon component; typed loosely the way `Item.icon` is. */
		icon: any
		count?: () => number
	}[] = [
		{ key: 'tools', label: 'Tools', icon: Boxes, count: () => tools.length },
		{ key: 'skills', label: 'Skills', icon: BookOpen, count: () => skillCount },
		{ key: 'instructions', label: 'Instructions', icon: ScrollText },
		{ key: 'mcp', label: 'MCP connections', icon: Plug, count: () => mcpCount }
	]

	/** Opens on `target`, or back on whichever section was last read. */
	export function open(target: AssistantSettingsSection = section) {
		section = target
		isOpen = true
		refresh()
		record('open')
		record(target)
	}

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

<Tooltip small placement="top">
	<Button
		unifiedSize="2xs"
		variant="default"
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
		<nav class="w-52 shrink-0 flex flex-col border-r border-border-light pr-3">
			<div class="flex flex-col gap-0.5">
				{#each sections as s (s.key)}
					{@const count = s.count?.()}
					<Button
						variant="subtle"
						unifiedSize="sm"
						selected={section === s.key}
						startIcon={{ icon: s.icon }}
						btnClasses="w-full !justify-between font-normal"
						wrapperClasses="w-full"
						onClick={() => select(s.key)}
					>
						<span class="grow min-w-0 text-left truncate">{s.label}</span>
						{#if count !== undefined}
							<span class="shrink-0 text-2xs tabular-nums text-secondary">{count}</span>
						{/if}
					</Button>
				{/each}
			</div>
			<div class="mt-auto pt-3 text-2xs text-hint truncate" title={ws}>
				Acting on <span class="font-mono">{ws}</span>
			</div>
		</nav>

		<div class="grow min-w-0 flex flex-col min-h-0">
			<!-- Every section stays mounted while the modal is open: the sidebar badges
			     count what each one loaded, so hiding is display-only. The scroll lives
			     here rather than inside the sections, so each one is a plain Section. -->
			<div class="{section === 'tools' ? 'block' : 'hidden'} grow min-h-0 overflow-y-auto">
				<AssistantToolsSection {tools} />
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
			<div class="{section === 'instructions' ? 'block' : 'hidden'} grow min-h-0 overflow-y-auto">
				<AssistantInstructionsSection
					{ws}
					active={section === 'instructions'}
					bind:blocksClose={instructionsBusy}
				/>
			</div>
			<div class="{section === 'mcp' ? 'block' : 'hidden'} grow min-h-0 overflow-y-auto">
				<AssistantMcpSection {ws} bind:count={mcpCount} bind:blocksClose={mcpBusy} />
			</div>
		</div>
	</div>
</Modal2>
