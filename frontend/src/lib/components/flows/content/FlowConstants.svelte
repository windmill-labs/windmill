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
						.filter((x) => x[1].type == 'static')
						.map((x) => x[0]),
					m
				] as [Record<string, InputTransform>, string[], FlowModule]
		)
		.filter(([i, f, m]) => f.length > 0)

	setContext<PropPickerWrapperContext>('PropPickerWrapper', {
		focusProp: () => {},
		propPickerConfig: writable(undefined),
		clearFocus: () => {}
	})
</script>

<div class="h-full overflow-hidden">
	<FlowCard title="All Static Inputs">
		<div class="h-full flex-1">
			<Alert type="info" title="Static Inputs"
				>This page centralizes the static inputs of every steps. It is akin to a file containing all
				constants. Modifying a value here modify it in the step input directly. It is especially
				useful when forking a flow to get an overview of all the variables to parametrize that are
				not exposed directly as flow inputs.</Alert
			>
			{#if steps.length == 0}
				<div class="mt-2" />
				{#if $flowStore.value.modules.length == 0}
					<Alert type="warning" title="No steps">
						This flow has no steps. Add a step to see its static inputs.
					</Alert>
				{:else}
					<Alert type="warning" title="No static inputs">
						This flow has no steps with static inputs. Add a step with static inputs to see them
						here.
					</Alert>
				{/if}
			{/if}
			{#each steps as [args, filter, m] (m.id)}
				{#if filter.length > 0}
					<div class="box">
						<h2 class="pb-4 inline-flex items-center"
							><span class="mr-4">{m.summary || m.value['path'] || 'Inline script'}</span>
							<Badge large color="indigo">{m.id}</Badge>
						</h2>

						<SchemaForm
							noDynamicToggle
							inputTransform
							{filter}
							schema={$flowStateStore[m.id]?.schema ?? {}}
							bind:args
						/>
					</div>
				{/if}
			{/each}
		</div>
	</FlowCard>
</div>
