<script lang="ts">
	import { Menu, Transition, MenuButton, MenuItems } from '@rgossiaux/svelte-headlessui'
	import { ChevronDown } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import { createPopperActions, type PopperOptions } from 'svelte-popperjs'
	import Portal from 'svelte-portal'

	export let hasPadding: boolean = true
	export let target: string | undefined = 'body'
	const [popperRef, popperContent] = createPopperActions({ placement: 'auto' })

	const popperOptions: PopperOptions<{}> = {
		placement: 'bottom-end',
		strategy: 'fixed',
		modifiers: [
			{ name: 'offset', options: { offset: [8, 8] } },
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
		<MenuButton
			class={twMerge('h-full w-full flex flex-row gap-2 items-center', hasPadding ? 'px-2' : '')}
		>
			{#if $$slots.buttonReplacement}
				<slot name="buttonReplacement" />
			{:else}
				<slot name="label" />
				<ChevronDown class="w-5 h-5" />
			{/if}
		</MenuButton>
	</span>

	<Portal {target}>
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
