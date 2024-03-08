<script lang="ts">
	import { Button, Drawer, DrawerContent } from '$lib/components/common'
	import { Network, Plus, Trash } from 'lucide-svelte'
	import type { AppComponent, DecisionTreeNode } from '../component'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import { setContext } from 'svelte'
	import InputsSpecEditor from './InputsSpecEditor.svelte'
	import Section from '$lib/components/Section.svelte'
	import { writable } from 'svelte/store'
	import DecisionTreePreview from './decisionTree/DecisionTreePreview.svelte'
	import { addNewBranch, removeNode } from './decisionTree/utils'
	import Label from '$lib/components/Label.svelte'
	import { debounce } from '$lib/utils'

	export let component: AppComponent
	export let nodes: DecisionTreeNode[]
	export let rebuildOnChange: any = undefined

	let drawer: Drawer | undefined = undefined
	let paneWidth = 0
	let paneHeight = 0
	let renderCount = 0

	const selectedNodeId = writable<string | undefined>(undefined)

	$: selectedNode = nodes?.find((node) => node.id == $selectedNodeId)

	setContext('DecisionTreeEditor', { selectedNodeId })
</script>

<Drawer bind:this={drawer} on:close={() => {}} on:open={() => {}} size="1200px">
	<DrawerContent
		title="Decision tree"
		on:close={drawer.closeDrawer}
		noPadding
		tooltip="Decision tree graph editor"
	>
		<Splitpanes>
			<Pane size={60}>
				<div class="w-full h-full" bind:clientWidth={paneWidth} bind:clientHeight={paneHeight}>
					<DecisionTreePreview
						bind:nodes
						bind:component
						{rebuildOnChange}
						{paneHeight}
						{paneWidth}
						on:render={() => {
							renderCount++
						}}
					/>
				</div>
			</Pane>
			<Pane size={40}>
				<div class="h-full w-full bg-surface p-4 flex flex-col gap-6">
					{#if selectedNode}
						<Section label="Conditions" class="w-full flex flex-col gap-2">
							<svelte:fragment slot="action">
								<Button
									size="xs"
									color="light"
									startIcon={{ icon: Trash }}
									variant="border"
									on:click={() => {
										nodes = removeNode(nodes, selectedNode)
										renderCount++
									}}
									disabled={selectedNode?.next?.length > 1}
								>
									Delete node
								</Button>
							</svelte:fragment>

							<Label label="Summary">
								<input
									type="text"
									class="input input-primary input-bordered"
									bind:value={selectedNode.label}
									on:input={() => {
										debounce(() => {
											renderCount++
										}, 300)()
									}}
								/>
							</Label>

							{#if selectedNode.next.length > 1}
								{#each selectedNode.next.sort((n1, n2) => {
									return n1.id.localeCompare(n2.id)
								}) as subNode, index (subNode.id)}
									{#if subNode.condition}
										<div class="flex flex-row gap-4 items-center w-full justify-center">
											<div class="grow relative">
												<InputsSpecEditor
													key={`condition-${selectedNode.id}-${index}`}
													customTitle={`${index > 0 ? 'Otherwise ' : ''}Goes to branch ${
														index + 1
													} (First node: ${nodes?.findIndex((node) => node.id == subNode.id)}) if:`}
													bind:componentInput={subNode.condition}
													id={selectedNode.id}
													userInputEnabled={false}
													shouldCapitalize={true}
													resourceOnly={false}
													fieldType={subNode.condition?.['fieldType']}
													subFieldType={subNode.condition?.['subFieldType']}
													format={subNode.condition?.['format']}
													selectOptions={subNode.condition?.['selectOptions']}
													tooltip={subNode.condition?.['tooltip']}
													fileUpload={subNode.condition?.['fileUpload']}
													placeholder={subNode.condition?.['placeholder']}
													displayType={false}
													fixedOverflowWidgets={false}
												/>
											</div>
										</div>
									{/if}
								{/each}
							{/if}
							{#key selectedNode.id}
								{#if selectedNode.allowed}
									<InputsSpecEditor
										key={`Can proceed to next step if:`}
										bind:componentInput={selectedNode.allowed}
										id={'allowed'}
										userInputEnabled={false}
										shouldCapitalize={true}
										resourceOnly={false}
										fieldType={selectedNode.allowed?.['fieldType']}
										subFieldType={selectedNode.allowed?.['subFieldType']}
										format={selectedNode.allowed?.['format']}
										selectOptions={selectedNode.allowed?.['selectOptions']}
										tooltip={selectedNode.allowed?.['tooltip']}
										fileUpload={selectedNode.allowed?.['fileUpload']}
										placeholder={selectedNode.allowed?.['placeholder']}
										customTitle={selectedNode.allowed?.['customTitle']}
										displayType={false}
										fixedOverflowWidgets={false}
									/>
								{/if}
							{/key}

							{#if selectedNode?.next.length > 0}
								<div>
									<Button
										startIcon={{ icon: Plus }}
										color="light"
										variant="border"
										size="xs"
										on:click={() => {
											if (!selectedNode) return

											nodes = addNewBranch(nodes, selectedNode)
											renderCount++
										}}
									>
										Add branch
									</Button>
								</div>
							{/if}
						</Section>
					{/if}
				</div>
			</Pane>
		</Splitpanes>
	</DrawerContent>
</Drawer>

<div class="p-2 flex flex-col gap-2">
	<Button
		tooltip="Decision tree graph editor"
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
