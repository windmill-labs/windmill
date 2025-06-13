<script lang="ts">
	import { Tabs, Tab, TabContent } from '$lib/components/common'
	import SplitPanesWrapper from '$lib/components/splitPanes/SplitPanesWrapper.svelte'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import DetailPageDetailPanel from './DetailPageDetailPanel.svelte'

	interface Props {
		isOperator?: boolean
		flow_json?: any | undefined
		selected: string
		header?: import('svelte').Snippet
		form?: import('svelte').Snippet
		scriptRender?: import('svelte').Snippet
		save_inputs?: import('svelte').Snippet
		flow_step?: import('svelte').Snippet
		triggers?: import('svelte').Snippet
	}

	let {
		isOperator = false,
		flow_json = undefined,
		selected = $bindable(),
		header,
		form,
		scriptRender: script,
		save_inputs,
		flow_step,
		triggers
	}: Props = $props()

	let mobileTab: 'form' | 'detail' = $state('form')

	let clientWidth = $state(window.innerWidth)

	const script_render = $derived(script)
	const save_inputs_render = $derived(save_inputs)
	const flow_step_render = $derived(flow_step)
	const triggers_render = $derived(triggers)
</script>

<main class="h-screen w-full" bind:clientWidth>
	{#if clientWidth >= 768}
		<div class="h-full w-full">
			{@render header?.()}
			<SplitPanesWrapper>
				<Splitpanes>
					<Pane size={65} minSize={50}>
						{@render form?.()}
					</Pane>
					<Pane size={35} minSize={15}>
						<DetailPageDetailPanel bind:selected {isOperator} {flow_json}>
							{#snippet script()}
								{@render script_render?.()}
							{/snippet}
							{#snippet save_inputs()}
								{@render save_inputs_render?.()}
							{/snippet}
							{#snippet flow_step()}
								{@render flow_step_render?.()}
							{/snippet}
							{#snippet triggers()}
								{@render triggers_render?.()}
							{/snippet}
						</DetailPageDetailPanel>
					</Pane>
				</Splitpanes>
			</SplitPanesWrapper>
		</div>
	{:else}
		<div class="h-full w-full">
			{@render header?.()}
			<Tabs bind:selected={mobileTab}>
				<Tab value="form">Run form</Tab>
				<Tab value="saved_inputs">Inputs</Tab>
				{#if !isOperator}
					<Tab value="triggers">Triggers</Tab>
				{/if}
				{#if flow_json}
					<Tab value="raw">Export</Tab>
				{:else}
					<Tab value="script">Script</Tab>
				{/if}

				{#snippet content()}
					<div class="h-full">
						<TabContent value="form" class="flex flex-col flex-1 h-full">
							{@render form?.()}
						</TabContent>

						<TabContent value="saved_inputs" class="flex flex-col flex-1 h-full">
							{@render save_inputs?.()}
						</TabContent>
						<TabContent value="triggers" class="flex flex-col flex-1 h-full mt-[-2px]">
							{@render triggers?.()}
						</TabContent>
						<TabContent value="script" class="flex flex-col flex-1 h-full">
							{@render script?.()}
						</TabContent>
					</div>
				{/snippet}
			</Tabs>
		</div>
	{/if}
</main>
