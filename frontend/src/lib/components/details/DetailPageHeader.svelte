<script lang="ts">
	import { Button } from '$lib/components/common'

	import Menu from '$lib/components/details/Menu.svelte'
	import MenuItem from '$lib/components/common/menu/MenuItem.svelte'

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
	}

	export let mainButtons: MainButton[] = []
	export let menuItems: MenuItemButton[] = []
</script>

<div class="border-b p-2 shadow-md">
	<div class="prose-sm mx-auto">
		<div class="flex w-full flex-wrap md:flex-nowrap justify-end gap-x-2 gap-y-4 h-8">
			<div class="flex gap-2">
				<Menu>
					<svelte:fragment slot="items">
						{#each menuItems as { label, Icon, onclick } (label)}
							<MenuItem on:click={onclick}>
								<div class="text-xs flex items-center gap-2 flex-row-2">
									<Icon class="h-4" />
									{label}
								</div>
							</MenuItem>
						{/each}
					</svelte:fragment>
				</Menu>
				{#each mainButtons as btn}
					<Button {...btn.buttonProps} startIcon={{ icon: btn.buttonProps.startIcon }}>
						{btn.label}
					</Button>
				{/each}
			</div>
		</div>
	</div>
</div>
