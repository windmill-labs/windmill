<script>
	import { Menu, Transition, MenuButton, MenuItems } from '@rgossiaux/svelte-headlessui'
</script>

<Menu let:open as="div" class="relative z-50">
	<MenuButton>
		<slot name="trigger" />
	</MenuButton>
	<Transition
		show={open}
		enter="transition ease-out duration-100"
		enterFrom="transform opacity-0 scale-95"
		enterTo="transform opacity-100 scale-100"
		leave="transition ease-in duration-75"
		leaveFrom="transform opacity-100 scale-100"
		leaveTo="transform opacity-0 scale-95"
	>
		<MenuItems
			class="absolute right-0 z-50 mt-2 w-56 origin-top-right divide-y divide-gray-100 rounded-md bg-white shadow-lg ring-1 ring-black ring-opacity-5 focus:outline-none"
		>
			<slot name="items" />
		</MenuItems>
	</Transition>
</Menu>
