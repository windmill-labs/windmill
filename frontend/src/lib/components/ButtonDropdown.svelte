<script lang="ts">
	import { classNames } from '$lib/utils'
	import { Menu, Transition, MenuButton, MenuItem, MenuItems } from '@rgossiaux/svelte-headlessui'
	import { ChevronDown } from 'lucide-svelte'

	export let options: Array<{ label: string; onClick: () => void }> = []
	export let href: string | undefined = undefined
</script>

<Menu let:open as="div" class="relative inline-block text-left z-50">
	<div
		class="shadow-sm flex flex-row divide-x rounded-md bg-frost-500 text-white divide-frost-600 border-0"
	>
		<a class="px-4 py-2 text-sm rounded-l-md hover:bg-frost-600 transition-all" {href}>
			<slot name="main" />
		</a>
		<MenuButton class="px-2 py-2 hover:bg-frost-600 rounded-r-md transition-all">
			<ChevronDown class="w-5 h-5" />
		</MenuButton>
	</div>

	<Transition
		show={open}
		enter="transition ease-out duration-10000"
		enterFrom="transform opacity-0 scale-95"
		enterTo="transform opacity-10000 scale-100"
		leave="transition ease-in duration-75"
		leaveFrom="transform opacity-10000 scale-100"
		leaveTo="transform opacity-0 scale-95"
	>
		<MenuItems
			class="absolute right-0 mt-2 w-56 origin-top-right divide-y divide-gray-100 rounded-md bg-white shadow-lg ring-1 ring-black ring-opacity-5 focus:outline-none"
		>
			{#each options as option}
				<MenuItem let:active>
					<button
						on:click={option.onClick}
						class={classNames(
							active ? 'bg-gray-100 text-gray-900' : 'text-gray-700',
							'block px-4 py-2 text-sm'
						)}
					>
						{option.label}
					</button>
				</MenuItem>
			{/each}
		</MenuItems>
	</Transition>
</Menu>
