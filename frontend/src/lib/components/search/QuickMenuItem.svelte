<script lang="ts">
	import { isMac } from '$lib/utils'
	import { createEventDispatcher, onDestroy, onMount } from 'svelte'
	import { twMerge } from 'tailwind-merge'

	export let hovered: boolean = false
	export let id: string
	export let label: string = ''
	export let icon: any = undefined
	export let shortcutKey: string | undefined = undefined

	const dispatch = createEventDispatcher()

	onMount(() => {
		window.addEventListener('keydown', handleKeydown)
	})

	onDestroy(() => {
		window.removeEventListener('keydown', handleKeydown)
	})

	async function handleKeydown(event: KeyboardEvent) {
		if (hovered && event.key === 'Enter') {
			event.preventDefault()
			runAction()
		}
	}

	function runAction() {
		dispatch('select')
	}
	export let kbdClass = ''
	export let small = true
	if (small) {
		kbdClass = twMerge(
			kbdClass,
			'!text-[10px]  px-1',
			false && isMac() ? '!text-lg ' : 'text-xs',
			'leading-none'
		)
	} else {
		kbdClass += ' !text-xs px-1.5'
	}
</script>

<div
	{id}
	on:click|stopPropagation={runAction}
	on:mouseenter={() => dispatch('hover')}
	class={`rounded-md w-full ${hovered ? 'bg-surface-hover' : ''}`}
>
	{#if $$slots.itemReplacement}
		<slot name="itemReplacement" />
	{:else}
		<div class="flex flex-row gap-2 items-center px-1 py-0.5 rounded-md pr-6 font-light">
			<div class="w-4">
				{#if icon}
					<svelte:component this={icon} size={14} />
				{:else if shortcutKey != undefined}
					<div class="font-bold flex items-center justify-center w-full">
						<span
							class="h-4 center-center ml-0.5 rounded border bg-surface-secondary text-primary shadow-sm font-light transition-all group-hover:border-primary-500 group-hover:text-primary-inverse"
						>
							<kbd class={kbdClass}>
								{shortcutKey}
							</kbd>
						</span>
					</div>
				{/if}
			</div>
			{label}
			{#if shortcutKey != undefined}
				<div class="ml-auto">
					<div class="font-bold flex items-center justify-center w-full">
						<span
							class="flex h-4 center-center ml-0.5 rounded border bg-surface-secondary text-primary shadow-sm font-light transition-all group-hover:border-primary-500 group-hover:text-primary-inverse"
						>
							<kbd class={kbdClass}>
								{shortcutKey}
							</kbd>
						</span>
					</div>
				</div>
			{/if}
		</div>
	{/if}
</div>
