<script lang="ts">
	import type { AppViewerContext, HiddenInlineScript } from '$lib/components/apps/types'
	import Alert from '$lib/components/common/alert/Alert.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import { classNames } from '$lib/utils'
	import { Plus, X } from 'lucide-svelte'
	import ScriptSettingsSection from './ScriptSettingsSection.svelte'
	import { getContext } from 'svelte'

	export let script: HiddenInlineScript
	export let recomputeOnInputChanged: boolean | undefined = undefined

	$: isFrontend = script.inlineScript?.language === 'frontend'
	$: triggerEvents = script.autoRefresh ? ['start', 'refresh'] : []
	$: changeEvents = script.inlineScript?.refreshOn
		? script.inlineScript.refreshOn.map((x) => `${x.id} - ${x.key}`)
		: []

	$: hasNoTriggers =
		triggerEvents.length === 0 && (changeEvents.length === 0 || !recomputeOnInputChanged)

	const badgeClass = 'inline-flex items-center rounded-md px-2 py-0.5 text-xs font-medium border'
	const colors = {
		green: 'text-green-800 border-green-600 bg-green-100',
		indigo: 'text-indigo-800 border-indigo-600 bg-indigo-100',
		blue: 'text-blue-800 border-blue-600 bg-blue-100'
	}

	const { app } = getContext<AppViewerContext>('AppViewerContext')
	const { connectingInput } = getContext<AppViewerContext>('AppViewerContext')

	let addingDependency: boolean = false

	function applyConnection() {
		if (!$connectingInput.opened && $connectingInput.input !== undefined && addingDependency) {
			if ($connectingInput.input.connection) {
				const x = {
					id: $connectingInput.input.connection.componentId,
					key: $connectingInput.input.connection.path
				}

				if (!script.inlineScript) {
					return
				}

				if (script.inlineScript.refreshOn?.find((y) => y.id === x.id && y.key === x.key)) {
					return
				}

				if (!script.inlineScript.refreshOn) {
					script.inlineScript.refreshOn = [x]
				} else {
					script.inlineScript.refreshOn.push(x)
				}

				script.inlineScript = JSON.parse(JSON.stringify(script.inlineScript))

				addingDependency = false
			}

			$connectingInput = {
				opened: false,
				input: undefined,
				hoveredComponent: undefined
			}
			$app = $app
		}
	}
	$: $connectingInput && applyConnection()
</script>

<ScriptSettingsSection title="Triggers">
	{#if isFrontend}
		<div class="flex mb-4">
			<Button
				size="xs2"
				color="dark"
				on:click={() => {
					addingDependency = true
					$connectingInput = {
						opened: true,
						input: undefined,
						hoveredComponent: undefined
					}
				}}
			>
				<div class="flex flex-row gap-1 items-center">
					<Plus size={14} />

					Add dependency
				</div>
			</Button>
		</div>
	{/if}
	{#if hasNoTriggers}
		<Alert type="warning" title="No triggers" size="xs">
			This script has no triggers. It will never run.
		</Alert>
	{:else}
		{#if triggerEvents.length > 0}
			<div class="text-xs font-semibold text-slate-800 mb-1">Events</div>
			<div class="flex flex-row gap-2 flex-wrap">
				{#each triggerEvents as triggerEvent}
					<span class={classNames(badgeClass, colors['green'])}>{triggerEvent}</span>
				{/each}
			</div>
		{/if}
		{#if changeEvents.length > 0 && recomputeOnInputChanged}
			<div class="text-xs font-semibold text-slate-800 mb-1 mt-2">Change on value</div>
			<div class="flex flex-row gap-2 flex-wrap">
				{#each changeEvents as changeEvent}
					<span class={classNames(badgeClass, colors['blue'])}>
						{changeEvent}
						<button
							class="bg-blue-300 ml-2 p-0.5 rounded-md hover:bg-blue-400 cursor-pointer"
							on:click={() => {
								if (script.inlineScript?.refreshOn) {
									script.inlineScript.refreshOn = script.inlineScript.refreshOn.filter(
										(x) => `${x.id} - ${x.key}` !== changeEvent
									)
									script.inlineScript = JSON.parse(JSON.stringify(script.inlineScript))
								}
							}}
						>
							<X size="14" />
						</button>
					</span>
					<!-- delete button -->
				{/each}
			</div>
		{/if}
	{/if}
</ScriptSettingsSection>
