<script lang="ts">
	import { createBubbler } from 'svelte/legacy'

	const bubble = createBubbler()
	import { melt, createSync } from '@melt-ui/svelte'
	import type { MenubarBuilders } from '@melt-ui/svelte'
	import type { Placement } from '@floating-ui/core'
	import { pointerDownOutside } from '$lib/utils'

	import { twMerge } from 'tailwind-merge'
	import ResolveOpen from '$lib/components/common/menu/ResolveOpen.svelte'

	interface Props {
		placement?: Placement
		justifyEnd?: boolean
		lightMode?: boolean
		maxHeight?: number
		disabled?: boolean
		createMenu: MenubarBuilders['createMenu']
		invisible?: boolean
		usePointerDownOutside?: boolean
		menuClass?: string
		open?: boolean
		renderContent?: boolean
		classNames?: string
		triggr?: import('svelte').Snippet<[any]>
		children?: import('svelte').Snippet<[any]>
		class?: string
	}

	let {
		placement = 'right-start',
		justifyEnd = false,
		lightMode = false,
		maxHeight = 900,
		disabled = false,
		createMenu,
		invisible = false,
		usePointerDownOutside = false,
		menuClass = '',
		open = $bindable(false),
		renderContent = false,
		class: classNames = '',
		triggr,
		children
	}: Props = $props()

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

	const sync = createSync(states)
	$effect(() => {
		sync.open(open, (v) => (open = Boolean(v)))
	})

	export function close() {
		open = false
	}

	async function getMenuElements(): Promise<HTMLElement[]> {
		return Array.from(document.querySelectorAll('[data-menu]')) as HTMLElement[]
	}
</script>

<div class={twMerge('w-full h-8', classNames)}>
	<ResolveOpen {open} on:open on:close />

	<button
		class={twMerge('w-full h-full', justifyEnd ? 'flex justify-end' : '')}
		{disabled}
		use:pointerDownOutside={{
			capture: true,
			stopPropagation: false,
			exclude: getMenuElements,
			customEventName: 'pointerdown_menu',
			onClickOutside: () => {
				if (usePointerDownOutside) {
					close()
				}
			}
		}}
		data-menu
	>
		{@render triggr?.({ trigger })}
	</button>

	<!--svelte-ignore a11y_no_static_element_interactions-->
	{#if open || renderContent}
		<div
			use:melt={$menuElement}
			data-menu
			class={twMerge(
				'z-[6000] border w-56 origin-top-right rounded-md shadow-md focus:outline-none overflow-y-auto',
				lightMode ? 'bg-surface-inverse' : 'bg-surface',
				invisible ? 'opacity-0' : '',
				menuClass
			)}
			onclick={bubble('click')}
		>
			<div class="py-1" style="max-height: {maxHeight}px; ">
				{@render children?.({ item, open })}
			</div>
		</div>
	{/if}
</div>
