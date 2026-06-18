<script lang="ts">
	import { createEventDispatcher } from 'svelte'
	import Button from '../common/button/Button.svelte'
	import { CheckCircle } from 'lucide-svelte'

	const dispatch = createEventDispatcher()
</script>

<div class="flex flex-row gap-2 justify-end w-full pt-6 pb-2">
	<Button
		size="xs"
		startIcon={{ icon: CheckCircle }}
		variant="default"
		btnClasses="font-normal"
		on:click={() => {
			dispatch('skipThis')
		}}
	>
		Mark this tutorial as completed
	</Button>
	<Button
		size="xs"
		startIcon={{ icon: CheckCircle }}
		btnClasses="font-normal"
		variant="default"
		on:click={() => {
			dispatch('skipAll')
		}}
	>
		Mark all tutorials as completed
	</Button>
</div>
