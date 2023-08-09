<script lang="ts">
	import { Filter } from 'lucide-svelte'
	import { Button, Popup } from '../common'
	import { page } from '$app/stores'
	import { setQuery } from '$lib/navigation'
	import ToggleButton from '../common/toggleButton-v2/ToggleButton.svelte'
	import ToggleButtonGroup from '../common/toggleButton-v2/ToggleButtonGroup.svelte'
	import Tooltip from '../Tooltip.svelte'
	import JsonEditor from '../apps/editor/settingsPanel/inputEditor/JsonEditor.svelte'

	export let paths: string[] = []
	export let selectedPath: string | undefined = undefined
	export let success: boolean | undefined = undefined
	export let isSkipped: boolean | undefined = undefined

	export let argFilter: string
	export let argError: string
	export let resultFilter: string
	export let resultError: string
</script>

<div class="flex flex-col items-center gap-6 xl:gap-2 xl:flex-row mt-4 xl:mt-0">
	<div class="relative">
		<span class="text-xs absolute -top-4">Status</span>
		<ToggleButtonGroup
			bind:selected={success}
			on:selected={async () => await setQuery($page.url, 'success', String(success))}
		>
			<ToggleButton value={undefined} label="All" />
			<ToggleButton value={true} label="Only success" class="whitespace-nowrap" />
			<ToggleButton value={false} label="Only errors" class="whitespace-nowrap" />
		</ToggleButtonGroup>
	</div>
	<div class="relative">
		<span class="text-xs absolute -top-4">
			Flow
			<Tooltip light>Skipped flows are flows that did an early break</Tooltip></span
		>

		<ToggleButtonGroup
			bind:selected={isSkipped}
			on:selected={async () => await setQuery($page.url, 'is_skipped', String(isSkipped))}
		>
			<ToggleButton value={false} label="Skipped" />
			<ToggleButton value={true} label="Not skipped" class="whitespace-nowrap" />
			<ToggleButton value={undefined} label="Show all" class="whitespace-nowrap" />
		</ToggleButtonGroup>
	</div>

	<div class="flex gap-1 relative w-full">
		<span class="text-xs absolute -top-4">Select by path</span>
		<select bind:value={selectedPath}>
			{#if paths}
				{#each paths as path}
					<option>{path}</option>
				{/each}
			{/if}
		</select>
	</div>

	<Popup
		floatingConfig={{ strategy: 'absolute', placement: 'bottom-end' }}
		containerClasses="border rounded-lg shadow-lg p-4 bg-surface"
		let:close
	>
		<svelte:fragment slot="button">
			<Button color="dark" size="xs" nonCaptureEvent={true}>
				<div class="flex flex-row gap-1 items-center">
					<Filter size={16} />
					Filter by arguments
				</div>
			</Button>
		</svelte:fragment>
		<div class="flex flex-col w-72 p-2 gap-2">
			<span class="text-sm eading-6 font-semibold">Filter by arguments</span>
			<span class="text-xs leading-6">
				{`Filter by a json being a subset of the args. Try '\{"foo": "bar"\}`}</span
			>

			<JsonEditor on:change bind:error={argError} bind:code={argFilter} />
			<Button
				size="xs"
				color="dark"
				on:click={() => {
					close(null)
					argFilter = ''
				}}
			>
				Clear filter
			</Button>
		</div>
	</Popup>
	<Popup
		floatingConfig={{ strategy: 'absolute', placement: 'bottom-end' }}
		containerClasses="border rounded-lg shadow-lg p-4 bg-surface"
		let:close
	>
		<svelte:fragment slot="button">
			<Button color="dark" size="xs" nonCaptureEvent={true}>
				<div class="flex flex-row gap-1 items-center">
					<Filter size={16} />
					Filter by result
				</div>
			</Button>
		</svelte:fragment>
		<div class="flex flex-col w-72 p-2 gap-2">
			<span class="text-sm leading-6 font-semibold">Filter by result</span>
			<span class="text-xs leading-6">
				{`Filter by a json being a subset of the args. Try '\{"foo": "bar"\}`}
			</span>

			<JsonEditor on:change bind:error={resultError} bind:code={resultFilter} />
			<Button
				size="xs"
				color="dark"
				on:click={() => {
					close(null)
					resultFilter = ''
				}}
			>
				Clear filter
			</Button>
		</div>
	</Popup>
	<Button
		color="dark"
		size="xs"
		on:click={() => {
			selectedPath = undefined
			success = undefined
			isSkipped = false
			argFilter = ''
			resultFilter = ''
			argError = ''
			resultError = ''
		}}
	>
		Clear filters
	</Button>
</div>
