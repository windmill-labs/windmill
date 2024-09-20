<script lang="ts">
	import type { AppViewerContext } from '$lib/components/apps/types'
	import { allItems } from '$lib/components/apps/utils'
	import { Pencil } from 'lucide-svelte'
	import { createEventDispatcher, getContext } from 'svelte'
	import IdEditorInput from '$lib/components/IdEditorInput.svelte'
	import { Popup } from '$lib/components/common'

	const { app, selectedComponent } = getContext<AppViewerContext>('AppViewerContext')

	export let id: string

	const dispatch = createEventDispatcher()

	$: reservedIds = allItems($app.grid, $app.subgrids).map((item) => item.id)
</script>

<Popup let:close floatingConfig={{ strategy: 'absolute', placement: 'bottom-start' }}>
	<svelte:fragment slot="button">
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
	<IdEditorInput
		initialId={id}
		on:close={() => close(null)}
		on:save={(e) => {
			dispatch('save', e.detail)
			close(null)
		}}
		{reservedIds}
	/>
</Popup>
