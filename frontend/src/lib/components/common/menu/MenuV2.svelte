<script lang="ts">
	import { Menu, MenuButton, MenuItems, Transition } from '@rgossiaux/svelte-headlessui'
	import Portal from 'svelte-portal'
	import { offset, flip, shift } from 'svelte-floating-ui/dom'
	import { createFloatingActions } from 'svelte-floating-ui'

	export let placement: any = 'bottom-start'

	const [floatingRef, floatingContent] = createFloatingActions({
		strategy: 'fixed',
		middleware: [offset(), flip(), shift()],
		placement: placement
	})
</script>

<Menu let:open as="div" class="relative hover:z-50 flex w-full h-8">
	<div use:floatingRef class="w-full">
		<MenuButton class="w-full">
			<slot name="trigger" />
		</MenuButton>
	</div>
	<Portal>
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
					class="border w-56 origin-top-right rounded-md bg-surface shadow-md focus:outline-none"
				>
					<div class="my-1">
						<slot />
					</div>
				</MenuItems>
			</Transition>
		</div>
	</Portal>
</Menu>
