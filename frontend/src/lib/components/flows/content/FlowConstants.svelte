<script lang="ts">
	import { dfs } from '$lib/components/flows/dfs'
	import FlowCard from '../common/FlowCard.svelte'
	import { Alert, Badge } from '$lib/components/common'
	import type { FlowModule, FlowModuleValue, InputTransform, PathScript, RawScript } from '$lib/gen'
	import { getContext, setContext } from 'svelte'
	import type { PropPickerWrapperContext } from '../propPicker/PropPickerWrapper.svelte'
	import { writable } from 'svelte/store'
	import Toggle from '../../Toggle.svelte'
	import InputTransformSchemaForm from '$lib/components/InputTransformSchemaForm.svelte'
	import type { FlowEditorContext } from '../types'

	export let noEditor: boolean

	let hideOptional = false
	const { flowStateStore, flowStore } = getContext<FlowEditorContext>('FlowEditorContext')

	$: scriptModules = dfs($flowStore.value.modules, (x) => x)
		.map((x) => [x.value, x] as [FlowModuleValue, FlowModule])
		.filter((x) => x[0].type == 'script' || x[0].type == 'rawscript' || x[0].type == 'flow') as [
		PathScript | RawScript,
		FlowModule
	][]

	$: resources = Object.fromEntries(
		scriptModules
			.map(([v, m]) => [
				m.id,
				Object.entries(v.input_transforms)
					.map((x) => {
						let schema = $flowStateStore[m.id]?.schema
						let val: { argName: string; type: string } | undefined = undefined

						const [k, inputTransform] = x
						const v = schema?.properties[k]

						if (
							v?.format?.includes('resource') &&
							inputTransform.type === 'static' &&
							(inputTransform.value === '' ||
								inputTransform.value === undefined ||
								inputTransform.value === null)
						) {
							val = {
								argName: k,
								type: v.format.split('-')[1]
							}
						}
						return val
					})
					.filter(Boolean)
			])
			.filter((x) => x[1].length > 0)
	) as {
		[k: string]: {
			argName: string
			type: string
		}[]
	}

	$: steps = scriptModules
		.map(
			([v, m]) =>
				[
					v.input_transforms,
					Object.entries(v.input_transforms)
						.filter((x) => {
							const shouldDisplay = hideOptional
								? $flowStateStore[m.id]?.schema?.required?.includes(x[0])
								: true
							return x[1].type == 'static' && shouldDisplay
						})
						.map((x) => x[0]),
					m
				] as [Record<string, InputTransform>, string[], FlowModule]
		)
		.filter(([i, f, m]) => f.length > 0)

	setContext<PropPickerWrapperContext>('PropPickerWrapper', {
		inputMatches: writable(undefined),
		focusProp: () => {},
		propPickerConfig: writable(undefined),
		clearFocus: () => {}
	})
</script>

<div class="min-h-full">
	<FlowCard {noEditor} title="All Static Inputs">
		<Toggle slot="header" bind:checked={hideOptional} options={{ left: 'Hide optional inputs' }} />
		<div class="min-h-full flex-1">
			<Alert type="info" title="Static Inputs" class="m-4"
				>This page centralizes the static inputs of every steps. It is aking to a file containing all
				constants. Modifying a value here modifies it in the step input directly. It is especially
				useful when forking a flow to get an overview of all the variables to parametrize that are
				not exposed directly as flow inputs.</Alert
			>
			{#if Object.keys(resources).length > 0}
				<Alert type="warning" title="Missing resources" class="m-4">
					The following resources are missing and the flow will not be fully runnable until they are
					set. Add your own resources:
					{#each Object.entries(resources) as [id, r]}
						{#each r as resource}
							<div class="mt-2">
								<Badge color="red">{id}</Badge> is missing a resource of type{' '}
								<Badge color="red">{resource?.type}</Badge> for the input{' '}
								<Badge color="red">{resource?.argName}</Badge>
							</div>
						{/each}
					{/each}
				</Alert>
			{/if}
			{#if steps.length == 0}
				<div class="mt-2"></div>
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
			{#each steps as [args, filter, m], index (m.id + index)}
				{#if filter.length > 0}
					<div class="relative h-full border-t p-4">
						<h2 class="sticky w-full top-0 z-10 inline-flex items-center py-2">
							<span class="mr-4">{m.summary || m.value['path'] || 'Inline script'}</span>
							<Badge large color="indigo">{m.id}</Badge>
						</h2>

						<InputTransformSchemaForm
							noDynamicToggle
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
