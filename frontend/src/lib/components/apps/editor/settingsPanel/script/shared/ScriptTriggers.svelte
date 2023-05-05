<script lang="ts">
	import type { InputConnection } from '$lib/components/apps/inputType'
	import Alert from '$lib/components/common/alert/Alert.svelte'
	import { classNames } from '$lib/utils'
	import { Plus, X } from 'lucide-svelte'
	import { Button } from '$lib/components/common'
	import { getContext } from 'svelte'
	import type { AppViewerContext, InlineScript } from '$lib/components/apps/types'
	import Tooltip from '$lib/components/Tooltip.svelte'

	export let triggerEvents: string[] = []
	export let inlineScript: InlineScript | undefined = undefined
	export let isFrontend: boolean = false
	export let dependencies: string[] = []
	export let shoudlDisplayChangeEvents: boolean = false

	$: changeEvents = isFrontend
		? inlineScript?.refreshOn
			? inlineScript.refreshOn.map((x) => `${x.id} - ${x.key}`)
			: []
		: dependencies

	$: hasNoTriggers =
		triggerEvents.length === 0 && (changeEvents.length === 0 || !shoudlDisplayChangeEvents)

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

		if (!inlineScript) {
			return
		}

		if (inlineScript.refreshOn?.find((y) => y.id === refresh.id && y.key === refresh.key)) {
			return
		}

		if (!inlineScript.refreshOn) {
			inlineScript.refreshOn = [refresh]
		} else {
			inlineScript.refreshOn.push(refresh)
		}

		inlineScript = JSON.parse(JSON.stringify(inlineScript))
		$app = $app
	}
</script>

{#if isFrontend && shoudlDisplayChangeEvents}
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
	{#if changeEvents.length > 0 && shoudlDisplayChangeEvents}
		<div class="text-xs font-semibold text-slate-800 mb-1 mt-2">Change on value</div>
		<div class="flex flex-row gap-2 flex-wrap">
			{#each changeEvents as changeEvent}
				<span class={classNames(badgeClass, colors['blue'])}>
					{changeEvent}
					{#if changeEvent === 'Eval'}
						<Tooltip class="!text-blue-600 ml-1">
							At least one input is configured as an evaluated input and the component will be
							triggered if the result of the eval change. Eval expressions are re-evaluated on any
							output or state changes.
						</Tooltip>
					{/if}
					{#if isFrontend}
						<button
							class="bg-blue-300 ml-2 p-0.5 rounded-md hover:bg-blue-400 cursor-pointer"
							on:click={() => {
								if (inlineScript?.refreshOn) {
									inlineScript.refreshOn = inlineScript.refreshOn.filter(
										(x) => `${x.id} - ${x.key}` !== changeEvent
									)
									inlineScript = JSON.parse(JSON.stringify(inlineScript))
								}
							}}
						>
							<X size="14" />
						</button>
					{/if}
				</span>
			{/each}
		</div>
	{/if}
{/if}
