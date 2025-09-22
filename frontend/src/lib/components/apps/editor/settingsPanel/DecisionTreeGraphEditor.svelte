<script lang="ts">
	import { Alert, Button, Drawer, DrawerContent } from '$lib/components/common'
	import { Network, Trash } from 'lucide-svelte'
	import type { AppComponent, DecisionTreeNode } from '../component'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import { getContext, setContext } from 'svelte'
	import InputsSpecEditor from './InputsSpecEditor.svelte'
	import Section from '$lib/components/Section.svelte'
	import { writable } from 'svelte/store'
	import DecisionTreePreview from './decisionTree/DecisionTreePreview.svelte'
	import { removeNode } from './decisionTree/utils'
	import Label from '$lib/components/Label.svelte'
	import { debounce } from '$lib/utils'
	import type { AppViewerContext } from '../../types'
	import Badge from '$lib/components/common/badge/Badge.svelte'

	interface Props {
		component: AppComponent
		nodes: DecisionTreeNode[]
	}

	let { component = $bindable(), nodes = $bindable() }: Props = $props()

	let drawer: Drawer | undefined = $state(undefined)
	let paneWidth = $state(0)
	let paneHeight = $state(0)
	let renderCount = $state(0)

	const { debuggingComponents } = getContext<AppViewerContext>('AppViewerContext')

	const selectedNodeId = writable<string | undefined>(undefined)

	const { debounced: debouncedNodes } = debounce(() => {
		nodes = nodes
		renderCount++
	}, 300)

	let selectedNode = $derived(nodes?.find((node) => node.id == $selectedNodeId))

	setContext('DecisionTreeEditor', { selectedNodeId })
</script>

<Drawer bind:this={drawer} size="1800px" on:close={() => {}} on:open={() => {}}>
	<DrawerContent
		title="Decision tree"
		on:close={drawer.closeDrawer}
		noPadding
		forceOverflowVisible
		tooltip="Decision tree graph editor"
	>
		<Splitpanes>
			<Pane size={60}>
				<div class="w-full h-full" bind:clientWidth={paneWidth} bind:clientHeight={paneHeight}>
					{#if paneWidth && paneHeight}
						<DecisionTreePreview
							bind:nodes
							bind:component
							{paneHeight}
							{paneWidth}
							onrender={() => {
								renderCount++
							}}
						/>
					{/if}
				</div>
			</Pane>
			<Pane size={40}>
				<!-- svelte-ignore a11y_no_static_element_interactions -->
				<div
					class="h-full w-full bg-surface p-4 flex flex-col gap-6"
					onkeydown={(e) => {
						// Prevent keyboard events from bubbling to SvelteFlow
						e.stopPropagation()
					}}
				>
					{#if selectedNode}
						<Section label="Conditions" class="w-full flex flex-col gap-2">
							{#snippet action()}
								<Button
									size="xs"
									color="light"
									startIcon={{ icon: Trash }}
									variant="border"
									on:click={() => {
										nodes = removeNode(nodes, selectedNode)

										$debuggingComponents = Object.fromEntries(
											Object.entries($debuggingComponents).filter(([key]) => key !== component.id)
										)

										renderCount++
									}}
									disabled={selectedNode?.next?.length > 1 || nodes.length == 1}
								>
									Delete node
								</Button>
							{/snippet}

							<Label label="Summary">
								<input
									type="text"
									class="input input-primary input-bordered"
									bind:value={selectedNode.label}
									oninput={() => {
										debouncedNodes()
									}}
									onkeydown={(e) => {
										// Prevent keyboard events from bubbling to SvelteFlow
										e.stopPropagation()
									}}
								/>
							</Label>

							{#if selectedNode.next.length > 1}
								{#each selectedNode.next ?? [] as subNode, index (subNode.id)}
									{#if subNode.condition}
										<div class="flex flex-row gap-4 items-center w-full justify-center">
											<div class="grow relative">
												<InputsSpecEditor
													key={`condition-${selectedNode.id}-${index}`}
													customTitle={index === 0
														? 'Goes to branch 1'
														: `${index > 0 ? 'Otherwise g' : 'G'}oes to branch ${index + 1}`}
													bind:componentInput={subNode.condition}
													id={selectedNode.id}
													userInputEnabled={false}
													shouldCapitalize={true}
													resourceOnly={false}
													fieldType={'boolean'}
													subFieldType={undefined}
													format={undefined}
													selectOptions={undefined}
													tooltip={undefined}
													fileUpload={undefined}
													placeholder={undefined}
													displayType={false}
													fixedOverflowWidgets={false}
												/>
												{#if index == selectedNode.next.length - 1}
													<div class="text-xs text-secondary">
														If no branch evaluates to true, clicking next will show an error toast.
													</div>
												{/if}

												<div class="flex flex-row gap-1 mt-2">
													<Badge>
														{`Next node id: ${subNode.id}`}
													</Badge>
													<Badge color="indigo">
														{`Next tab index: ${nodes?.findIndex((node) => node.id == subNode.id)}`}
													</Badge>
												</div>
											</div>
										</div>
									{/if}
								{/each}
								<Alert type="info" class="mt-4" title="Multiple branches" size="xs">
									The conditions above are evaluated in order. The first condition that is met will
									be the branch that is taken.
								</Alert>
							{/if}
							{#key selectedNode.id}
								{#if selectedNode.allowed}
									<InputsSpecEditor
										customTitle={`Can proceed to next step if:`}
										key={`allowed-${selectedNode.id}`}
										bind:componentInput={selectedNode.allowed}
										id={component.id}
										userInputEnabled={false}
										shouldCapitalize={true}
										fieldType={'boolean'}
										subFieldType={undefined}
										format={undefined}
										selectOptions={undefined}
										tooltip={undefined}
										fileUpload={undefined}
										placeholder={undefined}
										resourceOnly={false}
										displayType={false}
										fixedOverflowWidgets={false}
									/>
								{/if}
							{/key}
						</Section>
					{/if}
				</div>
			</Pane>
		</Splitpanes>
	</DrawerContent>
</Drawer>

<div class="p-2 flex flex-col gap-2">
	<Button
		title="Decision tree graph editor"
		id="decision-tree-graph-editor"
		on:click={() => {
			drawer?.openDrawer()
		}}
		size="xs"
		color="dark"
		startIcon={{ icon: Network }}
	>
		Graph editor
	</Button>
</div>
