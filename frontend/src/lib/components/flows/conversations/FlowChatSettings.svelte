<script lang="ts">
	import { ChevronDown, Settings2, SlidersHorizontal } from 'lucide-svelte'
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import { type DynamicInput } from '$lib/utils'
	import AgentChatInputSubmenu from './AgentChatInputSubmenu.svelte'
	import ChatModelPicker from './ChatModelPicker.svelte'
	import ReasoningEffortSlider from '$lib/components/copilot/ReasoningEffortSlider.svelte'
	import MenuItemWrapper from '$lib/components/meltComponents/MenuItemWrapper.svelte'
	import { getReasoningCapability, explicitOffToken } from '$lib/components/copilot/reasoningRegistry'
	import type { AIProvider } from '$lib/gen'
	import { type AgentChatInput, type AgentModel } from './agentChatInputs'

	interface Props {
		/** Wired agent fields, one submenu each. */
		inputs: AgentChatInput[]
		values: Record<string, any>
		onChange: (name: string, value: any) => void
		/** The model the flow fixes, when it fixes one. Named on the trigger, not editable. */
		staticModel?: AgentModel
		/** Opens the Configure-inputs modal, when the flow has inputs no agent field reads. */
		onOpenInputs?: () => void
		inputsMissingRequired?: boolean
		workspace?: string
		helperScript?: DynamicInput.HelperScript
	}

	let {
		inputs,
		values,
		onChange,
		staticModel,
		onOpenInputs,
		inputsMissingRequired = false,
		workspace,
		helperScript
	}: Props = $props()

	let menuOpen = $state(false)

	// The chosen model where the flow exposes one, else the model it fixes. With neither —
	// several agents, or a model computed per run — there is no single one to name, so the
	// trigger says what it is instead: settings.
	const modelInput = $derived(inputs.find((input) => input.key === 'provider'))
	// Everything but the model gets a submenu; the model and its thinking sit in this
	// panel together, the way the copilot's own settings menu lays them out.
	const submenuInputs = $derived(inputs.filter((input) => input.key !== 'provider'))
	const model = $derived.by((): AgentModel | undefined => {
		const chosen = modelInput ? values[modelInput.name] : undefined
		return typeof chosen?.model === 'string' ? chosen : staticModel
	})

	// Thinking sits in this menu rather than inside the model editor, matching where the
	// copilot's own chat puts it. Editable only where a flow input feeds the provider —
	// a model the flow fixes has nothing here for the composer to write.
	const capability = $derived(
		model?.kind && model?.model
			? getReasoningCapability(model.kind as AIProvider, model.model)
			: { supported: false, levels: [] as string[], canDisable: false }
	)
	const offToken = $derived(
		model?.kind && model?.model
			? explicitOffToken(model.kind as AIProvider, model.model)
			: undefined
	)
	const effortStops = $derived([
		...(capability.canDisable && offToken !== undefined ? [offToken] : []),
		...capability.levels
	])
	const currentEffort = $derived(model?.reasoning_effort ?? effortStops[0] ?? '')
	function selectEffort(stop: string) {
		if (!modelInput) return
		onChange(modelInput.name, { ...values[modelInput.name], reasoning_effort: stop })
	}
	let effortSlider: ReasoningEffortSlider | undefined = $state(undefined)

	// The trigger resizes when a value changes while the menu is open, which would shift a
	// bottom-end popover anchored to its right edge. Freeze the width for as long as it is open.
	let triggerEl: HTMLElement | undefined = $state(undefined)
	let lockedWidth = $state<number | undefined>(undefined)
	$effect(() => {
		if (menuOpen) {
			if (lockedWidth === undefined && triggerEl) {
				lockedWidth = triggerEl.getBoundingClientRect().width
			}
		} else {
			lockedWidth = undefined
		}
	})
</script>

<DropdownV2
	customMenu
	placement="bottom-end"
	fixedHeight={false}
	closeOnItemClick={false}
	bind:open={menuOpen}
>
	{#snippet buttonReplacement()}
		<div
			bind:this={triggerEl}
			class="relative"
			style={lockedWidth !== undefined ? `width: ${lockedWidth}px` : undefined}
		>
			<!-- With a single model the trigger reads exactly as the copilot's does:
			     the model, then its reasoning effort. Otherwise there is no one model to
			     name and it falls back to what it opens. -->
			<Button
				nonCaptureEvent
				unifiedSize="2xs"
				variant="subtle"
				startIcon={model ? undefined : { icon: Settings2 }}
				endIcon={{ icon: ChevronDown }}
				btnClasses="w-full max-w-[200px] text-secondary font-normal"
				title={model ? 'Model & agent settings' : 'Agent settings'}
			>
				{#if model}
					<span class="flex items-center gap-1 min-w-0">
						<span class="truncate">{model.model}</span>
						{#if model.reasoning_effort}
							<span class="shrink-0 text-tertiary">· {model.reasoning_effort}</span>
						{/if}
					</span>
				{:else}
					<span class="truncate">Settings</span>
				{/if}
			</Button>
			{#if inputsMissingRequired}
				<span class="absolute -top-0.5 -right-0.5 w-2 h-2 bg-yellow-500 rounded-full"></span>
			{/if}
		</div>
	{/snippet}
	{#snippet menu({ item, builders })}
		<div
			class="bg-surface-tertiary dark:border w-72 origin-top-right rounded-lg shadow-lg focus:outline-none py-1 text-xs"
		>
			{#each submenuInputs as input (input.name)}
				<AgentChatInputSubmenu
					{input}
					value={values[input.name]}
					onChange={(value) => onChange(input.name, value)}
					{builders}
					meltItem={item}
					{workspace}
					{helperScript}
				/>
			{/each}
			{#if modelInput}
				<div class="my-1 border-t border-border-light"></div>
				<div class="px-3 pt-1.5 pb-1 text-2xs uppercase tracking-wide text-secondary">Model</div>
				<div class="px-3 pb-2">
					<ChatModelPicker
						value={values[modelInput.name]}
						onChange={(v) => onChange(modelInput.name, v)}
						{workspace}
					/>
				</div>
			{/if}
			{#if modelInput && model?.model}
				<div class="my-1 border-t border-border-light"></div>
				{#if capability.supported && effortStops.length > 1}
					<MenuItemWrapper {item} onKeydown={(e) => effortSlider?.adjust(e)} class="block group">
						<ReasoningEffortSlider
							bind:this={effortSlider}
							stops={effortStops}
							current={currentEffort}
							onSelect={selectEffort}
							format={(stop) => (stop === offToken ? 'off' : stop)}
						/>
					</MenuItemWrapper>
				{:else}
					<ReasoningEffortSlider
						stops={[]}
						current=""
						onSelect={() => {}}
						unsupportedReason="Not supported by this model"
					/>
				{/if}
			{/if}
			<!-- A model the flow fixes is shown but not offered: no flow input feeds it, so
			     changing it here would mean editing the flow. -->
			{#if staticModel && !modelInput}
				<div class="my-1 border-t border-border-light"></div>
				<div class="px-3 pt-1.5 pb-1 text-2xs uppercase tracking-wide text-secondary">Model</div>
				<div class="px-3 pb-1 text-primary truncate">{staticModel.model}</div>
				<div class="px-3 pb-1.5 text-2xs text-tertiary">Set in the flow</div>
			{/if}
			{#if onOpenInputs}
				<div class="my-1 border-t border-border-light"></div>
				<button
					class="px-4 py-2 text-primary font-normal hover:bg-surface-hover cursor-pointer text-xs transition-colors w-full flex flex-row gap-2 items-center rounded-sm"
					onclick={() => {
						menuOpen = false
						onOpenInputs?.()
					}}
				>
					<SlidersHorizontal size={14} class="shrink-0" />
					<p class="truncate grow min-w-0 text-left">Other inputs</p>
					{#if inputsMissingRequired}
						<span class="w-2 h-2 bg-yellow-500 rounded-full shrink-0"></span>
					{/if}
				</button>
			{/if}
		</div>
	{/snippet}
</DropdownV2>
