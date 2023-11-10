<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'

	import { Popup } from '$lib/components/common'
	import { Pen } from 'lucide-svelte'
	import { createEventDispatcher } from 'svelte'

	export let kind: string
	export let row: {
		name: string
		path: string
	}

	let editedName = row.name

	const dispatch = createEventDispatcher()
	function onkeydown(e) {
		if (e.key === 'Enter') {
			dispatch('update', { path: row.path, name: editedName })
		}
	}
</script>

<Popup
	floatingConfig={{ strategy: 'absolute', placement: 'bottom-end' }}
	containerClasses="border rounded-lg shadow-lg p-4 bg-surface"
	let:close
>
	<svelte:fragment slot="button">
		<Button color="light" size="xs2" nonCaptureEvent={true}>
			<div class="flex flex-row gap-1 items-center">
				<Pen size={16} />
			</div>
		</Button>
	</svelte:fragment>
	<div class="flex flex-col w-80 gap-2">
		<div class="leading-6 font-semibold text-xs">Edit {kind} name</div>
		<div class="flex flex-row gap-2">
			<input on:keydown={onkeydown} bind:value={editedName} />
			<Button
				color="dark"
				size="xs"
				on:click={() => dispatch('update', { path: row.path, name: editedName }) && close(null)}
			>
				Update
			</Button>
		</div>
	</div>
</Popup>
