<script lang="ts">
	import { melt, createSync } from '@melt-ui/svelte'
	import type { MenubarBuilders } from '@melt-ui/svelte'
	import type { Placement } from '@floating-ui/core'
	import { pointerDownOutside } from '$lib/utils'

	import { twMerge } from 'tailwind-merge'
	import ResolveOpen from '$lib/components/common/menu/ResolveOpen.svelte'

	export let placement: Placement = 'right-start'
	export let justifyEnd: boolean = false
	export let lightMode: boolean = false
	export let maxHeight: number = 900
	export let disabled = false
	export let createMenu: MenubarBuilders['createMenu']
	export let invisible: boolean = false
	export let usePointerDownOutside: boolean = false

	// Use the passed createMenu function
	const menu = createMenu({
		positioning: {
			placement,
			fitViewport: true,
			strategy: 'fixed'
		},
		loop: true
	})

	//Melt
	const {
		elements: { trigger, menu: menuElement, item },
		states
	} = menu

	let open = false

	const sync = createSync(states)
	$: sync.open(open, (v) => (open = Boolean(v)))

	export function close() {
		open = false
	}

	async function getMenuElements(): Promise<HTMLElement[]> {
		return Array.from(document.querySelectorAll('[data-menu]')) as HTMLElement[]
	}
</script>

<div class={twMerge('w-full h-8', $$props.class)}>
	<ResolveOpen {open} on:open on:close />

	<button
		class={twMerge('w-full h-full', justifyEnd ? 'flex justify-end' : '')}
		{disabled}
		use:pointerDownOutside={{
			capture: true,
			stopPropagation: false,
			exclude: getMenuElements,
			customEventName: 'pointerdown_menu'
		}}
		on:pointerdown_outside={() => {
			if (usePointerDownOutside) {
				close()
			}
		}}
		data-menu
	>
		<slot name="trigger" {trigger} />
	</button>

	<!--svelte-ignore a11y-no-static-element-interactions-->
	{#if open}
		<div
			use:melt={$menuElement}
			data-menu
			class={twMerge(
				'z-[6000] border w-56 origin-top-right rounded-md shadow-md focus:outline-none overflow-y-auto',
				lightMode ? 'bg-surface-inverse' : 'bg-surface',
				invisible ? 'opacity-0' : ''
			)}
			on:click
		>
			<div class="py-1" style="max-height: {maxHeight}px; ">
				<slot {item} />
			</div>
		</div>
	{/if}
</div>
