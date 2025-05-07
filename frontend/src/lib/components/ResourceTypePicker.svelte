<script lang="ts">
	import { createEventDispatcher } from 'svelte'

	import { ResourceService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import IconedResourceType from './IconedResourceType.svelte'
	import { Button, ClearableInput } from './common'
	import Popover from './meltComponents/Popover.svelte'
	import { offset, flip, shift } from 'svelte-floating-ui/dom'
	import Label from './Label.svelte'
	import Tooltip from './Tooltip.svelte'
	import Badge from './common/badge/Badge.svelte'

	export let value: string | undefined
	export let notPickable = false
	export let nonePickable = false

	let resources: string[] = []

	async function loadResources() {
		resources = await ResourceService.listResourceTypeNames({ workspace: $workspaceStore! })
	}

	const dispatch = createEventDispatcher()

	function onClick(resource: string | undefined) {
		value = resource
		dispatch('click', resource)
	}

	$: if ($workspaceStore) {
		loadResources()
	}
	let search: string = ''

	$: filteredResources = resources.filter((r) => r.toLowerCase().includes(search.toLowerCase()))
</script>

<Label label="Resource type" class="w-full col-span-2">
	<svelte:fragment slot="header">
		<Tooltip light small>Select a resource type to narrow down the object type.</Tooltip>
	</svelte:fragment>

	<svelte:fragment slot="action">
		<div class="flex flex-row gap-1">
			<Button
				size="xs"
				color="light"
				on:click={() => onClick(undefined)}
				disabled={notPickable || value === undefined}
			>
				Clear
			</Button>
			<Popover
				floatingConfig={{
					strategy: 'fixed',
					placement: 'left-end',
					middleware: [offset(8), flip(), shift()]
				}}
				contentClasses="flex flex-col gap-2 h-full p-4 max-h-[40vh] w-[500px]"
			>
				<svelte:fragment slot="trigger">
					<Button nonCaptureEvent size="xs" color="dark">Select resource type</Button>
				</svelte:fragment>
				<svelte:fragment slot="content" let:close>
					<ClearableInput bind:value={search} placeholder="Search resource..." />

					<div class="overflow-y-scroll h-full">
						<div
							class="grid sm:grid-cols-2 md:grid-cols-3 gap-x-2 gap-y-1 items-center overflow-x-hidden"
						>
							{#if nonePickable && search === ''}
								{@const isPicked = value === undefined}
								<Button
									size="sm"
									variant="border"
									color={isPicked ? 'blue' : 'dark'}
									btnClasses={isPicked ? '!border-2' : 'm-[1px]'}
									disabled={notPickable}
									on:click={() => {
										onClick(undefined)
										close()
									}}
								>
									None
								</Button>
							{/if}
							{#each filteredResources as r}
								{@const isPicked = value === r}
								<Button
									size="sm"
									variant="border"
									color={isPicked ? 'blue' : 'light'}
									btnClasses={isPicked ? '!border-2' : 'm-[1px]'}
									disabled={notPickable}
									on:click={() => {
										onClick(r)
										close()
									}}
								>
									<IconedResourceType name={r} after={true} width="20px" height="20px" />
								</Button>
							{/each}

							{#if filteredResources.length === 0 && search !== ''}
								<div class="text-tertiary text-sm">No resources found</div>
							{/if}
						</div>
					</div>
				</svelte:fragment>
			</Popover>
		</div>
	</svelte:fragment>
	<div class="flex flex-row items-center w-full justify-between">
		<Badge color={!value ? 'gray' : 'blue'}>
			{value ?? 'None'}
		</Badge>
	</div>
</Label>
