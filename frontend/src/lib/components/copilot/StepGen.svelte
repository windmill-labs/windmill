<script lang="ts">
	import { copilotInfo, workspaceStore } from '$lib/stores'
	import { createEventDispatcher, getContext } from 'svelte'
	import type { FlowEditorContext } from '../flows/types'
	import type { FlowCopilotContext, FlowCopilotModule } from './flow'
	import { ScriptService, type FlowModule, type Script } from '$lib/gen'
	import { APP_TO_ICON_COMPONENT } from '../icons'
	import { sendUserToast } from '$lib/toast'
	import { nextId } from '../flows/flowModuleNextId'
	import { Wand2 } from 'lucide-svelte'
	import SearchItems from '../SearchItems.svelte'
	import { defaultIfEmptyString, emptyString } from '$lib/utils'

	export let index: number
	export let open: boolean | undefined
	export let close: () => void
	export let funcDesc: string
	export let modules: FlowModule[]
	export let trigger = false
	export let disableAi = false

	// state
	let input: HTMLInputElement | undefined
	let hubCompletions: FlowCopilotModule['hubCompletions'] = []
	let selectedCompletion: FlowCopilotModule['selectedCompletion'] = undefined
	let lang: FlowCopilotModule['lang'] = undefined

	const { flowStore, flowStateStore } = getContext<FlowEditorContext>('FlowEditorContext')
	const { modulesStore: copilotModulesStore, genFlow } =
		getContext<FlowCopilotContext>('FlowCopilotContext')

	let scripts: Script[] | undefined = undefined
	let filteredItems: (Script & { marked?: string })[] = []
	$: prefilteredItems = scripts ?? []

	async function loadScripts(): Promise<void> {
		const loadedScripts = await ScriptService.listScripts({
			workspace: $workspaceStore!,
			perPage: 300,
			kinds: trigger ? 'trigger' : 'script'
		})

		scripts = loadedScripts
	}

	$: scripts == undefined && funcDesc?.length > 1 && loadScripts()

	let doneTs = 0
	async function getHubCompletions(text: string) {
		try {
			// make sure we display the results of the last request last
			const ts = Date.now()
			const scripts = (
				await ScriptService.queryHubScripts({
					text: `${text}`,
					limit: 3,
					kind: trigger ? 'trigger' : 'script'
				})
			).map((s) => ({
				...s,
				path: `hub/${s.version_id}/${s.app}/${s.summary.toLowerCase().replaceAll(/\s+/g, '_')}`
			}))
			if (ts < doneTs) return
			doneTs = ts

			hubCompletions = scripts as FlowCopilotModule['hubCompletions']
		} catch (err) {
			if (err.name !== 'CancelError') throw err
		}
	}

	async function onGenerate() {
		if (!selectedCompletion && !$copilotInfo.enabled) {
			sendUserToast(
				'Windmill AI is not enabled, you can activate it in the workspace settings',
				true
			)
			return
		}
		$copilotModulesStore = [
			{
				id: nextId($flowStateStore, $flowStore),
				type: trigger ? 'trigger' : 'script',
				description: funcDesc,
				code: '',
				source: selectedCompletion ? 'hub' : 'custom',
				hubCompletions,
				selectedCompletion,
				editor: undefined,
				lang
			}
		]
		genFlow?.(index, modules, true)
	}

	const dispatch = createEventDispatcher()
	$: {
		if (open) {
			setTimeout(() => {
				input?.focus()
			}, 0)
		}
	}
</script>

<SearchItems
	filter={funcDesc}
	items={prefilteredItems}
	bind:filteredItems
	f={(x) => (emptyString(x.summary) ? x.path : x.summary + ' (' + x.path + ')')}
/>

<div class="text-primary transition-all {funcDesc?.length > 0 ? 'w-96' : 'w-60'}">
	<div>
		<div class="flex p-2 relative">
			<input
				type="text"
				bind:this={input}
				bind:value={funcDesc}
				on:input={() => {
					if (funcDesc?.length > 2) {
						getHubCompletions(funcDesc)
					} else {
						hubCompletions = []
					}
				}}
				placeholder="Search {trigger ? 'triggers' : 'scripts'} or AI gen"
			/>
			{#if funcDesc?.length === 0}
				<Wand2
					size={14}
					class="absolute right-4 top-1/2 -translate-y-1/2 fill-current opacity-70 text-violet-800 dark:text-violet-400"
				/>
			{/if}
		</div>
		{#if !disableAi && funcDesc?.length > 0}
			<ul class="transition-all divide-y">
				<li>
					<button
						class="py-2 gap-4 flex flex-row hover:bg-surface-hover transition-all items-center justify-between w-full"
						on:click={() => {
							lang = 'bun'
							onGenerate()
							close()
						}}
					>
						<div class="flex items-center gap-2.5 px-2">
							<div
								class="rounded-md p-1 flex justify-center items-center bg-surface border w-6 h-6"
							>
								<Wand2 size={14} class="text-violet-800 dark:text-violet-400" />
							</div>

							<div class="text-left text-xs text-secondary">
								Generate "{funcDesc}" in TypeScript
							</div>
						</div>
					</button>
				</li>
				<li>
					<button
						class="py-2 gap-4 flex flex-row hover:bg-surface-hover transition-all items-center justify-between w-full"
						on:click={() => {
							lang = 'python3'
							onGenerate()
							close()
						}}
					>
						<div class="flex items-center gap-2.5 px-2">
							<div
								class="rounded-md p-1 flex justify-center items-center bg-surface border w-6 h-6"
							>
								<Wand2 size={14} class="text-violet-800 dark:text-violet-400" />
							</div>

							<div class="text-left text-xs text-secondary">
								Generate "{funcDesc}" in Python
							</div>
						</div>
					</button>
				</li>
			</ul>
		{/if}
		{#if funcDesc?.length > 0 && filteredItems?.length > 0}
			<div class="text-left mt-2">
				<p class="text-xs text-secondary ml-2">Workspace {trigger ? 'Triggers' : 'Scripts'}</p>
				<ul class="transition-all divide-y">
					{#each filteredItems?.slice(0, 3) ?? [] as item (item.path)}
						<li>
							<button
								class="py-2 gap-4 flex flex-row hover:bg-surface-hover transition-all items-center justify-between w-full"
								on:click={() => {
									dispatch('insert', { path: item.path, summary: item.summary })
									close()
								}}
							>
								<div class="flex items-center gap-2.5 px-2">
									<div
										class="rounded-md p-1 flex justify-center items-center bg-surface border w-6 h-6"
									>
										<svelte:component this={APP_TO_ICON_COMPONENT[item['app']]} />
									</div>

									<div class="text-left text-xs text-secondary">
										{defaultIfEmptyString(item.summary, item.path)}
									</div>
								</div>
							</button>
						</li>
					{/each}
				</ul>
			</div>
		{/if}
		{#if hubCompletions.length > 0}
			<div class="text-left mt-2">
				<p class="text-xs text-secondary ml-2">Hub {trigger ? 'Triggers' : 'Scripts'}</p>
				<ul class="transition-all divide-y">
					{#each hubCompletions as item (item.path)}
						<li>
							<button
								class="py-2 gap-4 flex flex-row hover:bg-surface-hover transition-all items-center justify-between w-full"
								on:click={() => {
									selectedCompletion = item
									close()
									onGenerate()
								}}
							>
								<div class="flex items-center gap-2.5 px-2">
									<div
										class="rounded-md p-1 flex justify-center items-center bg-surface border w-6 h-6"
									>
										<svelte:component this={APP_TO_ICON_COMPONENT[item['app']]} />
									</div>

									<div class="text-left text-xs text-secondary">
										{item.summary ?? ''} ({item.app})
									</div>
								</div>
							</button>
						</li>
					{/each}
				</ul>
			</div>
		{/if}
	</div>
</div>
