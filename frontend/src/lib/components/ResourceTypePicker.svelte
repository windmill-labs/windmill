<script lang="ts">
	import { createEventDispatcher } from 'svelte'

	import { ResourceService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import IconedResourceType from './IconedResourceType.svelte'
	import { Button, ClearableInput } from './common'
	import Label from './Label.svelte'
	import Tooltip from './Tooltip.svelte'
	import Badge from './common/badge/Badge.svelte'
	import { untrack } from 'svelte'
	interface Props {
		value: string | undefined
		notPickable?: boolean
		nonePickable?: boolean
	}

	let { value = $bindable(), notPickable = false, nonePickable = false }: Props = $props()

	let resources: string[] = $state([])

	async function loadResources() {
		resources = await ResourceService.listResourceTypeNames({ workspace: $workspaceStore! })
	}

	const dispatch = createEventDispatcher()

	function onClick(resource: string | undefined) {
		value = resource
		dispatch('click', resource)
	}

	$effect(() => {
		if ($workspaceStore) {
			untrack(() => {
				loadResources()
			})
		}
	})
	let search: string = $state('')

	let filteredResources = $derived(
		resources.filter((r) => r.toLowerCase().includes(search.toLowerCase()))
	)
</script>

<Label label="Resource type" class="w-full mb-4">
	{#snippet header()}
		<Tooltip light small>Select a resource type to narrow down the object type.</Tooltip>

		<div class="flex flex-row items-center w-full justify-between">
			<Badge selected={!!value}>
				{value ?? 'None'}
			</Badge>
		</div>
	{/snippet}

	<div class="mt-2">
		<ClearableInput bind:value={search} placeholder="Search resource..." />

		<div class="overflow-y-scroll max-h-[330px] h-full mt-1">
			<div
				class="grid sm:grid-cols-2 md:grid-cols-3 gap-x-2 gap-y-1 items-center overflow-x-hidden"
			>
				{#if nonePickable && search === ''}
					{@const isPicked = value === undefined}
					<Button
						size="sm"
						variant="default"
						selected={isPicked}
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
						variant="default"
						selected={isPicked}
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
					<div class="text-primary text-sm">No resources found</div>
				{/if}
			</div>
		</div>
	</div>
</Label>
