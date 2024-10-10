<script lang="ts">
	import { Menu, MenuButton, MenuItems, Transition } from '@rgossiaux/svelte-headlessui'
	import Portal from '$lib/components/Portal.svelte'

	import { offset, flip, shift } from 'svelte-floating-ui/dom'
	import { createFloatingActions } from 'svelte-floating-ui'
	import { twMerge } from 'tailwind-merge'
	import ResolveOpen from './ResolveOpen.svelte'

	export let placement: any = 'bottom-start'
	export let justifyEnd: boolean = false
	export let lightMode: boolean = false
	export let maxHeight: number = 900
	const [floatingRef, floatingContent] = createFloatingActions({
		strategy: 'fixed',
		middleware: [offset(), flip(), shift()],
		placement: placement
	})
</script>

<Menu let:open as="div" class="relative hover:z-50 flex w-full h-8">
	<ResolveOpen {open} on:open on:close />
	<div use:floatingRef class="w-full">
		<MenuButton class={twMerge('w-full', justifyEnd ? 'flex justify-end' : '')}>
			<slot name="trigger" />
		</MenuButton>
	</div>
	<Portal name="menu-v2">
		<div use:floatingContent class="z-[6000]">
			<Transition
				{open}
				enter="transition ease-out duration-[25ms]"
				enterFrom="transform opacity-0 scale-95"
				enterTo="transform opacity-100 scale-100"
				leave="transition ease-in duration-[25ms]"
				leaveFrom="transform opacity-100 scale-100"
				leaveTo="transform opacity-0 scale-95"
			>
				<MenuItems
					class={twMerge(
						'border w-56 origin-top-right rounded-md shadow-md focus:outline-none overflow-y-auto',
						lightMode ? 'bg-surface-inverse' : 'bg-surface'
					)}
					style="max-height: {maxHeight}px;"
				>
					<div class="my-1">
						<slot />
					</div>
				</MenuItems>
			</Transition>
		</div>
	</Portal>
</Menu>
