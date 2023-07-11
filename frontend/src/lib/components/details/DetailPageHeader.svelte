<script lang="ts">
	import { Button } from '$lib/components/common'

	import Menu from '$lib/components/details/Menu.svelte'
	import MenuItem from '$lib/components/common/menu/MenuItem.svelte'
	import { classNames } from '$lib/utils'

	type MainButton = {
		label: string
		href: string
		buttonProps: ButtonProps
	}

	type ButtonProps = any
	type MenuItemButton = {
		label: string
		Icon: any
		onclick: () => void
		color?: 'red'
	}

	export let mainButtons: MainButton[] = []
	export let menuItems: MenuItemButton[] = []
	export let title: string
</script>

<div class="border-b p-2 shadow-md">
	<div class="mx-auto">
		<div class="flex w-full flex-wrap md:flex-nowrap justify-end gap-x-2 gap-y-4 h-8 items-center">
			<div class="grow text-lg font-bold truncate px-2 w-24 sm:w-full">
				{title}
			</div>
			<div class="flex gap-1 md:gap-2 items-center">
				<Menu>
					<svelte:fragment slot="items">
						{#each menuItems as { label, Icon, onclick, color } (label)}
							<MenuItem
								on:click={() => {
									const div = document.querySelector('[id^="headlessui-menu-items"]')
									div?.parentElement?.remove()

									onclick()
								}}
							>
								<div
									class={classNames(
										'text-xs flex items-center gap-2 flex-row-2',
										color === 'red' ? 'text-red-500' : ''
									)}
								>
									<Icon class="h-4" />
									{label}
								</div>
							</MenuItem>
						{/each}
					</svelte:fragment>
				</Menu>
				{#each mainButtons as btn}
					<Button
						{...btn.buttonProps}
						startIcon={{ icon: btn.buttonProps.startIcon }}
						on:click={btn.buttonProps.onClick}
						btnClasses="hidden md:block"
					>
						{btn.label}
					</Button>
					<Button
						{...btn.buttonProps}
						startIcon={{ icon: btn.buttonProps.startIcon }}
						on:click={btn.buttonProps.onClick}
						iconOnly
						btnClasses="block md:hidden"
					>
						{btn.label}
					</Button>
				{/each}
			</div>
		</div>
	</div>
</div>
