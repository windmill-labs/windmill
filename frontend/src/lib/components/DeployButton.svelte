<script lang="ts" module>
	import type { Item } from '$lib/utils'

	export type DeployDropdownItem =
		| {
				label: string
				/** Gets the message typed in the hover popover, for items that deploy. */
				onClick: (deploymentMsg: string) => void
		  }
		| {
				label: string
				/** Renders a link, for an item that only navigates. */
				href: string
				hrefTarget?: Item['hrefTarget']
		  }
</script>

<script lang="ts">
	import { Button } from '$lib/components/common'
	import { Save, CornerDownLeft } from 'lucide-svelte'
	import { createEventDispatcher } from 'svelte'

	type DropdownItem = DeployDropdownItem

	const {
		loading = false,
		loadingSave = false,
		dropdownItems = [],
		unifiedSize = 'md',
		iconOnly = false
	}: {
		loading?: boolean
		loadingSave?: boolean
		dropdownItems?: DropdownItem[] | (() => DropdownItem[]) | undefined
		unifiedSize?: 'sm' | 'md' | 'lg'
		iconOnly?: boolean
	} = $props()

	const dispatch = createEventDispatcher()

	let msgInput: HTMLInputElement | undefined = $state(undefined)
	let hideDropdown = $state(false)
	let deploymentMsg = $state('')
	let dropdownOpen = $state(false)

	/** Call after a successful deploy: the next deploy from the same editor would store it again. */
	export function clearMessage() {
		deploymentMsg = ''
	}

	function withMessage(items: DropdownItem[]) {
		return items.map((item) =>
			'href' in item ? item : { ...item, onClick: () => item.onClick(deploymentMsg) }
		)
	}
	let items = $derived(
		typeof dropdownItems === 'function'
			? () => withMessage(dropdownItems())
			: dropdownItems && withMessage(dropdownItems)
	)
</script>

<Button
	disabled={loading}
	loading={loadingSave}
	variant="accent"
	{unifiedSize}
	{iconOnly}
	title={iconOnly ? 'Deploy' : undefined}
	startIcon={{ icon: Save }}
	on:click={() => dispatch('save', deploymentMsg)}
	dropdownItems={items}
	tooltipPopover={{
		placement: 'bottom-end',
		openDelay: dropdownOpen ? 200 : 0,
		closeDelay: 0,
		portal: 'body'
	}}
	on:tooltipOpen={async ({ detail }) => {
		if (detail) {
			// Use setTimeout to ensure DOM is updated
			setTimeout(() => {
				msgInput?.focus()
			}, 0)
			hideDropdown = true
		} else {
			hideDropdown = false
		}
	}}
	{hideDropdown}
	on:dropdownOpen={({ detail }) => {
		dropdownOpen = detail
	}}
>
	Deploy
	{#snippet tooltip()}
		<div class="flex flex-row gap-2 w-80 p-4 bg-surface rounded-lg shadow-lg dark:border z-[5001]">
			<input
				type="text"
				placeholder="Deployment message"
				bind:value={deploymentMsg}
				onkeydown={async (e) => {
					if (e.key === 'Enter') {
						dispatch('save', deploymentMsg)
					}
				}}
				bind:this={msgInput}
			/>
			<Button
				unifiedSize="md"
				variant="accent"
				on:click={async () => dispatch('save', deploymentMsg)}
				endIcon={{ icon: CornerDownLeft }}
				loading={loadingSave}
			>
				Deploy
			</Button>
		</div>
	{/snippet}
</Button>
