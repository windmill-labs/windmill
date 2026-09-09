<script lang="ts" module>
	let refreshCount = $state({ val: 0 })
</script>

<script lang="ts">
	import { createEventDispatcher, getContext, onDestroy, onMount, untrack } from 'svelte'
	import StepGenQuick from '$lib/components/copilot/StepGenQuick.svelte'
	import FlowInputsQuick from '../content/FlowInputsQuick.svelte'
	import type { FlowBuilderWhitelabelCustomUi } from '$lib/components/custom_ui'
	import ToggleHubWorkspaceQuick from '$lib/components/ToggleHubWorkspaceQuick.svelte'
	import TopLevelNode from '../pickers/TopLevelNode.svelte'
	import RefreshButton from '$lib/components/common/button/RefreshButton.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import { ResourceService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import type { FlowEditorContext } from '../types'
	import { logReusableAgentUsage } from '../agentTelemetry'
	import { BotIcon, Loader2, Plus } from 'lucide-svelte'

	const dispatch = createEventDispatcher()
	interface Props {
		stop?: boolean
		funcDesc?: string
		disableAi?: boolean
		kind?: 'script' | 'trigger' | 'preprocessor' | 'failure'
		allowTrigger?: boolean
		toolMode?: boolean
		/** Narrow layout (450px instead of 650px). Defaults on for the preprocessor
		 *  and failure pickers; set it when the container cannot fit the wide one. */
		small?: boolean
	}

	let {
		stop = false,
		funcDesc = $bindable(''),
		disableAi = false,
		kind = 'script',
		allowTrigger = true,
		toolMode = false,
		small: smallProp = undefined
	}: Props = $props()

	let customUi: undefined | FlowBuilderWhitelabelCustomUi = getContext('customUi')
	let selectedKind:
		| 'script'
		| 'trigger'
		| 'preprocessor'
		| 'approval'
		| 'flow'
		| 'failure'
		| 'aisandbox'
		| 'aiagent' = $state(untrack(() => kind))
	let preFilter: 'all' | 'workspace' | 'hub' = $state('all')
	let loading = $state(false)
	let small = $derived(smallProp ?? (kind === 'preprocessor' || kind === 'failure'))

	// Optional: this picker also renders outside the flow editor's context (the triggers wrapper).
	const flowEditorContext = getContext<FlowEditorContext | undefined>('FlowEditorContext')
	let ws = $derived(flowEditorContext?.opWorkspace?.() ?? $workspaceStore)

	// Flagged for the whole menu, whichever pane is showing: the flow builder walks node
	// selection on ArrowUp/ArrowDown while it is unset, which tears the popover down.
	onMount(() => {
		flowEditorContext?.insertButtonOpen.set(true)
	})
	onDestroy(() => {
		flowEditorContext?.insertButtonOpen.set(false)
	})

	let savedAgents = $state<{ path: string; description?: string }[]>([])
	let savedAgentsLoading = $state(false)
	let savedAgentsWs: string | undefined = undefined
	async function loadSavedAgents() {
		if (!ws || savedAgentsWs === ws) {
			return
		}
		savedAgentsLoading = true
		try {
			const rs = await ResourceService.listResource({
				workspace: ws,
				resourceType: 'ai_agent',
				perPage: 1000
			})
			savedAgents = rs.map((r) => ({ path: r.path, description: r.description }))
			savedAgentsWs = ws
		} catch {
			savedAgents = []
		} finally {
			savedAgentsLoading = false
		}
	}
	let filteredAgents = $derived(
		funcDesc
			? savedAgents.filter((a) => a.path.toLowerCase().includes(funcDesc.toLowerCase()))
			: savedAgents
	)

	function newBlankAgent() {
		dispatch('close')
		dispatch('new', { kind: 'aiagent' })
	}
	function newLinkedAgent(agentPath: string) {
		dispatch('close')
		logReusableAgentUsage('linked')
		dispatch('new', { kind: 'aiagent', agentPath })
	}
	function newClaudeSandbox() {
		dispatch('close')
		dispatch('new', {
			kind: 'script',
			inlineScript: {
				language: 'bun',
				kind: 'script',
				subkind: 'claudesandbox'
			}
		})
	}

	// Rows of the AI panes in render order. FlowInputsQuick owns the keyboard index for the other
	// panes and is unmounted while these render, so the two never fight over Enter or the arrows.
	let aiRows = $derived.by((): (() => void)[] => {
		if (selectedKind === 'aiagent') {
			const saved = savedAgentsLoading ? [] : filteredAgents
			return [newBlankAgent, ...saved.map((a) => () => newLinkedAgent(a.path))]
		}
		if (selectedKind === 'aisandbox') {
			return [newClaudeSandbox]
		}
		return []
	})
	let selectedByKeyboard = $state(0)
	// Clamped rather than reset when the search filter shrinks the list, so the index never
	// points past the last rendered row.
	let aiSelected = $derived(Math.min(selectedByKeyboard, Math.max(aiRows.length - 1, 0)))
	let aiScrollable: HTMLElement | undefined = $state()
	let stepGen: StepGenQuick | undefined = $state()
	function onAiKeyDown(e: KeyboardEvent) {
		if (aiRows.length === 0) {
			return
		}
		// Enter activates the highlighted row only from the search box (or nothing focused); a
		// focused button, link or field keeps its own native activation.
		if (
			e.key === 'Enter' &&
			e.target instanceof HTMLElement &&
			e.target.closest('button, a, select, textarea')
		) {
			return
		}
		if (e.key === 'ArrowDown') {
			selectedByKeyboard = (aiSelected + 1) % aiRows.length
			aiScrollable?.scrollTo({ top: selectedByKeyboard * 32, behavior: 'smooth' })
			e.preventDefault()
		} else if (e.key === 'ArrowUp') {
			selectedByKeyboard = (aiSelected - 1 + aiRows.length) % aiRows.length
			aiScrollable?.scrollTo({ top: selectedByKeyboard * 32, behavior: 'smooth' })
			e.preventDefault()
		} else if (e.key === 'Enter') {
			e.preventDefault()
			aiRows[aiSelected]()
		}
	}

	let height = $state(0)
	let owners = $state([])
	// Only the content-sized host (TriggersWrapper) grows past this. The fixed-height hosts top out
	// at 464px and must stay under the threshold, or every workspace row goes two-line in the step
	// picker.
	let displayPath = $derived(height > 480)
</script>

<svelte:window onkeydown={onAiKeyDown} />
<div
	id="flow-editor-insert-module"
	class="flex flex-col h-full gap-2 max-w-full {small ? 'w-[450px]' : 'w-[650px]'}"
	onwheel={(e) => {
		e.stopPropagation()
	}}
	role="none"
	bind:clientHeight={height}
>
	<div class="flex flex-row items-center gap-2">
		<StepGenQuick
			bind:this={stepGen}
			on:escape={() => dispatch('close')}
			{disableAi}
			on:insert
			bind:funcDesc
			{preFilter}
			{loading}
		/>
		{#if selectedKind != 'preprocessor' && selectedKind != 'flow'}
			<ToggleHubWorkspaceQuick bind:selected={preFilter} />
		{/if}
		<RefreshButton
			size="sm"
			{loading}
			onClick={() => {
				refreshCount.val += 1
				if (selectedKind === 'aiagent') {
					savedAgentsWs = undefined
					loadSavedAgents()
				}
			}}
		/>
	</div>

	<div class="flex flex-row grow min-h-0 gap-2">
		{#if kind === 'script'}
			<div class="flex-none w-40 flex flex-col text-xs text-primary overflow-auto gap-1">
				<TopLevelNode
					label="Action"
					selected={selectedKind === 'script'}
					onSelect={() => {
						selectedKind = 'script'
					}}
				/>
				{#if toolMode}
					<TopLevelNode
						label="MCP"
						onSelect={() => {
							dispatch('pickMcpTool')
							dispatch('close')
						}}
					/>
					<TopLevelNode
						label="Web Search"
						onSelect={() => {
							dispatch('pickWebsearchTool')
							dispatch('close')
						}}
					/>
					<TopLevelNode
						label="AI Agent"
						onSelect={() => {
							dispatch('pickAiAgentTool')
							dispatch('close')
						}}
					/>
				{:else}
					{#if customUi?.triggers != false && allowTrigger}
						<TopLevelNode
							label="Trigger"
							selected={selectedKind === 'trigger'}
							onSelect={() => {
								selectedKind = 'trigger'
							}}
						/>
					{/if}
					<TopLevelNode
						label="Approval/Prompt"
						selected={selectedKind === 'approval'}
						onSelect={() => {
							selectedKind = 'approval'
						}}
					/>
					{#if customUi?.flowNode != false}
						<TopLevelNode
							label="Flow"
							selected={selectedKind === 'flow'}
							onSelect={() => {
								selectedKind = 'flow'
							}}
						/>
					{/if}
					{#if stop}
						<TopLevelNode
							label="End flow"
							selected={selectedKind === 'script'}
							onSelect={() => {
								selectedKind = 'script'
							}}
						/>
					{/if}

					<TopLevelNode
						label="For loop"
						onSelect={() => {
							dispatch('close')
							dispatch('new', { kind: 'forloop' })
						}}
					/>
					<TopLevelNode
						label="While loop"
						onSelect={() => {
							dispatch('close')
							dispatch('new', { kind: 'whileloop' })
						}}
					/>
					<TopLevelNode
						label="Branch to one"
						onSelect={() => {
							dispatch('close')
							dispatch('new', { kind: 'branchone' })
						}}
					/>
					<TopLevelNode
						label="Branch to all"
						onSelect={() => {
							dispatch('close')
							dispatch('new', { kind: 'branchall' })
						}}
					/>
					{#if customUi?.aiAgent != false}
						<TopLevelNode
							label="AI Agent"
							selected={selectedKind === 'aiagent'}
							onSelect={() => {
								selectedKind = 'aiagent'
								selectedByKeyboard = 0
								loadSavedAgents()
								// Clicking leaves focus on this button, where Enter would only re-select it.
								stepGen?.focus()
							}}
						/>
					{/if}
					{#if customUi?.aiSandbox != false}
						<TopLevelNode
							label="AI Sandbox"
							selected={selectedKind === 'aisandbox'}
							onSelect={() => {
								selectedKind = 'aisandbox'
								selectedByKeyboard = 0
								stepGen?.focus()
							}}
						/>
					{/if}
				{/if}
			</div>
		{/if}

		{#if selectedKind === 'aiagent'}
			<div
				bind:this={aiScrollable}
				class="h-full overflow-auto grow min-w-0 p-2 gap-1 flex flex-col"
			>
				<Button
					onClick={newBlankAgent}
					onmousemove={() => (selectedByKeyboard = 0)}
					role="menuitem"
					variant="subtle"
					unifiedSize="sm"
					btnClasses="justify-start {aiSelected === 0
						? 'bg-surface-hover'
						: 'hover:bg-transparent'}"
				>
					<Plus size={13} class="shrink-0" />
					<span class="grow truncate text-left">Blank AI agent</span>
					{#if aiSelected === 0}
						<kbd class="!text-xs">&crarr;</kbd>
					{/if}
				</Button>
				{#if savedAgentsLoading}
					<div class="flex items-center gap-2 p-2 text-xs text-tertiary">
						<Loader2 size={13} class="animate-spin" /> Loading saved agents
					</div>
				{:else if filteredAgents.length > 0}
					<div class="pt-2 pb-0 text-2xs font-normal text-secondary ml-2">Saved agents</div>
					{#each filteredAgents as agent, i (agent.path)}
						<Button
							onClick={() => newLinkedAgent(agent.path)}
							onmousemove={() => (selectedByKeyboard = i + 1)}
							role="menuitem"
							variant="subtle"
							unifiedSize="sm"
							btnClasses="justify-start {aiSelected === i + 1
								? 'bg-surface-hover'
								: 'hover:bg-transparent'}"
							title={agent.description || agent.path}
						>
							<BotIcon size={13} class="shrink-0 text-ai" />
							<span class="grow truncate text-left">{agent.path}</span>
							{#if aiSelected === i + 1}
								<kbd class="!text-xs">&crarr;</kbd>
							{/if}
						</Button>
					{/each}
				{:else}
					<div class="p-2 text-xs text-tertiary">
						{savedAgents.length > 0
							? 'No saved agent matches this search'
							: 'No saved agent in this workspace yet. Configure a blank one, then Save as reusable agent to reuse it.'}
					</div>
				{/if}
			</div>
		{:else if selectedKind === 'aisandbox'}
			<div class="h-full overflow-auto grow min-w-0 p-2 gap-1 flex flex-col">
				<TopLevelNode
					label="Claude Code"
					neutral
					returnIcon
					selected={aiSelected === 0}
					onSelect={newClaudeSandbox}
					onHover={() => (selectedByKeyboard = 0)}
				/>
			</div>
		{:else}
			<FlowInputsQuick
				{selectedKind}
				bind:loading
				filter={funcDesc}
				{disableAi}
				{funcDesc}
				{kind}
				bind:owners
				on:close={() => {
					dispatch('close')
				}}
				on:new
				on:pickScript
				on:pickFlow
				{preFilter}
				{displayPath}
				refreshCount={refreshCount.val}
			/>
		{/if}
	</div>
</div>
