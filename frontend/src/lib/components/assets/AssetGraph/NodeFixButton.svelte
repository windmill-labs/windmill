<!--
@component
The "Fix" pill on the bottom edge of a misconfigured (red) pipeline node: what is
wrong, and the actions that fix it. Styled like the node's "+" pill; the node's
red fill already says something is wrong.
-->
<script lang="ts" module>
	export type NodeFixAction = {
		label: string
		/** The side effect, spelled out: what changes, where. */
		detail?: string
		/** `anchor`: where the Fix pill sits, for an action opening a menu there. */
		run: (anchor: { x: number; y: number }) => void
	}
	/** Prose, with the entities it names as links that select them. */
	export type FixExplainer = Array<string | { text: string; onClick: () => void }>
	export type NodeFix = { explainer: FixExplainer; actions: NodeFixAction[] }

	/** A fix as its supplier declares it: an action, or opening the schedule
	 * wizard (which the canvas owns). */
	export type NodeFixSpec = {
		explainer: FixExplainer
		actions: Array<{
			label: string
			detail?: string
			run?: () => void
			/** Opens the schedule wizard for this script; its confirm calls `onSchedule`. */
			scheduleFor?: {
				script: string
				onSchedule: (s: import('./PipelineInsertMenu.svelte').PipelineInsertSchedule) => void
			}
		}>
	}
</script>

<script lang="ts">
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import { Button } from '$lib/components/common'
	import { Wrench } from 'lucide-svelte'

	let {
		fix,
		label = 'Fix',
		chip = false
	}: {
		fix: NodeFix
		label?: string
		/** A small chip in a node's text, rather than the pill on its edge. */
		chip?: boolean
	} = $props()

	let open = $state(false)
	let anchorEl: HTMLElement | undefined = $state()
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
	bind:this={anchorEl}
	onpointerdown={(e) => e.stopPropagation()}
	onkeydown={(e) => e.stopPropagation()}
	onclick={(e) => e.stopPropagation()}
>
	<Popover
		bind:isOpen={open}
		placement="bottom"
		enableFlyTransition
		usePointerDownOutside
		contentClasses="p-3 w-96"
	>
		{#snippet trigger()}
			{#if chip}
				<span
					class="flex items-center gap-1 rounded-md border px-1.5 py-0.5 text-3xs leading-none font-normal whitespace-nowrap bg-surface border-red-300 dark:border-red-600 text-red-700 dark:text-red-300 hover:bg-surface-hover"
				>
					<Wrench size={10} class="shrink-0" />
					{label}
				</span>
			{:else}
				<span
					class="h-6 px-2 rounded-full flex items-center gap-1 bg-surface border border-gray-400 dark:border-gray-600 text-secondary hover:text-primary hover:border-gray-500 shadow-sm text-2xs font-normal leading-none whitespace-nowrap"
				>
					<Wrench size={12} class="shrink-0" />
					{label}
				</span>
			{/if}
		{/snippet}
		{#snippet content()}
			<div class="flex flex-col gap-3">
				<p class="text-xs text-primary">
					{#each fix.explainer as part}
						{#if typeof part === 'string'}{part}{:else}<button
								type="button"
								class="text-accent hover:underline"
								onclick={() => {
									open = false
									part.onClick()
								}}>{part.text}</button
							>{/if}
					{/each}
				</p>
				<div class="flex flex-col gap-2">
					{#each fix.actions as action, i (action.label)}
						<div class="flex flex-col gap-0.5">
							<Button
								variant={i === 0 ? 'accent' : 'default'}
								unifiedSize="sm"
								onclick={() => {
									const r = anchorEl?.getBoundingClientRect()
									open = false
									action.run({ x: r ? r.left + r.width / 2 : 0, y: r ? r.bottom : 0 })
								}}
							>
								{action.label}
							</Button>
							{#if action.detail}
								<span class="text-2xs text-secondary">{action.detail}</span>
							{/if}
						</div>
					{/each}
				</div>
			</div>
		{/snippet}
	</Popover>
</div>
