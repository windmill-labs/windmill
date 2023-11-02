<script lang="ts">
	import { Menu, Transition, MenuItems, MenuButton } from '@rgossiaux/svelte-headlessui'
	import { createPopperActions, type PopperOptions } from 'svelte-popperjs'
	import Portal from 'svelte-portal'

	const [popperRef, popperContent] = createPopperActions({ placement: 'auto' })

	const popperOptions: PopperOptions<{}> = {
		placement: 'right-end',
		strategy: 'fixed',
		modifiers: [
			{ name: 'offset', options: { offset: [-100, 125] } },
			{
				name: 'arrow',
				options: {
					padding: 10
				}
			}
		]
	}
</script>

<Menu let:open as="div" class="relative hover:z-50 flex w-full h-full">
	<span use:popperRef>
		<MenuButton class="w-full">
			<slot />
		</MenuButton>
	</span>

	<Portal>
		<div use:popperContent={popperOptions} class="z-[6000]">
			<Transition
				show={open}
				enter="transition ease-out duration-[25ms]"
				enterFrom="transform opacity-0 scale-95"
				enterTo="transform opacity-100 scale-100"
				leave="transition ease-in duration-[25ms]"
				leaveFrom="transform opacity-100 scale-100"
				leaveTo="transform opacity-0 scale-95"
			>
				<MenuItems
					class="absolute border right-0 z-50 w-56 origin-top-right top-1 rounded-md bg-surface shadow-md focus:outline-none"
				>
					<div class="my-1">
						<slot name="items" />
					</div>
				</MenuItems>
			</Transition>
		</div>
	</Portal>
</Menu>
