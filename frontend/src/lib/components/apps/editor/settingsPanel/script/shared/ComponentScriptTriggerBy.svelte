<script lang="ts">
	import type { InputConnection, ResultAppInput } from '$lib/components/apps/inputType'
	import { Button } from '$lib/components/common'
	import Alert from '$lib/components/common/alert/Alert.svelte'
	import { classNames } from '$lib/utils'
	import { Plus, X } from 'lucide-svelte'
	import type { AppComponent } from '../../../component'
	import { getAllTriggerEvents, isTriggerable, getDependencies } from '../utils'
	import ScriptSettingsSection from './ScriptSettingsSection.svelte'
	import { getContext } from 'svelte'
	import type { AppViewerContext } from '$lib/components/apps/types'

	export let appComponent: AppComponent
	export let appInput: ResultAppInput

	$: isFrontend =
		appInput.runnable?.type === 'runnableByName' &&
		appInput.runnable?.inlineScript?.language === 'frontend'
	$: triggerEvents = getAllTriggerEvents(appComponent, appInput.autoRefresh)
	$: changeEvents =
		isFrontend && appInput.runnable?.type === 'runnableByName'
			? appInput.runnable.inlineScript?.refreshOn
				? appInput.runnable.inlineScript.refreshOn.map((x) => `${x.id} - ${x.key}`)
				: ['s']
			: getDependencies(appInput.fields)

	$: hasNoTriggers = triggerEvents.length === 0 && changeEvents.length === 0

	const badgeClass = 'inline-flex items-center rounded-md px-2 py-0.5 text-xs font-medium border'
	const colors = {
		green: 'text-green-800 border-green-600 bg-green-100',
		indigo: 'text-indigo-800 border-indigo-600 bg-indigo-100',
		blue: 'text-blue-800 border-blue-600 bg-blue-100'
	}

	const { connectingInput, app } = getContext<AppViewerContext>('AppViewerContext')

	function applyConnection(connection: InputConnection) {
		const refresh = {
			id: connection.componentId,
			key: connection.path
		}

		if (appInput.runnable?.type !== 'runnableByName') {
			return
		}

		if (!appInput.runnable?.inlineScript) {
			return
		}

		if (
			appInput.runnable.inlineScript.refreshOn?.find(
				(y) => y.id === refresh.id && y.key === refresh.key
			)
		) {
			return
		}

		if (!appInput.runnable.inlineScript.refreshOn) {
			appInput.runnable.inlineScript.refreshOn = [refresh]
		} else {
			appInput.runnable.inlineScript.refreshOn.push(refresh)
		}

		appInput.runnable.inlineScript = JSON.parse(JSON.stringify(appInput.runnable.inlineScript))
		$app = $app
	}
</script>

<ScriptSettingsSection title="Triggers">
	{#if isFrontend}
		<div class="flex mb-4">
			<Button
				size="xs2"
				color="dark"
				on:click={() => {
					$connectingInput = {
						opened: true,
						input: undefined,
						hoveredComponent: undefined,
						onConnect: applyConnection
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

		{#if changeEvents.length > 0 && !isTriggerable(appComponent.type)}
			<div class="text-xs font-semibold text-slate-800 mb-1 mt-2">Change on value</div>
			<div class="flex flex-row gap-2 flex-wrap">
				{#each changeEvents as changeEvent}
					<span class={classNames(badgeClass, colors['blue'])}>
						{changeEvent}
						{#if isFrontend}
							<button
								class="bg-blue-300 ml-2 p-0.5 rounded-md hover:bg-blue-400 cursor-pointer"
								on:click={() => {
									if (
										appInput.runnable?.type === 'runnableByName' &&
										appInput.runnable.inlineScript
									) {
										appInput.runnable.inlineScript.refreshOn =
											appInput.runnable.inlineScript.refreshOn?.filter(
												(x) => `${x.id} - ${x.key}` !== changeEvent
											)
										appInput.runnable.inlineScript = JSON.parse(
											JSON.stringify(appInput.runnable.inlineScript)
										)
										$app = $app
									}
								}}
							>
								<X size="14" />
							</button>
						{/if}</span
					>
				{/each}
			</div>
		{/if}
	{/if}
</ScriptSettingsSection>
