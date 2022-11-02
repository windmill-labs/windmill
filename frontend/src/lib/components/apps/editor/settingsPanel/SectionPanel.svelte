<script lang="ts">
	import { ToggleButton, ToggleButtonGroup } from '$lib/components/common'
	import Badge from '$lib/components/common/badge/Badge.svelte'

	import { createEventDispatcher } from 'svelte'
	import type { AppSection } from '../../types'

	export let section: AppSection | undefined

	const dispatch = createEventDispatcher()

	function deleteSection() {
		dispatch('remove')
	}
</script>

{#if section}
	<div class="p-2 flex flex-col gap-2 items-start">
		<Badge color="indigo">{section.id}</Badge>

		<div class="text-sm font-bold">Columns</div>
		<ToggleButtonGroup bind:selected={section.columns}>
			<ToggleButton position="left" value={1}>1</ToggleButton>
			<ToggleButton position="center" value={2}>2</ToggleButton>
			<ToggleButton position="right" value={3}>3</ToggleButton>
		</ToggleButtonGroup>
	</div>
{/if}
