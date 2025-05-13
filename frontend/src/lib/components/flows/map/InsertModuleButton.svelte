<script lang="ts">
	import { createEventDispatcher } from 'svelte'
	import { Cross } from 'lucide-svelte'
	import InsertModuleInner from './InsertModuleInner.svelte'
	import { twMerge } from 'tailwind-merge'
	import type { ComputeConfig } from 'svelte-floating-ui'

	import PopupV2 from '$lib/components/common/popup/PopupV2.svelte'
	import { flip, offset } from 'svelte-floating-ui/dom'
	import { SchedulePollIcon } from '$lib/components/icons'

	// import type { Writable } from 'svelte/store'

	const dispatch = createEventDispatcher()
	export let index: number = 0
	export let funcDesc = ''
	export let kind: 'script' | 'trigger' | 'preprocessor' | 'failure' = 'script'
	export let iconSize = 12

	type Alignment = 'start' | 'end' | 'center'
	type Side = 'top' | 'bottom'
	type Placement = `${Side}-${Alignment}`

	export let placement: Placement = 'bottom-center'

	let floatingConfig: ComputeConfig = {
		strategy: 'fixed',
		// @ts-ignore
		placement,
		middleware: [offset(8), flip()],
		autoUpdate: true
	}
	$: !open && (funcDesc = '')

	let open = false
</script>

<!-- <Menu transitionDuration={0} pointerDown bind:show={open} noMinW {placement} let:close> -->

<!-- {floatingConfig}
floatingClasses="mt-2"
containerClasses="border rounded-lg shadow-lg  bg-surface"
noTransition
shouldUsePortal={true} -->

<PopupV2 {floatingConfig} bind:open let:close target="#flow-editor">
	<svelte:fragment let:pointerdown let:pointerup slot="button">
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
				$$props.class
			)}
			on:pointerdown|preventDefault|stopPropagation={() => {
				dispatch('open')
				pointerdown()
			}}
			on:pointerup={pointerup}
		>
			{#if kind === 'trigger'}
				<SchedulePollIcon size={14} />
			{:else}
				<Cross size={iconSize} />
			{/if}
		</button>
	</svelte:fragment>
	<InsertModuleInner on:close={() => close(null)} on:insert on:new on:pickFlow on:pickScript />
</PopupV2>
