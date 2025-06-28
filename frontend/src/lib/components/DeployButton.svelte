<script lang="ts">
	import { Button } from '$lib/components/common'
	import { Save, CornerDownLeft } from 'lucide-svelte'
	import { createEventDispatcher } from 'svelte'

	const {
		loading = false,
		loadingSave = false,
		newFlow = false,
		dropdownItems = []
	}: {
		loading?: boolean
		loadingSave?: boolean
		newFlow?: boolean
		dropdownItems?: Array<{
			label: string
			onClick: () => void
		}>
	} = $props()

	const dispatch = createEventDispatcher()

	let msgInput: HTMLInputElement | undefined = $state(undefined)
	let hideDropdown = $state(false)
	let deploymentMsg = $state('')
	let dropdownOpen = $state(false)
</script>

<Button
	disabled={loading}
	loading={loadingSave}
	size="xs"
	startIcon={{ icon: Save }}
	on:click={() => dispatch('save')}
	dropdownItems={!newFlow ? dropdownItems : undefined}
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
	on:dropdownOpen={({ detail }) => (dropdownOpen = detail)}
>
	Deploy
	<svelte:fragment slot="tooltip">
		<div class="flex flex-row gap-2 w-80 p-4 bg-surface rounded-lg shadow-lg border z-[5001]">
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
				size="xs"
				on:click={async () => dispatch('save', deploymentMsg)}
				endIcon={{ icon: CornerDownLeft }}
				loading={loadingSave}
			>
				Deploy
			</Button>
		</div>
	</svelte:fragment>
</Button>
