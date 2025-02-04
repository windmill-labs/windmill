<script lang="ts">
	import Portal from '$lib/components/Portal.svelte'
	import { melt, createSync } from '@melt-ui/svelte'

	import { twMerge } from 'tailwind-merge'
	import ResolveOpen from '$lib/components/common/menu/ResolveOpen.svelte'

	export let placement: any = 'right-start'
	export let justifyEnd: boolean = false
	export let lightMode: boolean = false
	export let maxHeight: number = 900
	export let disabled = false
	export let createMenu: (any) => any
	export let invisible: boolean = false

	// Use the passed createMenu function
	const menu = createMenu({
		positioning: {
			placement
		}
	})

	//Melt
	const {
		elements: { trigger, menu: menuElement, item },
		states
	} = menu

	let open = false

	const sync = createSync(states)
	$: sync.open(open, (v) => (open = Boolean(v)))
</script>

<div class="w-full h-8">
	<ResolveOpen {open} on:open on:close />

	<button class={twMerge('w-full', justifyEnd ? 'flex justify-end' : '')} {disabled}>
		<slot name="trigger" trigger={$trigger} />
	</button>

	<Portal name="menu-v3">
		<div class="z-[6000]" use:melt={$menuElement}>
			<div
				class={twMerge(
					'border w-56 origin-top-right rounded-md shadow-md focus:outline-none overflow-y-auto',
					lightMode ? 'bg-surface-inverse' : 'bg-surface',
					invisible ? 'opacity-0' : ''
				)}
				style="max-height: {maxHeight}px;"
			>
				<div class="my-1">
					<slot item={$item} />
				</div>
			</div>
		</div>
	</Portal>
</div>
