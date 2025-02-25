<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
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

<Popover floatingConfig={{ strategy: 'absolute', placement: 'bottom-end' }}>
	<svelte:fragment slot="trigger">
		<Button color="light" size="xs2" nonCaptureEvent={true}>
			<div class="flex flex-row gap-1 items-center">
				<Pen size={16} />
			</div>
		</Button>
	</svelte:fragment>
	<svelte:fragment slot="content" let:close>
		<div class="flex flex-col w-80 gap-2 p-4">
			<div class="leading-6 font-semibold text-xs">Edit {kind} name</div>
			<div class="flex flex-row gap-2">
				<input on:keydown={onkeydown} bind:value={editedName} />
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
	</svelte:fragment>
</Popover>
