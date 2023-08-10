<script lang="ts">
	import { Button } from '../common'
	import { page } from '$app/stores'
	import { setQuery } from '$lib/navigation'
	import ToggleButton from '../common/toggleButton-v2/ToggleButton.svelte'
	import ToggleButtonGroup from '../common/toggleButton-v2/ToggleButtonGroup.svelte'
	import Tooltip from '../Tooltip.svelte'
	import { goto } from '$app/navigation'
	import { createEventDispatcher } from 'svelte'
	import AutoComplete from 'simple-svelte-autocomplete'
	import Toggle from '../Toggle.svelte'

	export let paths: string[] = []
	export let selectedPath: string | undefined = undefined
	export let success: boolean | undefined = undefined
	export let isSkipped: boolean | undefined = undefined
	export let autoRefresh: boolean = false

	export let jobKindsCat: string

	const dispatch = createEventDispatcher()
</script>

<div class="flex flex-col items-start gap-6 xl:gap-2 xl:flex-row mt-4 xl:mt-0">
	{#key selectedPath}
		<AutoComplete
			items={paths}
			value={selectedPath}
			bind:selectedItem={selectedPath}
			placeholder="Search by path"
			inputClassName="!h-[30px] py-1 !text-xs !w-64"
		/>
	{/key}
	<div class="hidden xl:block">
		<div class="flex flex-row gap-4 w-full">
			<div class="relative">
				<span class="text-xs absolute -top-4">Kind</span>
				<ToggleButtonGroup
					bind:selected={jobKindsCat}
					on:selected={(e) => {
						const url = new URL($page.url)

						debugger
						url.searchParams.set('job_kinds', e.detail)
						goto(url)
					}}
				>
					<ToggleButton value="all" label="All" />
					<ToggleButton value="runs" label="Runs" />
					<ToggleButton value="previews" label="Previews" />
					<ToggleButton value="dependencies" label="Dependencies" />
				</ToggleButtonGroup>
			</div>
			<div class="relative">
				<span class="text-xs absolute -top-4">Status</span>
				<ToggleButtonGroup
					bind:selected={success}
					on:selected={async () =>
						await setQuery($page.url, 'success', success === undefined ? success : String(success))}
				>
					<ToggleButton value={undefined} label="All" />
					<ToggleButton value={true} label="Success" class="whitespace-nowrap" />
					<ToggleButton value={false} label="Failure" class="whitespace-nowrap" />
				</ToggleButtonGroup>
			</div>
			<div class="relative">
				<span class="text-xs absolute -top-4">
					Flow
					<Tooltip light>Skipped flows are flows that did an early break</Tooltip></span
				>

				<ToggleButtonGroup
					bind:selected={isSkipped}
					on:selected={async () =>
						await setQuery(
							$page.url,
							'is_skipped',
							isSkipped === undefined ? isSkipped : String(isSkipped)
						)}
				>
					<ToggleButton value={undefined} label="All" class="whitespace-nowrap" />
					<ToggleButton value={false} label="Not skipped" class="whitespace-nowrap" />
					<ToggleButton value={true} label="Skipped" class="whitespace-nowrap" />
				</ToggleButtonGroup>
			</div>
		</div>
	</div>
	<div class="block xl:hidden">
		<div>
			<span class="text-xs">Kind</span>
			<ToggleButtonGroup
				bind:selected={jobKindsCat}
				on:selected={(e) => {
					const url = new URL($page.url)

					debugger
					url.searchParams.set('job_kinds', e.detail)
					goto(url)
				}}
			>
				<ToggleButton value="all" label="All" />
				<ToggleButton value="runs" label="Runs" />
				<ToggleButton value="previews" label="Previews" />
				<ToggleButton value="dependencies" label="Dependencies" />
			</ToggleButtonGroup>
		</div>
		<div>
			<span class="text-xs">Status</span>
			<ToggleButtonGroup
				bind:selected={success}
				on:selected={async () =>
					await setQuery($page.url, 'success', success === undefined ? success : String(success))}
			>
				<ToggleButton value={undefined} label="All" />
				<ToggleButton value={true} label="Success" class="whitespace-nowrap" />
				<ToggleButton value={false} label="Failure" class="whitespace-nowrap" />
			</ToggleButtonGroup>
		</div>
		<div>
			<span class="text-xs">
				Flow
				<Tooltip light>Skipped flows are flows that did an early break</Tooltip></span
			>

			<ToggleButtonGroup
				bind:selected={isSkipped}
				on:selected={async () =>
					await setQuery(
						$page.url,
						'is_skipped',
						isSkipped === undefined ? isSkipped : String(isSkipped)
					)}
			>
				<ToggleButton value={undefined} label="All" class="whitespace-nowrap" />
				<ToggleButton value={false} label="Not skipped" class="whitespace-nowrap" />
				<ToggleButton value={true} label="Skipped" class="whitespace-nowrap" />
			</ToggleButtonGroup>
		</div>
	</div>

	<Button
		color="light"
		variant="border"
		size="xs"
		on:click={() => {
			goto('/runs')
			dispatch('clearFilters')
		}}
	>
		Clear filters
	</Button>
	<Toggle
		size="xs"
		bind:checked={autoRefresh}
		options={{ right: 'Auto-refresh' }}
		textClass="whitespace-nowrap"
	/>
</div>
