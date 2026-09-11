<script lang="ts">
	/**
	 * The model button every chat puts in the bottom-right of its composer: the trigger
	 * names the model and its reasoning effort, and the menu holds the choices behind
	 * both. Driven entirely by ChatModelSettingsConfig, so the session chat and the flow
	 * chat render the same control from different data — see chatModelSettings.ts.
	 */
	import { ChevronDown, Check, Loader2 } from 'lucide-svelte'
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import DropdownSubmenuItem from '$lib/components/DropdownSubmenuItem.svelte'
	import MenuItem from '$lib/components/meltComponents/MenuItem.svelte'
	import MenuItemWrapper from '$lib/components/meltComponents/MenuItemWrapper.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import ReasoningEffortSlider from './ReasoningEffortSlider.svelte'
	import { getReasoningCapability, resolveEffectiveReasoning } from './reasoningRegistry'
	import {
		reasoningDisplay,
		type ChatModelSettingsConfig,
		type ChoiceSection
	} from './chatModelSettings'
	import type { Item } from '$lib/utils'
	import type { MenubarMenuElements, createDropdownMenu } from '@melt-ui/svelte'
	import { twMerge } from 'tailwind-merge'

	type MeltItem = MenubarMenuElements['item']
	type MeltBuilders = ReturnType<typeof createDropdownMenu>['builders']

	let { config }: { config: ChatModelSettingsConfig } = $props()

	const reasoning = $derived(config.reasoning)
	const capability = $derived(
		reasoning
			? getReasoningCapability(reasoning.provider, reasoning.model)
			: { supported: false, levels: [] as string[], canDisable: false, known: false }
	)
	// Effective effort accounts for the default-on level on capable models.
	const effective = $derived(
		reasoning
			? resolveEffectiveReasoning({
					provider: reasoning.provider,
					model: reasoning.model,
					reasoning: reasoning.value
				})
			: undefined
	)
	// The stops, the one in use and the trigger's suffix are decided together, in one
	// tested place: they have to agree, and three rounds of review found them disagreeing.
	const display = $derived(reasoningDisplay(reasoning, capability, effective))
	const stops = $derived(display.stops)
	const currentStop = $derived(display.currentStop)
	const effortLabel = $derived(display.label)

	let effortSlider: ReasoningEffortSlider | undefined = $state(undefined)

	// The trigger label resizes when the effort changes (dragging the slider while the menu
	// is open). With a `bottom-end` popover anchored to the trigger's right edge, that resize
	// would shift the popover, so freeze the trigger to its width at open time and release it
	// on close — no movement while open, natural sizing the rest of the time.
	let menuOpen = $state(false)
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

	// Blocks are separated, not prefixed: a rule belongs between two of them, so the first
	// one rendered must not draw one above itself whichever block that turns out to be.
	const BLOCK_CLASS =
		'border-border-light [&:not(:first-child)]:border-t [&:not(:first-child)]:mt-1 [&:not(:first-child)]:pt-1'

	const ROW_CLASS =
		'w-full flex items-center gap-2 px-3 py-1.5 text-left font-normal hover:bg-surface-hover data-[highlighted]:bg-surface-hover rounded-sm transition-colors cursor-pointer'
</script>

{#snippet trigger()}
	<div
		bind:this={triggerEl}
		style={lockedWidth !== undefined ? `width: ${lockedWidth}px` : undefined}
	>
		<Button
			nonCaptureEvent
			unifiedSize="2xs"
			variant="subtle"
			disabled={config.readOnly}
			endIcon={config.readOnly ? undefined : { icon: ChevronDown }}
			btnClasses="w-full max-w-[200px] text-secondary font-normal"
			title={config.readOnly ? config.readOnlyReason : config.title}
		>
			<span class="flex items-center gap-1 min-w-0">
				<span class="truncate">{config.label}</span>
				{#if effortLabel}
					<span class="shrink-0 text-tertiary">· {effortLabel}</span>
				{/if}
				{#if config.badge}
					<span
						class={twMerge(
							'shrink-0 rounded-full px-1.5 text-2xs',
							config.badge.warn
								? 'bg-yellow-100 text-yellow-600 dark:bg-yellow-900/40'
								: 'bg-surface-secondary text-tertiary'
						)}>{config.badge.text}</span
					>
				{/if}
			</span>
		</Button>
	</div>
{/snippet}

{#snippet section(sec: ChoiceSection, item: MeltItem)}
	<div class="px-3 pt-1.5 pb-1 text-2xs uppercase tracking-wide text-secondary">{sec.label}</div>
	{#if sec.loading}
		<div class="flex items-center gap-2 px-3 py-1.5 text-tertiary">
			<Loader2 size={14} class="animate-spin" /> Loading...
		</div>
	{:else if sec.options.length === 0}
		<div class="px-3 py-1.5 text-tertiary">{sec.emptyMessage ?? 'Nothing to choose from'}</div>
	{:else}
		<div class={twMerge('overflow-y-auto', sec.maxHeight ?? 'max-h-48')}>
			{#each sec.options as option (option.key)}
				<MenuItem {item} class={ROW_CLASS} onClick={() => option.onSelect()}>
					<span class="truncate grow min-w-0">{option.label}</span>
					{#if option.hint}
						<span class="shrink-0 text-tertiary truncate max-w-[70px]">{option.hint}</span>
					{/if}
					{#if option.selected}
						<Check size={14} class="shrink-0 text-primary" />
					{/if}
				</MenuItem>
			{/each}
		</div>
	{/if}
{/snippet}

{#snippet rows(items: Item[], item: MeltItem, builders: MeltBuilders)}
	{#each items.filter((row) => !row.hide) as row (row.displayName)}
		{#if row.separatorTop}
			<div class="my-1 border-t border-border-light"></div>
		{/if}
		{#if row.submenuItems}
			<!-- Melt submenu: hover-opens and is floating-positioned (flips on screen edges). -->
			<DropdownSubmenuItem item={row} {builders} meltItem={item} />
		{:else}
			<MenuItem {item} class={ROW_CLASS} onClick={(e) => row.action?.(e)}>
				{#if row.icon}
					<row.icon size={14} class="shrink-0" />
				{/if}
				<span class="truncate grow min-w-0 text-2xs text-secondary">{row.displayName}</span>
				{#if row.selected}
					<Check size={14} class="shrink-0 text-primary" />
				{/if}
			</MenuItem>
		{/if}
	{/each}
{/snippet}

{#if config.readOnly}
	{@render trigger()}
{:else}
	<DropdownV2
		customMenu
		placement="bottom-end"
		fixedHeight={false}
		closeOnItemClick={false}
		bind:open={menuOpen}
	>
		{#snippet buttonReplacement()}
			{@render trigger()}
		{/snippet}
		{#snippet menu({ item, builders, close })}
			<div
				class="bg-surface-tertiary dark:border w-64 origin-top-right rounded-lg shadow-lg focus:outline-none py-1 text-xs"
			>
				{#if config.topItems}
					<div class={BLOCK_CLASS}>
						{@render rows(config.topItems(close), item, builders)}
					</div>
				{/if}
				{#each config.sections ?? [] as sec (sec.label)}
					<div class={BLOCK_CLASS}>
						{@render section(sec, item)}
					</div>
				{/each}
				{#if reasoning}
					<div class={BLOCK_CLASS}>
						{#if capability.supported}
							<!-- Registered as a melt item so it joins the roving focus/highlight (and arrow
						     up/down navigation), and so hovering it takes the highlight off the row
						     above. Left/right adjust the effort; the slider's input handler also drives it. -->
							<MenuItemWrapper
								{item}
								onKeydown={(e) => effortSlider?.adjust(e)}
								class="block group"
							>
								<ReasoningEffortSlider
									bind:this={effortSlider}
									{stops}
									current={currentStop}
									onSelect={reasoning.onSelect}
									format={(stop) => (stop === reasoning?.offToken ? 'off' : stop)}
									overrideLabel={stops.includes(currentStop) ? undefined : effortLabel}
								/>
							</MenuItemWrapper>
						{:else}
							<!-- Kept in place rather than dropped: the row saying the model cannot think
							     is the answer to why there is no slider. -->
							<ReasoningEffortSlider
								stops={[]}
								current=""
								onSelect={() => {}}
								unsupportedReason="Not supported by this model"
							/>
						{/if}
					</div>
				{/if}
				{#if config.bottomItems}
					<div class={BLOCK_CLASS}>
						{@render rows(config.bottomItems(close), item, builders)}
					</div>
				{/if}
			</div>
		{/snippet}
	</DropdownV2>
{/if}
