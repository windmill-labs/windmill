<script lang="ts">
	import { Badge, Button } from '$lib/components/common'

	import Menu from '$lib/components/details/Menu.svelte'
	import MenuItem from '$lib/components/common/menu/MenuItem.svelte'
	import { classNames } from '$lib/utils'
	import ErrorHandlerToggleButton from './ErrorHandlerToggleButton.svelte'

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
	export let tag: string | undefined

	export let errorHandlerKind: 'flow' | 'script'
	export let scriptOrFlowPath: string
	export let errorHandlerMuted: boolean | undefined
</script>

<div class="border-b p-2 shadow-md">
	<div class="mx-auto">
		<div class="flex w-full flex-wrap md:flex-nowrap justify-end gap-x-2 gap-y-4 items-center">
			<div class="grow px-2 inline-flex items-center gap-4 min-w-0">
				<div class="text-lg min-w-24 font-bold truncate">{title}</div>{#if tag}
					<Badge>tag: {tag}</Badge>
				{/if}
				<slot />
			</div>
			<div class="flex gap-1 md:gap-2 items-center">
				{#if menuItems.length > 0}
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
											'text-xs flex items-center gap-2 flex-row-2 ',
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
				{/if}
				<ErrorHandlerToggleButton
					kind={errorHandlerKind}
					{scriptOrFlowPath}
					bind:errorHandlerMuted
				/>
				{#each mainButtons as btn}
					<Button
						{...btn.buttonProps}
						startIcon={{ icon: btn.buttonProps.startIcon }}
						on:click={btn.buttonProps.onClick}
						btnClasses="hidden md:flex items-center gap-1"
					>
						{btn.label}
					</Button>
					<Button
						{...btn.buttonProps}
						startIcon={{ icon: btn.buttonProps.startIcon }}
						on:click={btn.buttonProps.onClick}
						iconOnly
						btnClasses="flex md:hidden items-center gap-1"
					>
						{btn.label}
					</Button>
				{/each}
			</div>
		</div>
	</div>
</div>
