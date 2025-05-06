<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import { Trash, Save, Pen, X } from 'lucide-svelte'
	import { createEventDispatcher } from 'svelte'
	import Toggle from '$lib/components/Toggle.svelte'

	const dispatch = createEventDispatcher<{
		delete: undefined
		deploy: undefined
		reset: undefined
		'save-draft': undefined
		edit: undefined
		cancel: undefined
		'toggle-enabled': boolean
	}>()

	export let isDraftOnly
	export let hasDraft
	export let editMode
	export let saveDisabled
	export let enabled: boolean | undefined
	export let allowDraft
	export let edit
	export let isLoading
	export let neverSaved
	export let isEditor
	export let permissions: 'write' | 'create' | 'none'
</script>

{#if !allowDraft}
	{#if edit && enabled !== undefined}
		<div class="-mt-1">
			<Toggle
				size="sm"
				disabled={permissions === 'none' || !editMode}
				checked={enabled}
				options={{ right: 'enable', left: 'disable' }}
				on:change={({ detail }) => {
					dispatch('toggle-enabled', detail)
				}}
			/>
		</div>
	{/if}
	{#if (permissions === 'write' && edit) || (permissions === 'create' && editMode)}
		<Button
			size="sm"
			startIcon={{ icon: Save }}
			disabled={saveDisabled}
			on:click={() => {
				dispatch('deploy')
			}}
			loading={isLoading}
		>
			Save
		</Button>
	{/if}
{:else}
	<div class="flex flex-row gap-2 items-center">
		{#if !isDraftOnly && !hasDraft}
			<div class="center-center">
				<Toggle
					size="xs"
					disabled={permissions === 'none'}
					checked={enabled}
					options={{ left: 'enable' }}
					on:change={(e) => {
						dispatch('toggle-enabled', e.detail)
					}}
				/>
			</div>
		{/if}
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
		{:else if hasDraft && isEditor}
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
		{#if (permissions === 'write' && isEditor) || permissions === 'create'}
			{#if editMode}
				{@const dropdownItems =
					!isDraftOnly && isEditor
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
						if (isEditor) {
							dispatch('save-draft')
						} else if ((permissions === 'write' && edit) || permissions === 'create') {
							dispatch('deploy')
						}
					}}
					{dropdownItems}
				>
					{isEditor ? 'Save draft' : 'Save'}
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
			{:else if editMode && !neverSaved}
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
{/if}
