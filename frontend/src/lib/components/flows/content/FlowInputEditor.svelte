<script lang="ts">
	import { createEventDispatcher, onDestroy } from 'svelte'
	import Section from '$lib/components/Section.svelte'

	export let name: string = ''
	export let preventEnter = false

	const dispatch = createEventDispatcher()

	function applySchemaAndArgs() {
		dispatch('applySchemaAndArgs')
	}

	onDestroy(() => {
		dispatch('destroy')
	})
</script>

<svelte:window
	on:keydown={(e) => {
		if (e.key === 'Enter' && !preventEnter) {
			applySchemaAndArgs()
			e.preventDefault()
		}
	}}
/>

<div class="h-full p-2">
	<Section label={name} class="h-full" small={true}>
		<svelte:fragment slot="header">
			<slot name="header" />
		</svelte:fragment>
		<svelte:fragment slot="action">
			<div class="flex flex-row gap-2 data-schema-picker min-h-[22px]">
				<slot name="action" />
			</div>
		</svelte:fragment>
		<slot />
	</Section>
</div>
