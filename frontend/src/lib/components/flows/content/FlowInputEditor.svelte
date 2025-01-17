<script lang="ts">
	import { createEventDispatcher, onDestroy } from 'svelte'

	const dispatch = createEventDispatcher()

	onDestroy(() => {
		dispatch('destroy')
	})
</script>

<div class="flex flex-col h-full p-2">
	<div class="items-center flex flex-row justify-end gap-2 data-schema-picker min-h-[30px]">
		<slot name="action" />
	</div>

	<div class="w-full min-h-0 flex-grow h-full">
		<slot />
	</div>
</div>
