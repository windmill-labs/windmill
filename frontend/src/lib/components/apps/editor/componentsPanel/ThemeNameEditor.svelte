<script lang="ts">
	import { updateTheme } from './themeUtils'
	import { workspaceStore } from '$lib/stores'
	import Button from '$lib/components/common/button/Button.svelte'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import { Pen } from 'lucide-svelte'
	import { createEventDispatcher } from 'svelte'
	import { sendUserToast } from '$lib/toast'

	interface Props {
		row: {
			name: string
			path: string
		}
	}

	let { row }: Props = $props()

	let editedName = $state(row.name)

	const dispatch = createEventDispatcher()
</script>

<Popover
	floatingConfig={{ strategy: 'absolute', placement: 'bottom-end' }}
	closeOnOtherPopoverOpen
	contentClasses="flex flex-col w-80 gap-2 p-4"
>
	{#snippet trigger()}
		<Button color="light" size="xs2" nonCaptureEvent={true}>
			<div class="flex flex-row gap-1 items-center">
				<Pen size={16} />
			</div>
		</Button>
	{/snippet}
	{#snippet content({ close })}
		<div class="leading-6 font-semibold text-xs">Edit theme name</div>
		<div class="flex flex-row gap-2">
			<input bind:value={editedName} />
			<Button
				color="dark"
				size="xs"
				on:click={async () => {
					if (!$workspaceStore) return
					await updateTheme($workspaceStore, row.path, {
						value: {
							...row,
							name: editedName
						}
					})
					dispatch('reloadThemes')
					close()
					sendUserToast('Theme name updated:\n' + editedName)
				}}
			>
				Update
			</Button>
		</div>
	{/snippet}
</Popover>
