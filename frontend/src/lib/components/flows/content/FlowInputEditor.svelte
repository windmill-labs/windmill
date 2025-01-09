<script lang="ts">
	import { createEventDispatcher, onDestroy } from 'svelte'
	import Section from '$lib/components/Section.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import { CornerDownLeft, Save } from 'lucide-svelte'

	export let name: string = ''
	export let disabled = false
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
			<div class="flex flex-row gap-2 data-schema-picker">
				<slot name="action" />
				<Button
					size="xs2"
					color="dark"
					{disabled}
					shortCut={{ Icon: CornerDownLeft, hide: false, withoutModifier: true }}
					startIcon={{ icon: Save }}
					on:click={applySchemaAndArgs}>Update schema</Button
				>
			</div>
		</svelte:fragment>
		<slot />
	</Section>
</div>
