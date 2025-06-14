<script lang="ts">
	import ConfirmationModal from './ConfirmationModal.svelte'
	import Button from '../button/Button.svelte'
	import type DiffDrawer from '$lib/components/DiffDrawer.svelte'
	import {  type Value } from '$lib/utils'

	export let deployedValue: Value | undefined = undefined
	export let currentValue: Value | undefined = undefined
	export let diffDrawer: DiffDrawer | undefined = undefined
	export let confirmCallback: () => void
	export let deployedBy : string | undefined = undefined
	export let open = false
</script>

<ConfirmationModal
	{open}
	title={"New version deployed by " + deployedBy}
	confirmationText="Override"
	on:canceled={() => {
		open = false
	}}
	on:confirmed={() => confirmCallback()}
>
	<div class="flex flex-col w-full space-y-4">
		<span>A new version was deployed while you were editing this one.</span>
		{#if diffDrawer}
			<Button
				wrapperClasses="self-start"
				color="light"
				variant="border"
				size="xs"
				on:click={() => {
					if (!deployedValue || !currentValue) {
						return
					}
					open = false
					diffDrawer?.openDrawer()
					diffDrawer?.setDiff({
						mode: 'simple',
						original: deployedValue,
						current: currentValue,
						title: 'Deployed <> Current',
						button: {
							text: 'Override anyway',
							onClick: () => confirmCallback()
						}
					})
				}}
				>Show diff
			</Button>
		{/if}
	</div>
</ConfirmationModal>
