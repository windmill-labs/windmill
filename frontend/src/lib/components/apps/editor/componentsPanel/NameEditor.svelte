<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import { Pen } from 'lucide-svelte'
	import { createEventDispatcher } from 'svelte'

	interface Props {
		kind: string
		row: {
			name: string
			path: string
		}
	}

	let { kind, row }: Props = $props()

	let editedName = $state(row.name)

	const dispatch = createEventDispatcher()
	function onkeydown(e) {
		if (e.key === 'Enter') {
			dispatch('update', { path: row.path, name: editedName })
		}
	}
</script>

<Popover floatingConfig={{ strategy: 'absolute', placement: 'bottom-end' }}>
	{#snippet trigger()}
		<Button color="light" size="xs2" nonCaptureEvent={true}>
			<div class="flex flex-row gap-1 items-center">
				<Pen size={16} />
			</div>
		</Button>
	{/snippet}
	{#snippet content({ close })}
		<div class="flex flex-col w-80 gap-2 p-4">
			<div class="leading-6 font-semibold text-xs">Edit {kind} name</div>
			<div class="flex flex-row gap-2">
				<input {onkeydown} bind:value={editedName} />
				<Button
					color="dark"
					size="xs"
					on:click={() => {
						dispatch('update', { path: row.path, name: editedName })
						close()
					}}
				>
					Update
				</Button>
			</div>
		</div>
	{/snippet}
</Popover>
