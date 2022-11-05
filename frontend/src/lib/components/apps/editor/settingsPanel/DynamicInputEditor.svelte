<script lang="ts">
	import type { DynamicInput } from '../../types'
	import { Alert, Badge, Button } from '$lib/components/common'
	import { faLink } from '@fortawesome/free-solid-svg-icons'

	export let input: DynamicInput
</script>

{#if input.id && input.name}
	<div class="flex justify-between w-full">
		<span class="text-xs font-bold">Component</span>
		<Badge color="indigo">{input.id}</Badge>
	</div>
	<div class="flex justify-between w-full">
		<span class="text-xs font-bold">Field name</span>
		<Badge color="green">{input.name}</Badge>
	</div>
{:else}
	<div class="flex justify-between w-full">
		<span class="text-xs font-bold">Status</span>
		<Badge color="dark-yellow">Not connected</Badge>
	</div>
	<Button
		size="xs"
		startIcon={{ icon: faLink }}
		color="dark"
		on:click={() => {
			alert('todo')
		}}
	>
		Connect this input to an ouput
	</Button>
{/if}
