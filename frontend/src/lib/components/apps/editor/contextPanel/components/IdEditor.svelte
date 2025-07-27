<script lang="ts">
	import type { AppViewerContext } from '$lib/components/apps/types'
	import { allItems } from '$lib/components/apps/utils'
	import { Pencil } from 'lucide-svelte'
	import { getContext } from 'svelte'
	import IdEditorInput from '$lib/components/IdEditorInput.svelte'
	import Popover from '$lib/components/meltComponents/Popover.svelte'

	const { app, selectedComponent } = getContext<AppViewerContext>('AppViewerContext')

	interface Props {
		id: string
		onChange: ({ oldId, newId }: { oldId: string; newId: string }) => void
		onClose?: () => void
	}

	let { id, onChange, onClose }: Props = $props()

	let reservedIds = $derived(allItems($app.grid, $app.subgrids).map((item) => item.id))
</script>

<Popover
	floatingConfig={{ strategy: 'absolute', placement: 'bottom-start' }}
	closeOnOtherPopoverOpen
	contentClasses="p-4"
>
	{#snippet trigger()}
		<button
			onclick={() => {
				$selectedComponent = [id]
			}}
			title="Edit ID"
			class="flex items-center px-1 rounded-sm bg-surface-secondary hover:text-primary text-secondary h-5"
			aria-label="Open component ID editor"
		>
			<Pencil size={14} />
		</button>
	{/snippet}
	{#snippet content({ close })}
		<IdEditorInput
			initialId={id}
			{onClose}
			onSave={(e) => {
				onChange(e)
				onClose?.()
			}}
			{reservedIds}
		/>
	{/snippet}
</Popover>
