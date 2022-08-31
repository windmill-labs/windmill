<script lang="ts">
	import { faClose } from '@fortawesome/free-solid-svg-icons'
	import { createEventDispatcher } from 'svelte'
	import Icon from 'svelte-awesome'

	export let title: string | undefined = undefined
	const dispatch = createEventDispatcher()
</script>

<div class="divide-y h-screen">
	<div class="flex justify-between items-center py-2 px-4">
		<span class="text-sm font-bold">{title}</span>
		<button on:click={() => dispatch('close')}>
			<Icon data={faClose} class="w-4 h-4" />
		</button>
	</div>

	<div class="p-2 h-full">
		<slot />
	</div>
</div>
