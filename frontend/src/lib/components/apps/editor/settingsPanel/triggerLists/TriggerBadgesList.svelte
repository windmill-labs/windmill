<script lang="ts">
	import type { AppViewerContext, InlineScript } from '$lib/components/apps/types'
	import { Button } from '$lib/components/common'
	import { classNames } from '$lib/utils'
	import { Plus, X } from 'lucide-svelte'
	import { getContext } from 'svelte'
	import { getAllRecomputeIdsForComponent } from '../../appUtils'

	export let inputDependencies: string[] = []
	export let inlineScript: InlineScript | undefined
	export let onClick: boolean = false
	export let onLoad: boolean = false
	export let id: string | undefined = undefined
	export let doNotRecomputeOnInputChanged: boolean = false

	const colors = {
		red: 'text-red-800 border-red-600 bg-red-100',
		green: 'text-green-800 border-green-600 bg-green-100',
		indigo: 'text-indigo-800 border-indigo-600 bg-indigo-100',
		blue: 'text-blue-800 border-blue-600 bg-blue-100'
	}

	let badgeClass = 'inline-flex items-center rounded-md px-2 py-0.5 text-xs font-medium border'

	const { app, worldStore } = getContext<AppViewerContext>('AppViewerContext')

	const { connectingInput } = getContext<AppViewerContext>('AppViewerContext')

	let addingDependency: boolean = false

	function applyConnection() {
		if (!$connectingInput.opened && $connectingInput.input !== undefined && addingDependency) {
			if ($connectingInput.input.connection) {
				const x = {
					id: $connectingInput.input.connection.componentId,
					key: $connectingInput.input.connection.path
				}

				if (!inlineScript) {
					return
				}

				if (inlineScript.refreshOn?.find((y) => y.id === x.id && y.key === x.key)) {
					return
				}

				if (!inlineScript.refreshOn) {
					inlineScript.refreshOn = [x]
				} else {
					inlineScript.refreshOn.push(x)
				}

				inlineScript = JSON.parse(JSON.stringify(inlineScript))

				addingDependency = false
			}

			$connectingInput = {
				opened: false,
				input: undefined
			}
			$app = $app
			$worldStore = $worldStore
		}
	}

	$: frontendDependencies =
		inlineScript?.language === 'frontend'
			? inlineScript?.refreshOn?.map((x) => `${x.id} - ${x.key}`) ?? []
			: undefined

	$: $connectingInput && applyConnection()

	$: recomputedBadges = getAllRecomputeIdsForComponent($app, id)

	function deleteDep(index: number) {
		if (inlineScript) {
			inlineScript.refreshOn?.splice(index, 1)
		}
		inlineScript = inlineScript
	}
</script>

<div class="flex w-full flex-col items-start gap-2 mt-2 mb-1">
	{#if recomputedBadges.length === 0 && !onLoad && !onClick && inputDependencies?.length === 0 && !frontendDependencies}
		<p class="text-xs font-semibold text-slate-800 ">
			This script has no triggers. It will never run.
		</p>
	{:else}
		<div class="text-sm font-semibold text-gray-800 ">Triggered by</div>

		{#if onLoad || onClick}
			<div class="w-full">
				<div class="text-xs font-semibold text-slate-800 mb-1">Events</div>
				<div class="flex flex-row gap-2 flex-wrap">
					{#if onLoad}
						<span class={classNames(badgeClass, colors['green'])}>Start</span>
						<span class={classNames(badgeClass, colors['green'])}>Refresh</span>
					{/if}
					{#if onClick}
						<span class={classNames(badgeClass, colors['green'])}>Click</span>
					{/if}
				</div>
			</div>
		{/if}
		{#if inputDependencies.length > 0 && !doNotRecomputeOnInputChanged}
			<div class="w-full">
				<div class="flex justify-between items-center mb-1">
					<div class="text-xs font-semibold text-slate-800">Change on values</div>
				</div>
				<div class="flex flex-row gap-2 flex-wrap">
					{#each inputDependencies as label}
						<span class={classNames(badgeClass, colors['blue'])}>
							{label}
						</span>
					{/each}
				</div>
			</div>
		{/if}

		{#if recomputedBadges?.length > 0}
			<div class="w-full">
				<div class="text-xs font-semibold text-slate-800 mb-1">Computation of</div>
				<div class="flex flex-row gap-2 flex-wrap">
					{#each recomputedBadges as badge}
						<span class={classNames(badgeClass, colors['indigo'])}>{badge}</span>
					{/each}
				</div>
			</div>
		{/if}
	{/if}
	{#if frontendDependencies && !doNotRecomputeOnInputChanged}
		<div class="w-full">
			<div class="flex justify-between items-center">
				<div class="text-xs font-semibold text-slate-800 mb-1">Change on values</div>
				{#if inlineScript?.language === 'frontend'}
					<Button
						variant="border"
						size="xs"
						color="light"
						btnClasses="!px-1 !py-0.5"
						on:click={() => {
							addingDependency = true
							$connectingInput = {
								opened: true,
								input: undefined
							}
						}}
					>
						<div class="flex flex-row gap-1 items-center">
							Add dependency
							<Plus size={14} />
						</div>
					</Button>
				{/if}
			</div>
			<div class="flex flex-row gap-2 flex-wrap">
				{#each frontendDependencies as label, index}
					<span class={classNames(badgeClass, colors['red'])}>
						{label}
						<button
							on:click={() => deleteDep(index)}
							class="bg-red-300 cursor-pointer hover:bg-red-400 ml-1 rounded-md"
						>
							<X size={18} class="p-0.5" />
						</button>
					</span>
				{/each}
			</div>
		</div>
	{/if}
</div>
