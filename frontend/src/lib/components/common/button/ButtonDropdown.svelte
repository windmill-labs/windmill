<script>
	import { Menu, Transition, MenuButton, MenuItems } from '@rgossiaux/svelte-headlessui'
	import { ChevronDown } from 'lucide-svelte'
</script>

<Menu let:open as="div" class="relative z-50 flex w-full h-full">
	<MenuButton class="h-full w-full px-2">
		<ChevronDown class="w-5 h-5" />
	</MenuButton>
	<Transition
		show={open}
		enter="transition ease-out duration-75"
		enterFrom="transform opacity-0 scale-95"
		enterTo="transform opacity-100 scale-100"
		leave="transition ease-in duration-75"
		leaveFrom="transform opacity-100 scale-100"
		leaveTo="transform opacity-0 scale-95"
	>
		<MenuItems
			class="absolute border right-0 z-50 mt-2 w-56 origin-top-right top-8 rounded-md bg-white shadow-md focus:outline-none"
		>
			<div class="my-1">
				<slot name="items" />
			</div>
		</MenuItems>
	</Transition>
</Menu>
