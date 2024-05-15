<script lang="ts">
	import Popover from '$lib/components/Popover.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'

	export let type: string

	const tables = [
		'aggridcomponent',
		'aggridcomponentee',
		'dbexplorercomponent',
		'aggridinfinitecomponent',
		'aggridinfinitecomponentee'
	]

	const contextVariables: {
		label: string
		description: string
	}[] = []

	if (tables.includes(type)) {
		contextVariables.push({
			label: 'row',
			description: 'The current row of a table. Row is an object with keys index and value.'
		})
	} else if (type === 's3fileinputcomponent' || type === 'fileinputcomponent') {
		contextVariables.push({
			label: 'file',
			description: 'The current file being processed.'
		})
	} else if (type === 'containercomponent') {
		contextVariables.push({
			label: 'group',
			description: 'The group name of the container.'
		})
	} else if (type === 'listcomponent') {
		contextVariables.push({
			label: 'iter',
			description: 'The current iteration of the list. Iter is an object with keys index and value.'
		})
	}
</script>

{#if contextVariables.length > 0}
	<div class="my-2">
		<div class="text-xs font-semibold flex flex-row gap-1 items-center">
			<div class="mb-1"> Context </div>
			<Tooltip small light>
				Context are variables available in any expressions of config and runnable inputs that are
				specific to this particular component. Hover over the variables to see their descriptions.
			</Tooltip>
		</div>
		<div class="flex flex-row gap-1">
			{#each contextVariables as contextVariable}
				<Popover>
					<svelte:fragment slot="text">
						{contextVariable.description}
					</svelte:fragment>

					<span class="inline-flex items-center rounded-md px-2 py-0.5 text-xs font-medium border">
						{contextVariable.label}
					</span>
				</Popover>
			{/each}
		</div>
	</div>
{/if}
