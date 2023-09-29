<script lang="ts">
	import { faMagicWandSparkles } from '@fortawesome/free-solid-svg-icons'
	import { Icon } from 'svelte-awesome'
	import { copilotInfo, hubScripts } from '$lib/stores'
	import { getContext } from 'svelte'
	import type { FlowEditorContext } from '../flows/types'
	import type { FlowCopilotContext, FlowCopilotModule } from './flow'
	import { nextId } from '../flows/flowStateUtils'
	import { ScriptService, type FlowModule } from '$lib/gen'
	import { APP_TO_ICON_COMPONENT } from '../icons'
	import { sendUserToast } from '$lib/toast'
	export let index: number
	export let open: boolean | undefined
	export let close: () => void
	export let funcDesc: string
	export let modules: FlowModule[]
	export let trigger = false

	// state
	let input: HTMLInputElement | undefined
	let hubCompletions: FlowCopilotModule['hubCompletions'] = []
	let selectedCompletion: FlowCopilotModule['selectedCompletion'] = undefined
	let lang: FlowCopilotModule['lang'] = undefined
	const { flowStore, flowStateStore } = getContext<FlowEditorContext>('FlowEditorContext')
	const { modulesStore: copilotModulesStore, genFlow } =
		getContext<FlowCopilotContext>('FlowCopilotContext')

	let doneTs = 0
	async function getHubCompletions(text: string) {
		try {
			// make sure we display the results of the last request last
			const ts = Date.now()
			const scriptIds = await ScriptService.queryHubScripts({
				text: `${text}`,
				limit: 3,
				kind: trigger ? 'trigger' : 'script'
			})
			if (ts < doneTs) return
			doneTs = ts

			const scripts = scriptIds
				.map((qs) => {
					const s = $hubScripts?.find((hs) => hs.ask_id === Number(qs.id))
					return s
				})
				.filter((s) => !!s)

			hubCompletions = scripts as FlowCopilotModule['hubCompletions']
		} catch (err) {
			if (err.name !== 'CancelError') throw err
		}
	}

	async function onGenerate() {
		if (!selectedCompletion && !$copilotInfo.exists_openai_resource_path) {
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

	$: {
		if (open) {
			setTimeout(() => {
				input?.focus()
			}, 0)
		}
	}
</script>

<div class="text-primary transition-all {funcDesc.length > 0 ? 'w-96' : 'w-60'}">
	<div>
		<div class="flex p-2 relative">
			<input
				type="text"
				bind:this={input}
				bind:value={funcDesc}
				on:input={() => {
					if (funcDesc.length > 2) {
						getHubCompletions(funcDesc)
					} else {
						hubCompletions = []
					}
				}}
				placeholder="AI Gen       or search hub {trigger ? 'triggers' : 'scripts'}"
			/>
			{#if funcDesc.length === 0}
				<Icon
					data={faMagicWandSparkles}
					class="absolute left-[65px] qhd:left-[75px] top-[18px] qhd:top-[20px] fill-current opacity-70"
				/>
			{/if}
		</div>
		{#if funcDesc.length > 0}
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
								<Icon data={faMagicWandSparkles} scale={0.8} />
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
								<Icon data={faMagicWandSparkles} scale={0.8} />
							</div>

							<div class="text-left text-xs text-secondary">
								Generate "{funcDesc}" in Python
							</div>
						</div>
					</button>
				</li>
			</ul>
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
										{item.summary ?? ''}
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
