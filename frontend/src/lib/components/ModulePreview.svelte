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

	interface Props {
		mod: FlowModule
		schema: Schema | { properties?: Record<string, any> }
		pickableProperties: PickableProperties | undefined
		testJob?: Job | undefined
		testIsLoading?: boolean
		noEditor?: boolean
		scriptProgress?: any
		focusArg?: string
	}

	let {
		mod,
		schema,
		pickableProperties,
		testJob = $bindable(undefined),
		testIsLoading = $bindable(false),
		noEditor = false,
		scriptProgress = $bindable(undefined),
		focusArg = undefined
	}: Props = $props()

	const { flowStore, testSteps } = getContext<FlowEditorContext>('FlowEditorContext')
	let moduleTest: ModuleTest | undefined = $state()

	export function runTestWithStepArgs() {
		moduleTest?.runTest(testSteps.getStepArgs(mod.id))
	}
</script>

<ModuleTest
	{mod}
	{noEditor}
	bind:testJob
	bind:testIsLoading
	bind:scriptProgress
	bind:this={moduleTest}
/>

<div class="p-4">
	{#if flowStore.val.value.same_worker}
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

	<ModulePreviewForm {pickableProperties} {mod} {schema} {focusArg} />
</div>
