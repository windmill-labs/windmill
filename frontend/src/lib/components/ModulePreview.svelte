<script lang="ts">
	import type { Schema } from '$lib/common'
	import { type FlowModule, type Job } from '$lib/gen'
	import { CornerDownLeft, Loader2 } from 'lucide-svelte'
	import Button from './common/button/Button.svelte'
	import ModulePreviewForm from './ModulePreviewForm.svelte'
	import type { PickableProperties } from './flows/previousResults'
	import ModuleTest from './ModuleTest.svelte'
	import { getContext } from 'svelte'
	import type { FlowEditorContext } from './flows/types'

	const { flowStore } = getContext<FlowEditorContext>('FlowEditorContext')

	export let mod: FlowModule
	export let schema: Schema | { properties?: Record<string, any> }
	export let pickableProperties: PickableProperties | undefined
	export let testJob: Job | undefined = undefined
	export let testIsLoading = false
	export let noEditor = false
	export let scriptProgress = undefined

	let moduleTest: ModuleTest
	let stepArgs: Record<string, any> | undefined = undefined

	export function runTestWithStepArgs() {
		moduleTest?.runTest(stepArgs)
	}
</script>

<ModuleTest
	{mod}
	{schema}
	{pickableProperties}
	{stepArgs}
	{noEditor}
	bind:testJob
	bind:testIsLoading
	bind:scriptProgress
	bind:this={moduleTest}
/>

<div class="p-4">
	{#if $flowStore.value.same_worker}
		<div class="mb-1 bg-yellow-100 text-yellow-700 p-1 text-xs"
			>The `./shared` folder is not passed across individual "Test this step"</div
		>
	{/if}

	<div class="w-full justify-center flex">
		{#if testIsLoading}
			<Button size="sm" on:click={moduleTest?.cancelJob} btnClasses="w-full" color="red">
				<Loader2 size={16} class="animate-spin mr-1" />
				Cancel
			</Button>
		{:else}
			<Button
				color="dark"
				btnClasses="truncate"
				size="sm"
				on:click={runTestWithStepArgs}
				shortCut={{
					Icon: CornerDownLeft
				}}
			>
				Run
			</Button>
		{/if}
	</div>

	<ModulePreviewForm {pickableProperties} {mod} {schema} bind:args={stepArgs} />
</div>
