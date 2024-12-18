<script lang="ts">
	import { createEventDispatcher } from 'svelte'
	import Section from '$lib/components/Section.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import { CornerDownLeft } from 'lucide-svelte'

	export let name: string
	export let disabled = false

	const dispatch = createEventDispatcher()

	const dropdownItems: Array<{
		label: string
		onClick: () => void
	}> = [
		{
			label: 'Apply schema only',
			onClick: () => {
				applySchema()
			}
		}
	]

	function applySchema() {
		dispatch('applySchema')
	}

	function applySchemaAndArgs() {
		dispatch('applySchemaAndArgs')
	}
</script>

<svelte:window
	on:keydown={(e) => {
		if (e.key === 'Enter') {
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
			<Button
				size="xs2"
				color="dark"
				{disabled}
				{dropdownItems}
				shortCut={{ Icon: CornerDownLeft, hide: false, withoutModifier: true }}
				on:click={applySchemaAndArgs}>Apply</Button
			>
		</svelte:fragment>
		<slot />
	</Section>
</div>
