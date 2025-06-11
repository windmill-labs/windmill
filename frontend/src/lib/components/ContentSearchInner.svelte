<script lang="ts">
	import { AppService, FlowService, ResourceService, ScriptService } from '$lib/gen'
	import { enterpriseLicense, workspaceStore } from '$lib/stores'
	import {
		ArrowDown,
		Boxes,
		Code2,
		Edit,
		ExternalLink,
		LayoutDashboard,
		Loader2
	} from 'lucide-svelte'
	import SearchItems from './SearchItems.svelte'
	import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'
	import FlowIcon from './home/FlowIcon.svelte'
	import { Alert, Button } from './common'
	import YAML from 'yaml'
	import { twMerge } from 'tailwind-merge'
	import ContentSearchInnerItem from './ContentSearchInnerItem.svelte'
	import { createEventDispatcher, untrack } from 'svelte'

	const dispatch = createEventDispatcher()
	interface Props {
		search?: string
	}

	let { search = $bindable('') }: Props = $props()

	export async function open(nsearch?: string) {
		await Promise.all([loadScripts(), loadResources(), loadApps(), loadFlows()])
		if (nsearch) {
			search = nsearch
		}
	}

	export async function loadScripts() {
		scripts = await ScriptService.listSearchScript({ workspace: $workspaceStore ?? '' })
	}

	export async function loadResources() {
		resources = await ResourceService.listSearchResource({ workspace: $workspaceStore ?? '' })
	}

	export async function loadApps() {
		apps = await AppService.listSearchApp({ workspace: $workspaceStore ?? '' })
	}

	export async function loadFlows() {
		flows = await FlowService.listSearchFlow({ workspace: $workspaceStore ?? '' })
	}

	let searchKind: 'all' | 'scripts' | 'flows' | 'apps' | 'resources' = $state('all')

	let scripts: undefined | { path: string; content: string }[] = $state(undefined)
	let filteredScriptItems: { path: string; content: string; marked: any }[] = $state([])

	let resources: undefined | { path: string; value: any }[] = $state(undefined)
	let filteredResourceItems: { path: string; value: any; marked: any }[] = $state([])

	let flows: undefined | { path: string; value: any }[] = $state(undefined)
	let filteredFlowItems: { path: string; value: any; marked: any }[] = $state([])

	let apps: undefined | { path: string; value: any }[] = $state(undefined)
	let filteredAppItems: { path: string; value: any; marked: any }[] = $state([])

	function getCounts(n: number) {
		return ` (${n})`
	}

	function escape(htmlStr) {
		return htmlStr
			.replace(/&/g, '&amp;')
			.replace(/</g, '&lt;')
			.replace(/>/g, '&gt;')
			.replace(/"/g, '&quot;')
			.replace(/'/g, '&#39;')
	}

	let showNbScripts = $state(10)
	let showNbApps = $state(10)
	let showNbResources = $state(10)
	let showNbFlows = $state(10)

	function resetShows() {
		showNbScripts = 10
		showNbApps = 10
		showNbResources = 10
		showNbFlows = 10
	}
	let counts = $derived(
		search == '' ||
			!scripts ||
			!resources ||
			!flows ||
			!apps ||
			!filteredAppItems ||
			!filteredFlowItems ||
			!filteredResourceItems ||
			!filteredScriptItems
			? {
					all: '',
					apps: '',
					flows: '',
					resources: '',
					scripts: ''
				}
			: {
					all: getCounts(
						filteredAppItems.length +
							filteredFlowItems.length +
							filteredResourceItems.length +
							filteredScriptItems.length
					),
					apps: getCounts(filteredAppItems.length),
					resources: getCounts(filteredResourceItems.length),
					flows: getCounts(filteredFlowItems.length),
					scripts: getCounts(filteredScriptItems.length)
				}
	)
	$effect(() => {
		search && untrack(() => resetShows())
	})
</script>

<SearchItems
	filter={search}
	items={scripts}
	f={(s) => {
		return escape(s.content)
	}}
	bind:filteredItems={filteredScriptItems}
/>

<SearchItems
	filter={search}
	items={resources}
	f={(s) => {
		return escape(YAML.stringify(s.value))
	}}
	bind:filteredItems={filteredResourceItems}
/>

<SearchItems
	filter={search}
	items={flows}
	f={(s) => {
		return escape(YAML.stringify(s.value, null, 4))
	}}
	bind:filteredItems={filteredFlowItems}
/>

<SearchItems
	filter={search}
	items={apps}
	f={(s) => {
		return escape(YAML.stringify(s.value, null, 4))
	}}
	bind:filteredItems={filteredAppItems}
/>

<div class="flex flex-col gap-2">
	<div class="flex gap-2 flex-wrap sticky top-0 left-0 right-0 bg-surface">
		<div class="p-2">
			<ToggleButtonGroup bind:selected={searchKind} className="h-10 ">
				{#snippet children({ item })}
					<ToggleButton small light value="all" label={'All' + counts.all} {item} />
					<ToggleButton
						small
						light
						value="scripts"
						icon={Code2}
						label={'Scripts' + counts.scripts}
						{item}
					/>
					<ToggleButton
						small
						light
						value="resources"
						icon={Boxes}
						label={'Resources' + counts.resources}
						{item}
					/>
					<ToggleButton
						small
						light
						value="flows"
						label={'Flows' + counts.flows}
						icon={FlowIcon}
						selectedColor="#14b8a6"
						{item}
					/>
					<ToggleButton
						small
						light
						value="apps"
						label={'Apps' + counts.apps}
						icon={LayoutDashboard}
						selectedColor="#fb923c"
						{item}
					/>
				{/snippet}
			</ToggleButtonGroup>
		</div>
	</div>
	<div class="px-2">
		<div class="text-xs text-secondary">
			Searching among <div class="inline-flex">
				{#if scripts}{scripts?.length}{:else}
					<Loader2 size={10} class="animate-spin " />
				{/if}
			</div>
			scripts,
			<div class="inline-flex">
				{#if resources}{resources?.length}{:else}
					<Loader2 size={10} class="animate-spin " />
				{/if}
			</div>
			resources,
			<div class="inline-flex">
				{#if flows}{flows?.length}{:else}
					<Loader2 size={10} class="animate-spin " />
				{/if}
			</div>
			flows,
			<div class="inline-flex">
				{#if apps}{apps?.length}{:else}
					<Loader2 size={10} class="animate-spin " />
				{/if}
			</div>
			apps
		</div>
	</div>

	<div class={twMerge('p-2')}>
		{#if !$enterpriseLicense}
			<div class="py-1"></div>

			<Alert title="Content Search is an EE feature" type="warning">
				Without EE, content search will only search among 10 scripts, 3 flows, 3 apps and 3
				resources.
			</Alert>
			<div class="py-1"></div>
		{/if}

		{#if search.trim().length > 0}
			<div class="flex flex-col gap-4">
				{#if (searchKind == 'all' || searchKind == 'scripts') && filteredScriptItems?.length > 0}
					{#each filteredScriptItems.slice(0, showNbScripts) ?? [] as item}
						<ContentSearchInnerItem
							title={`Script: ${item.path}`}
							href={`/scripts/get/${item.path}`}
							on:close
						>
							{#snippet actions()}
								<Button
									href={`/scripts/get/${item.path}`}
									color="light"
									size="xs"
									startIcon={{ icon: ExternalLink }}
									on:click={() => {
										dispatch('close')
									}}
								>
									Open
								</Button>

								<Button
									href={`/scripts/edit/${item.path}?no_draft=true`}
									target="_blank"
									color="light"
									size="xs"
									startIcon={{ icon: Edit }}
								>
									Edit
								</Button>
							{/snippet}
							<pre class="text-xs border rounded-md p-2 overflow-auto max-h-40 w-full"
								><code>{@html item.marked}</code>
							</pre>
						</ContentSearchInnerItem>
					{/each}
					{#if filteredScriptItems.length > showNbScripts}
						<Button
							color="light"
							size="xs"
							on:click={() => {
								showNbScripts += 30
							}}
							startIcon={{
								icon: ArrowDown
							}}
						>
							Show more scripts ({showNbScripts} of {filteredScriptItems.length})
						</Button>
					{/if}
				{/if}
				{#if (searchKind == 'all' || searchKind == 'resources') && filteredResourceItems?.length > 0}
					{#each filteredResourceItems.slice(0, showNbResources) ?? [] as item}
						<ContentSearchInnerItem
							title={`Resource: ${item.path}`}
							href={`/resources#${item.path}`}
							on:close
						>
							{#snippet actions()}
								<Button
									href={`/resources#${item.path}`}
									color="light"
									target="_blank"
									size="xs"
									startIcon={{ icon: Edit }}
								>
									Edit
								</Button>
							{/snippet}
							<pre class="text-xs border rounded-md p-2 overflow-auto max-h-40 w-full"
								><code>{@html item.marked}</code></pre
							>
						</ContentSearchInnerItem>
					{/each}
					{#if filteredResourceItems.length > showNbResources}
						<Button
							color="light"
							size="xs"
							on:click={() => {
								showNbResources += 30
							}}
							startIcon={{
								icon: ArrowDown
							}}
						>
							Show more resources ({showNbResources} of {filteredResourceItems.length})
						</Button>
					{/if}
				{/if}
				{#if (searchKind == 'all' || searchKind == 'flows') && filteredFlowItems?.length > 0}
					{#each filteredFlowItems.slice(0, showNbFlows) ?? [] as item}
						<ContentSearchInnerItem
							title={`Flow: ${item.path}`}
							href={`/flows/get/${item.path}`}
							on:close
						>
							{#snippet actions()}
								<Button
									href={`/flows/get/${item.path}`}
									color="light"
									size="xs"
									startIcon={{ icon: ExternalLink }}
									on:click={() => {
										dispatch('close')
									}}
								>
									Open
								</Button>
								<Button
									href={`/flows/edit/${item.path}?no_draft=true`}
									color="light"
									target="_blank"
									size="xs"
									startIcon={{ icon: Edit }}
								>
									Edit
								</Button>
							{/snippet}

							<pre class="text-xs border p-2 overflow-auto max-h-40 w-full"
								><code>{@html item.marked}</code></pre
							>
						</ContentSearchInnerItem>
					{/each}
					{#if filteredFlowItems.length > showNbFlows}
						<Button
							color="light"
							size="xs"
							on:click={() => {
								showNbFlows += 30
							}}
							startIcon={{
								icon: ArrowDown
							}}
						>
							Show more flows ({showNbFlows} of {filteredFlowItems.length})
						</Button>
					{/if}
				{/if}
				{#if (searchKind == 'all' || searchKind == 'apps') && filteredAppItems?.length > 0}
					{#each filteredAppItems.slice(0, showNbApps) ?? [] as item}
						<ContentSearchInnerItem
							title={`App: ${item.path}`}
							href={`/apps/get/${item.path}`}
							on:close
						>
							{#snippet actions()}
								<Button
									href={`/apps/get/${item.path}`}
									color="light"
									size="xs"
									startIcon={{ icon: ExternalLink }}
									on:click={() => {
										dispatch('close')
									}}
								>
									Open
								</Button>
								<Button
									href={`/apps/edit/${item.path}?no_draft=true`}
									color="light"
									target="_blank"
									size="xs"
									startIcon={{ icon: Edit }}
								>
									Edit
								</Button>
							{/snippet}

							<pre class="text-xs border p-2 overflow-auto max-h-40 w-full"
								><code>{@html item.marked}</code></pre
							>
						</ContentSearchInnerItem>
					{/each}
					{#if filteredAppItems.length > showNbApps}
						<Button
							color="light"
							size="xs"
							on:click={() => {
								showNbApps += 30
							}}
							startIcon={{
								icon: ArrowDown
							}}
						>
							Show more apps ({showNbApps} of {filteredAppItems.length})
						</Button>
					{/if}
				{/if}
			</div>
		{:else}
			<div class="flex justify-center items-center h-48">
				<div class="text-tertiary text-center">
					<div class="text-2xl font-bold">Empty Search Filter</div>
					<div class="text-sm"
						>Start writing, search everywhere a path is referenced for instance</div
					>
				</div>
			</div>
		{/if}
	</div>
</div>
