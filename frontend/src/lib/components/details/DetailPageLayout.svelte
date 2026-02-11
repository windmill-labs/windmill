<script lang="ts">
	import { Tabs, Tab, TabContent } from '$lib/components/common'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import DetailPageDetailPanel from './DetailPageDetailPanel.svelte'
	import FlowViewerInner from '../FlowViewerInner.svelte'

	interface Props {
		isOperator?: boolean
		flow_json?: any | undefined
		selected: string
		forceSmallScreen?: boolean
		isChatMode?: boolean
		header?: import('svelte').Snippet
		form?: import('svelte').Snippet
		scriptRender?: import('svelte').Snippet
		save_inputs?: import('svelte').Snippet
		flow_step?: import('svelte').Snippet
		triggers?: import('svelte').Snippet
		flow_graph?: import('svelte').Snippet
	}

	let {
		isOperator = false,
		flow_json = undefined,
		selected = $bindable(),
		forceSmallScreen = false,
		isChatMode = false,
		header,
		form,
		scriptRender: script,
		save_inputs,
		flow_step,
		triggers,
		flow_graph
	}: Props = $props()

	let mobileTab: 'form' | 'detail' = $state('form')

	let clientWidth = $state(window.innerWidth)

	const script_render = $derived(script)
	const save_inputs_render = $derived(save_inputs)
	const flow_step_render = $derived(flow_step)
	const triggers_render = $derived(triggers)
	const flow_graph_render = $derived(flow_graph)

	const useDesktopLayout = $derived(clientWidth >= 768 && !forceSmallScreen)
</script>

<main class="h-screen w-full" bind:clientWidth>
	{#if useDesktopLayout}
		<div class="h-full w-full flex flex-col">
			{@render header?.()}
			<div class="grow min-h-0 w-full">
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
			</div>
		</div>
	{:else}
		<div class="h-full w-full flex flex-col">
			{@render header?.()}
			<div class="grow min-h-0 w-full flex flex-col">
				<Tabs bind:selected={mobileTab} wrapperClass="flex-none">
					<Tab value="form" label={isChatMode ? 'Chat' : 'Run form'} />
					{#if !isChatMode}
						<Tab value="saved_inputs" label="Inputs" />
					{/if}
					{#if isChatMode && flow_json}
						<Tab value="flow" label="Flow graph" />
					{/if}
					{#if !isOperator}
						<Tab value="triggers" label="Triggers" />
					{/if}
					{#if flow_json}
						<Tab value="raw" label="Export" />
					{:else}
						<Tab value="script" label="Script" />
					{/if}

					{#snippet content()}
						<div class="grow min-h-0 overflow-y-auto">
							<TabContent value="form" class="flex flex-col flex-1 h-full">
								{@render form?.()}
							</TabContent>

							<TabContent value="saved_inputs" class="flex flex-col flex-1 h-full">
								{@render save_inputs?.()}
							</TabContent>
							<TabContent value="triggers" class="flex flex-col flex-1 h-full mt-[-2px]">
								{@render triggers?.()}
							</TabContent>
							{#if isChatMode && flow_json}
								<TabContent value="flow" class="flex flex-col flex-1 h-full">
									{@render flow_graph_render?.()}
								</TabContent>
							{/if}
							{#if flow_json}
								<TabContent value="raw" class="flex flex-col flex-1 h-full overflow-auto p-2">
									<FlowViewerInner flow={flow_json} />
								</TabContent>
							{/if}
							<TabContent value="script" class="flex flex-col flex-1 h-full">
								{@render script?.()}
							</TabContent>
						</div>
					{/snippet}
				</Tabs>
			</div>
		</div>
	{/if}
</main>
