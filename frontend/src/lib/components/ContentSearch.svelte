<script lang="ts">
	import { AppService, FlowService, ResourceService, ScriptService } from '$lib/gen'
	import { enterpriseLicense, workspaceStore } from '$lib/stores'
	import { clickOutside } from '$lib/utils'
	import { Boxes, Code2, Edit, LayoutDashboard, Loader2, X } from 'lucide-svelte'
	import Portal from 'svelte-portal'
	import { twMerge } from 'tailwind-merge'
	import SearchItems from './SearchItems.svelte'
	import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'
	import FlowIcon from './home/FlowIcon.svelte'
	import { Alert, Button } from './common'
	import { goto } from '$app/navigation'

	let search: string = ''

	export function open() {
		isOpen = true
		loadScripts()
		loadResources()
		loadApps()
		loadFlows()
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

	let isOpen = false

	let scripts: undefined | { path: string; content: string }[] = undefined
	let filteredScriptItems: { path: string; content: string; marked: any }[] = []

	let resources: undefined | { path: string; value: any }[] = undefined
	let filteredResourceItems: { path: string; value: any; marked: any }[] = []

	let flows: undefined | { path: string; value: any }[] = undefined
	let filteredFlowItems: { path: string; value: any; marked: any }[] = []

	let apps: undefined | { path: string; value: any }[] = undefined
	let filteredAppItems: { path: string; value: any; marked: any }[] = []

	let searchKind: 'all' | 'scripts' | 'flows' | 'apps' | 'resources' = 'all'

	function getCounts(n: number) {
		return ` (${n})`
	}
	$: counts =
		search == '' || !scripts || !resources || !flows || !apps
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
</script>

<SearchItems
	filter={search}
	items={scripts}
	f={(s) => {
		return s.content
	}}
	bind:filteredItems={filteredScriptItems}
/>

<SearchItems
	filter={search}
	items={resources}
	f={(s) => {
		return JSON.stringify(s.value, null, 4)
	}}
	bind:filteredItems={filteredResourceItems}
/>

<SearchItems
	filter={search}
	items={flows}
	f={(s) => {
		return JSON.stringify(s.value, null, 4)
	}}
	bind:filteredItems={filteredFlowItems}
/>

<SearchItems
	filter={search}
	items={apps}
	f={(s) => {
		return JSON.stringify(s.value, null, 4)
	}}
	bind:filteredItems={filteredAppItems}
/>

{#if isOpen}
	<Portal>
		<div
			class={twMerge(
				`fixed top-0 bottom-0 left-0 right-0 transition-all duration-50`,
				' bg-black bg-opacity-60',
				'z-[1100]'
			)}
		>
			<div
				class={'max-w-4xl lg:mx-auto mx-10 mt-8 bg-surface rounded-lg relative'}
				use:clickOutside={false}
				on:click_outside={() => {
					isOpen = false
				}}
			>
				<div class="px-4 py-2 border-b flex justify-between items-center">
					<div>Search by content</div>
					<div class="w-8">
						<button
							on:click|stopPropagation={() => {
								isOpen = false
							}}
							class="hover:bg-surface-hover bg-surface-secondary rounded-full w-8 h-8 flex items-center justify-center transition-all"
						>
							<X class="text-tertiary" />
						</button>
					</div>
				</div>
				<div class="px-2 py-2 overflow-auto">
					<div class="flex gap-2 flex-wrap">
						<div class="flex justify-start">
							<ToggleButtonGroup bind:selected={searchKind} class="h-10">
								<ToggleButton small light value="all" label={'All' + counts.all} />
								<ToggleButton
									small
									light
									value="scripts"
									icon={Code2}
									label={'Scripts' + counts.scripts}
								/>
								<ToggleButton
									small
									light
									value="resources"
									icon={Boxes}
									label={'Resources' + counts.resources}
								/>
								<ToggleButton
									small
									light
									value="flows"
									label={'Flows' + counts.flows}
									icon={FlowIcon}
									selectedColor="#14b8a6"
								/>
								<ToggleButton
									small
									light
									value="apps"
									label={'Apps' + counts.apps}
									icon={LayoutDashboard}
									selectedColor="#fb923c"
								/>
							</ToggleButtonGroup>
						</div>
						<div class="relative text-tertiary grow min-w-[100px]">
							<!-- svelte-ignore a11y-autofocus -->
							<input
								autofocus
								placeholder={'Search in the content of resources, scripts, flows and apps'}
								bind:value={search}
								class="bg-surface !h-10 !px-4 !pr-10 !rounded-lg text-sm focus:outline-none"
							/>
							<button type="submit" class="absolute right-0 top-0 mt-3 mr-4">
								<svg
									class="h-4 w-4 fill-current"
									xmlns="http://www.w3.org/2000/svg"
									xmlns:xlink="http://www.w3.org/1999/xlink"
									version="1.1"
									id="Capa_1"
									x="0px"
									y="0px"
									viewBox="0 0 56.966 56.966"
									style="enable-background:new 0 0 56.966 56.966;"
									xml:space="preserve"
									width="512px"
									height="512px"
								>
									<path
										d="M55.146,51.887L41.588,37.786c3.486-4.144,5.396-9.358,5.396-14.786c0-12.682-10.318-23-23-23s-23,10.318-23,23  s10.318,23,23,23c4.761,0,9.298-1.436,13.177-4.162l13.661,14.208c0.571,0.593,1.339,0.92,2.162,0.92  c0.779,0,1.518-0.297,2.079-0.837C56.255,54.982,56.293,53.08,55.146,51.887z M23.984,6c9.374,0,17,7.626,17,17s-7.626,17-17,17  s-17-7.626-17-17S14.61,6,23.984,6z"
									/>
								</svg>
							</button>
						</div>
					</div>
					<div class="mt-1">
						<div class="text-xs text-secondary"
							>Searching among <div class="inline-flex"
								>{#if scripts}{scripts?.length}{:else}
									<Loader2 size={10} class="animate-spin " />
								{/if}</div
							>
							scripts,
							<div class="inline-flex"
								>{#if resources}{resources?.length}{:else}
									<Loader2 size={10} class="animate-spin " />
								{/if}</div
							>
							resources,
							<div class="inline-flex"
								>{#if flows}{flows?.length}{:else}
									<Loader2 size={10} class="animate-spin " />
								{/if}</div
							>
							flows,
							<div class="inline-flex"
								>{#if apps}{apps?.length}{:else}
									<Loader2 size={10} class="animate-spin " />
								{/if}</div
							>
							apps</div
						>
					</div>

					<div class="mt-1 overflow-auto max-h-[80vh]">
						{#if !$enterpriseLicense}
							<Alert title="Content Search is an EE feature" type="warning">
								Without EE, content search will only search among 10 scripts, 3 flows, 3 apps and 3
								resources.
							</Alert>
							<div class="py-1" />
						{/if}

						{#if search.length > 0}
							<div class="flex flex-col gap-4">
								{#if (searchKind == 'all' || searchKind == 'scripts') && filteredScriptItems.length > 0}
									{#each filteredScriptItems as item}
										<div>
											<div class="text-sm font-semibold"
												><a href="/scripts/get/{item.path}">Script: {item.path}</a></div
											>
											<div class="flex gap-2 justify-between">
												<pre class="text-xs border p-2 overflow-auto max-h-40 w-full max-w-2xl"
													><code>{@html item.marked}</code></pre
												>
												<div>
													<div class="flex gap-2">
														<Button
															on:click|once={() => {
																goto(`/scripts/edit/${item.path}?no_draft=true`)
															}}
															color="light"
															size="sm"
															startIcon={{ icon: Edit }}
														>
															Edit
														</Button>
													</div>
												</div>
											</div>
										</div>
									{/each}
								{/if}
								{#if (searchKind == 'all' || searchKind == 'resources') && filteredResourceItems.length > 0}
									{#each filteredResourceItems as item}
										<div>
											<div class="text-sm font-semibold">Resource: {item.path}</div>
											<div class="flex gap-2 justify-between">
												<pre class="text-xs border p-2 overflow-auto max-h-40 w-full max-w-2xl"
													><code>{@html item.marked}</code></pre
												>
											</div>
										</div>
									{/each}
								{/if}
								{#if (searchKind == 'all' || searchKind == 'flows') && filteredFlowItems.length > 0}
									{#each filteredFlowItems as item}
										<div>
											<div class="text-sm font-semibold"
												><a href="/flows/get/{item.path}">Flow: {item.path}</a></div
											>
											<div class="flex gap-2 justify-between">
												<pre class="text-xs border p-2 overflow-auto max-h-40 w-full max-w-2xl"
													><code>{@html item.marked}</code></pre
												>
												<div>
													<div class="flex gap-2">
														<Button
															on:click|once={() => {
																goto(`/flows/edit/${item.path}?no_draft=true`)
															}}
															color="light"
															size="sm"
															startIcon={{ icon: Edit }}>Edit</Button
														>
													</div>
												</div>
											</div>
										</div>
									{/each}
								{/if}
								{#if (searchKind == 'all' || searchKind == 'apps') && filteredAppItems.length > 0}
									{#each filteredAppItems as item}
										<div>
											<div class="text-sm font-semibold"
												><a href="/apps/get/{item.path}">App: {item.path}</a></div
											>
											<div class="flex gap-2 justify-between">
												<pre class="text-xs border p-2 overflow-auto max-h-40 w-full max-w-2xl"
													><code>{@html item.marked}</code></pre
												>
											</div>
										</div>
									{/each}
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
				</div></div
			></div
		></Portal
	>
{/if}
