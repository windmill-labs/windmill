<script lang="ts">
	import { createBubbler, stopPropagation } from 'svelte/legacy'

	const bubble = createBubbler()
	import { createEventDispatcher, onMount } from 'svelte'
	import SearchItems from '$lib/components/SearchItems.svelte'
	import NoItemFound from '$lib/components/home/NoItemFound.svelte'
	import RowIcon from '$lib/components/common/table/RowIcon.svelte'
	import { type Script, ScriptService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { emptyString } from '$lib/utils'
	import { Skeleton } from '$lib/components/common'

	interface Props {
		filter?: string
		children?: import('svelte').Snippet
	}

	let { filter = $bindable(''), children }: Props = $props()

	let scripts: Script[] | undefined = $state(undefined)
	let filteredItems: (Script & { marked?: string })[] = $state([])
	let prefilteredItems = $derived(scripts ?? [])

	const dispatch = createEventDispatcher()

	async function loadScripts(): Promise<void> {
		const loadedScripts = await ScriptService.listScripts({
			workspace: $workspaceStore!,
			perPage: 300
		})

		scripts = loadedScripts
	}

	onMount(() => {
		loadScripts()
	})
</script>

<SearchItems
	{filter}
	items={prefilteredItems}
	bind:filteredItems
	f={(x) => (emptyString(x.summary) ? x.path : x.summary + ' (' + x.path + ')')}
/>
<div class="w-full flex mt-1 items-center gap-2">
	{@render children?.()}
	<input
		type="text"
		onkeydown={stopPropagation(bubble('keydown'))}
		placeholder="Search workspace scripts"
		bind:value={filter}
		class="text-2xl grow mb-4"
	/>
</div>

{#if scripts}
	{#if filteredItems.length == 0}
		<NoItemFound />
	{:else}
		<ul class="divide-y border rounded-md">
			{#each filteredItems as item (item)}
				<li class="flex flex-row w-full">
					<button
						class="p-4 gap-4 flex flex-row grow justify-between hover:bg-surface-hover bg-surface transition-all items-center rounded-md"
						onclick={() => dispatch('pick', item.path)}
					>
						<div class="flex items-center gap-4">
							<RowIcon kind="script" />

							<div class="w-full text-left font-normal">
								<div class="text-primary flex-wrap text-md font-semibold mb-1">
									{#if item.marked}
										{@html item.marked ?? ''}
									{:else}
										{!item.summary || item.summary.length == 0 ? item.path : item.summary}
									{/if}
								</div>
								<div class="text-tertiary text-xs">
									{item.path}
								</div>
							</div>
						</div>
					</button>
				</li>
			{/each}
		</ul>
	{/if}
{:else}
	{#each Array(10).fill(0) as _}
		<Skeleton layout={[[4], 0.5]} />
	{/each}
{/if}
