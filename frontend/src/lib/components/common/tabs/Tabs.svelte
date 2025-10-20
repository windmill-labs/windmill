<script lang="ts">
	import { setContext } from 'svelte'
	import { writable } from 'svelte/store'
	import { createEventDispatcher } from 'svelte'
	import { twMerge } from 'tailwind-merge'
	import type { TabsContext } from '$lib/components/apps/editor/settingsPanel/inputEditor/tabs.svelte'

	const dispatch = createEventDispatcher<{ selected: string }>()

	interface Props {
		selected: string
		hideTabs?: boolean
		class?: string
		wrapperClass?: string
		style?: string
		hashNavigation?: boolean
		values?: string[] | undefined
		children?: import('svelte').Snippet<[any]>
		content?: import('svelte').Snippet
		/**
		 * If true, the tab component will only update the internal store when a tab is clicked,
		 * but will NOT immediately update the bindable 'selected' prop. This allows the parent
		 * component to control when the tab actually changes (e.g., after navigation completes).
		 * Use this when you want to prevent navigation before checking for unsaved changes.
		 */
		deferSelectedUpdate?: boolean
	}

	let {
		selected = $bindable(),
		hideTabs = false,
		class: c = '',
		wrapperClass = '',
		style = '',
		hashNavigation = false,
		values = undefined,
		children,
		content,
		deferSelectedUpdate = false
	}: Props = $props()

	// Single source of truth for tab state
	const selectedStore = writable(selected)

	function update(value: string) {
		if (!deferSelectedUpdate) {
			selected = value
		}
		dispatch('selected', value)
	}

	setContext<TabsContext>('Tabs', {
		selected: selectedStore,
		update,
		hashNavigation
	})

	// Sync external prop changes to store (single direction: prop → store)
	$effect(() => {
		selectedStore.set(selected)
	})

	let hashValues = $derived(values ? values.map((x) => '#' + x) : undefined)

	function hashChange() {
		if (hashNavigation) {
			const hash = window.location.hash
			if (hash && hashValues?.includes(hash)) {
				const id = hash.replace('#', '')
				update(id)
			}
		}
	}
</script>

<svelte:window onhashchange={hashChange} />
{#if !hideTabs}
	<div class="overflow-x-auto {wrapperClass}">
		<div class={twMerge('border-b flex flex-row whitespace-nowrap scrollbar-hidden', c)} {style}>
			{@render children?.({ selected })}
		</div>
	</div>
{/if}
{@render content?.()}
