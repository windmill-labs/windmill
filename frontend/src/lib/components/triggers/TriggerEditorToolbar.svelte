<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import { Trash, Save, Pen, X } from 'lucide-svelte'
	import { createEventDispatcher } from 'svelte'

	const dispatch = createEventDispatcher<{
		delete: undefined
		deploy: undefined
		reset: undefined
		'save-draft': undefined
		edit: undefined
		cancel: undefined
	}>()

	export let isDraftOnly = false
	export let hasDraft = false
	export let canEdit = false
	export let editMode = false
	export let saveDisabled = false
</script>

<div class="flex flex-row gap-2 items-center">
	{#if isDraftOnly}
		<Button
			size="xs"
			startIcon={{ icon: Trash }}
			iconOnly
			color={'light'}
			on:click={() => {
				dispatch('delete')
			}}
			btnClasses="hover:bg-red-500 hover:text-white"
		/>
	{:else if hasDraft}
		<DropdownV2
			items={[
				{
					displayName: 'Reset to deployed version',
					action: () => {
						dispatch('reset')
					}
				}
			]}
		/>
	{/if}
	{#if canEdit}
		{#if editMode}
			{@const dropdownItems = !isDraftOnly
				? [
						{
							label: 'Deploy changes now',
							onClick: () => {
								dispatch('deploy')
							}
						}
					]
				: undefined}
			<Button
				size="xs"
				startIcon={{ icon: Save }}
				disabled={saveDisabled}
				on:click={() => {
					dispatch('save-draft')
				}}
				{dropdownItems}
			>
				{'Save draft'}
			</Button>
		{/if}
		{#if !editMode}
			<Button
				size="xs"
				color="light"
				startIcon={{ icon: Pen }}
				on:click={() => {
					dispatch('edit')
				}}
			>
				Edit
			</Button>
		{:else if editMode}
			<Button
				size="xs"
				color="light"
				startIcon={{ icon: X }}
				on:click={() => {
					dispatch('cancel')
				}}
			>
				Cancel
			</Button>
		{/if}
	{/if}
</div>
