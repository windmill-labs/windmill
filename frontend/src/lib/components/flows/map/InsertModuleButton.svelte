<script lang="ts">
	import { preventDefault, stopPropagation } from 'svelte/legacy'

	import { createEventDispatcher } from 'svelte'
	import { Bug, Cross } from 'lucide-svelte'
	import InsertModuleInner from './InsertModuleInner.svelte'
	import { twMerge } from 'tailwind-merge'
	import type { ComputeConfig } from 'svelte-floating-ui'

	import PopupV2 from '$lib/components/common/popup/PopupV2.svelte'
	import { flip, offset } from 'svelte-floating-ui/dom'
	import { SchedulePollIcon } from '$lib/components/icons'

	// import type { Writable } from 'svelte/store'

	const dispatch = createEventDispatcher()

	type Alignment = 'start' | 'end' | 'center'
	type Side = 'top' | 'bottom'
	type Placement = `${Side}-${Alignment}`

	interface Props {
		index?: number
		funcDesc?: string
		kind?: 'script' | 'trigger' | 'preprocessor' | 'failure'
		iconSize?: number
		clazz?: string
		placement?: Placement
	}

	let {
		index = 0,
		funcDesc = $bindable(''),
		kind = 'script',
		iconSize = 12,
		clazz = '',
		placement = 'bottom-center'
	}: Props = $props()

	let floatingConfig: ComputeConfig = {
		strategy: 'fixed',
		// @ts-ignore
		placement,
		middleware: [offset(8), flip()],
		autoUpdate: true
	}

	let open = $state(false)
	$effect(() => {
		!open && (funcDesc = '')
	})
</script>

<!-- <Menu transitionDuration={0} pointerDown bind:show={open} noMinW {placement} let:close> -->

<!-- {floatingConfig}
floatingClasses="mt-2"
containerClasses="border rounded-lg shadow-lg  bg-surface"
noTransition
shouldUsePortal={true} -->

<PopupV2 {floatingConfig} bind:open target="#flow-editor">
	{#snippet button({ pointerdown, pointerup })}
		<button
			title={`Add ${
				kind === 'failure'
					? ' failure module '
					: kind === 'preprocessor'
						? 'preprocessor step'
						: kind === 'trigger'
							? 'trigger'
							: 'step'
			}`}
			id={`flow-editor-add-step-${index}`}
			type="button"
			class={twMerge(
				'w-[17.5px] h-[17.5px] flex items-center justify-center !outline-[1px] outline dark:outline-gray-500 outline-gray-300 text-secondary bg-surface focus:outline-none hover:bg-surface-hover rounded',
				clazz
			)}
			onpointerdown={stopPropagation(
				preventDefault(() => {
					dispatch('open')
					pointerdown()
				})
			)}
			onpointerup={pointerup}
		>
			{#if kind === 'trigger'}
				<SchedulePollIcon size={14} />
			{:else if kind === 'failure'}
				<div class="flex items-center gap-1">
					<Bug size={14} />
					<span class="text-xs w-20">Error Handler</span>
				</div>
			{:else}
				<Cross size={iconSize} />
			{/if}
		</button>
	{/snippet}
	{#snippet children({ close })}
		<InsertModuleInner
			on:close={() => close(null)}
			on:insert
			on:new
			on:pickFlow
			on:pickScript
			{kind}
		/>
	{/snippet}
</PopupV2>
