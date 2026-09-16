<script lang="ts">
	import { Tabs, Tab, TabContent } from '$lib/components/common'
	import PagedContent from '$lib/components/common/modal/PagedContent.svelte'
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
		/** `graphInline`: whether the form should carry the flow graph under it. It does in the
		 * split layout; the tabbed layout gives the graph a tab of its own. */
		form?: import('svelte').Snippet<[{ graphInline: boolean }]>
		scriptRender?: import('svelte').Snippet
		save_inputs?: import('svelte').Snippet
		/** `onBack`: set where the step is a page pushed over the graph, so its header can lead
		 * back; absent where the step has a tab of its own. */
		flow_step?: import('svelte').Snippet<[{ onBack?: () => void }]>
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

	let mobileTab = $state('form')

	let clientWidth = $state(window.innerWidth)

	const script_render = $derived(script)
	const save_inputs_render = $derived(save_inputs)
	const flow_step_render = $derived(flow_step)
	const triggers_render = $derived(triggers)
	const flow_graph_render = $derived(flow_graph)

	const useDesktopLayout = $derived(clientWidth >= 768 && !forceSmallScreen)

	// The tabbed layout has no Step tab: a step opens as a page pushed over the graph tab, and
	// the way back is the graph.
	const graphPage = $derived(selected === 'flow_step' ? 'step' : 'graph')

	// The Trigger node asks for the triggers pane the same way a step asks for its own; on the
	// tabbed layout that pane is a tab, and the tabs keep their own selection.
	$effect(() => {
		if (selected === 'triggers') mobileTab = 'triggers'
	})
</script>

<main class="h-screen w-full" bind:clientWidth>
	{#if useDesktopLayout}
		<div class="h-full w-full flex flex-col">
			{@render header?.()}
			<div class="grow min-h-0 w-full">
				<Splitpanes>
					<Pane size={65} minSize={50}>
						{@render form?.({ graphInline: true })}
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
								{@render flow_step_render?.({})}
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
				<!-- no-scrollbar: at phone widths the tabs overflow their strip, and a browser with
				     classic scrollbars would spend a track under them, opening a band between the tabs
				     and the content. Wheel, trackpad and drag still scroll the strip. -->
				<Tabs bind:selected={mobileTab} wrapperClass="flex-none no-scrollbar">
					<Tab value="form" label={isChatMode ? 'Chat' : 'Run form'} />
					{#if !isChatMode}
						<Tab value="saved_inputs" label="Inputs" />
					{/if}
					{#if flow_json}
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
								{@render form?.({ graphInline: false })}
							</TabContent>

							<TabContent value="saved_inputs" class="flex flex-col flex-1 h-full">
								{@render save_inputs?.()}
							</TabContent>
							<TabContent value="triggers" class="flex flex-col flex-1 h-full mt-[-2px]">
								{@render triggers?.()}
							</TabContent>
							{#if flow_json}
								<TabContent value="flow" class="flex flex-col flex-1 h-full">
									{@render pagedGraph()}
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

<!-- Warmed so a tab reopened on the step page has the graph built before the way back is taken;
     the pages are absolutely positioned, so each carries its own scroll. -->
{#snippet pagedGraph()}
	<PagedContent
		warm
		class="h-full"
		current={graphPage}
		onNavigate={(key) => {
			if (key === 'graph') selected = 'saved_inputs'
		}}
		pages={[
			{ key: 'graph', content: graphPageContent },
			{ key: 'step', content: stepPageContent }
		]}
	/>
{/snippet}

{#snippet graphPageContent()}
	<div class="h-full overflow-y-auto flex flex-col">
		{@render flow_graph_render?.()}
	</div>
{/snippet}

{#snippet stepPageContent()}
	<!-- The step body brings its own inner padding; this outer band brings it level with the
	     Inputs and Export tabs. -->
	<div class="min-h-0 grow overflow-y-auto p-2">
		{@render flow_step_render?.({ onBack: () => (selected = 'saved_inputs') })}
	</div>
{/snippet}
