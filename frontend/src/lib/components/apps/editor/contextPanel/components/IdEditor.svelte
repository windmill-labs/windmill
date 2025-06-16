<script lang="ts">
	import type { AppViewerContext } from '$lib/components/apps/types'
	import { allItems } from '$lib/components/apps/utils'
	import { Pencil } from 'lucide-svelte'
	import { createEventDispatcher, getContext } from 'svelte'
	import IdEditorInput from '$lib/components/IdEditorInput.svelte'
	import Popover from '$lib/components/meltComponents/Popover.svelte'

	const { app, selectedComponent } = getContext<AppViewerContext>('AppViewerContext')

	export let id: string

	const dispatch = createEventDispatcher()

	$: reservedIds = allItems(app.val.grid, app.val.subgrids).map((item) => item.id)
</script>

<Popover
	floatingConfig={{ strategy: 'absolute', placement: 'bottom-start' }}
	closeOnOtherPopoverOpen
	contentClasses="p-4"
>
	<svelte:fragment slot="trigger">
		<button
			on:click={() => {
				$selectedComponent = [id]
			}}
			title="Edit ID"
			class="flex items-center px-1 rounded-sm bg-surface-secondary hover:text-primary text-secondary h-5"
			aria-label="Open component ID editor"
		>
			<Pencil size={14} />
		</button>
	</svelte:fragment>
	<svelte:fragment slot="content" let:close>
		<IdEditorInput
			initialId={id}
			on:close={() => close()}
			on:save={(e) => {
				dispatch('save', e.detail)
				close()
			}}
			{reservedIds}
		/>
	</svelte:fragment>
</Popover>
