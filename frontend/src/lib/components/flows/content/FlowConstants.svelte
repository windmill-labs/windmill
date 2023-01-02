<script lang="ts">
	import { dfs, flowStore } from '$lib/components/flows/flowStore'
	import FlowCard from '../common/FlowCard.svelte'
	import { Alert, Badge } from '$lib/components/common'
	import type { FlowModule, FlowModuleValue, InputTransform, PathScript, RawScript } from '$lib/gen'
	import SchemaForm from '$lib/components/SchemaForm.svelte'
	import { flowStateStore } from '../flowState'
	import { setContext } from 'svelte'
	import type { PropPickerWrapperContext } from '../propPicker/PropPickerWrapper.svelte'
	import { writable } from 'svelte/store'
	import { slide } from 'svelte/transition'
	import Toggle from '../../Toggle.svelte'

	let hideOptional = false

	$: steps = (
		dfs($flowStore.value.modules, (x) => x)
			.map((x) => [x.value, x] as [FlowModuleValue, FlowModule])
			.filter((x) => x[0].type == 'script' || x[0].type == 'rawscript') as [
			PathScript | RawScript,
			FlowModule
		][]
	)
		.map(
			([v, m]) =>
				[
					v.input_transforms,
					Object.entries(v.input_transforms)
						.filter((x) => {
							const shouldDisplay = hideOptional
								? $flowStateStore[m.id]?.schema.required?.includes(x[0])
								: true
							return x[1].type == 'static' && shouldDisplay
						})
						.map((x) => x[0]),
					m
				] as [Record<string, InputTransform>, string[], FlowModule]
		)
		.filter(([i, f, m]) => f.length > 0)
	
	$: console.log({flowState: $flowStateStore, steps});

	setContext<PropPickerWrapperContext>('PropPickerWrapper', {
		focusProp: () => {},
		propPickerConfig: writable(undefined),
		clearFocus: () => {}
	})
</script>

<div class="min-h-full">
	<FlowCard title="All Static Inputs">
		<Toggle
			slot="header"
			bind:checked={hideOptional}
			options={{left: 'Hide optional inputs'}}
		/>
		<div class="min-h-full flex-1">
			<Alert type="info" title="Static Inputs" class="m-4"
				>This page centralizes the static inputs of every steps. It is akin to a file containing all
				constants. Modifying a value here modify it in the step input directly. It is especially
				useful when forking a flow to get an overview of all the variables to parametrize that are
				not exposed directly as flow inputs.</Alert
			>
			{#if steps.length == 0}
				<div class="mt-2" />
				{#if $flowStore.value.modules.length == 0}
					<Alert type="warning" title="No steps" class="m-4">
						This flow has no steps. Add a step to see its static inputs.
					</Alert>
				{:else}
					<Alert type="warning" title="No static inputs" class="m-4">
						This flow has no steps with static inputs. Add a step with static inputs to see them
						here.
					</Alert>
				{/if}
			{/if}
			{#each steps as [args, filter, m] (m.id)}
				{#if filter.length > 0}
					<div transition:slide class="relative h-full border-t p-4">
						<h2 class="sticky w-full top-0 z-10 inline-flex items-center bg-white py-2">
							<span class="mr-4">{m.summary || m.value['path'] || 'Inline script'}</span>
							<Badge large color="indigo">{m.id}</Badge>
						</h2>

						<SchemaForm
							noDynamicToggle
							inputTransform
							{filter}
							class="mt-2"
							schema={$flowStateStore[m.id]?.schema ?? {}}
							bind:args
						/>
					</div>
				{/if}
			{/each}
		</div>
	</FlowCard>
</div>
