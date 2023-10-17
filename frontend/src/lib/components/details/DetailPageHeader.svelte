<script lang="ts">
	import { Button } from '$lib/components/common'

	import { Bell, BellOff } from 'lucide-svelte'
	import Menu from '$lib/components/details/Menu.svelte'
	import MenuItem from '$lib/components/common/menu/MenuItem.svelte'
	import { sendUserToast } from '$lib/toast'
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

	export let errorHandlerMuted: boolean | undefined
	export let errorHandlerToggleFunc: (e: boolean) => void

	async function toggleErrorHandler(): Promise<void> {
		try {
			await errorHandlerToggleFunc(!errorHandlerMuted)
		} catch (error) {
			sendUserToast(
				`Error while toggling Workspace Error Handler: ${error.body || error.message}`,
				true
			)
			return
		}
		errorHandlerMuted = !errorHandlerMuted
		sendUserToast(
			errorHandlerMuted ? 'Workspace error handler muted' : 'Workspace error handler active',
			false
		)
	}
</script>

<div class="border-b p-2 shadow-md">
	<div class="mx-auto">
		<div class="flex w-full flex-wrap md:flex-nowrap justify-end gap-x-2 gap-y-4 h-8 items-center">
			<div class="grow px-2 sm:w-full inline-flex items-center gap-4">
				<div class="text-lg min-w-24 font-bold truncate">{title}</div>
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
				<Button
					title={errorHandlerMuted === undefined || !errorHandlerMuted
						? 'Disable workspace error handler for this script'
						: 'Enable workspace error handler for this script'}
					size="xs"
					on:click={toggleErrorHandler}
					color="light"
				>
					{#if errorHandlerMuted === undefined || !errorHandlerMuted}
						<div class="flex flex-row items-center">
							<Bell class="w-4" size={12} fill="currentcolor" />
						</div>
					{:else}
						<div class="flex flex-row items-center">
							<BellOff class="w-4" size={12} fill="currentcolor" />
						</div>
					{/if}
				</Button>
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
