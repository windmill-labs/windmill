<script lang="ts">
	import { Button, Drawer, DrawerContent } from '$lib/components/common'
	import { Network, Plus, Trash } from 'lucide-svelte'
	import type { AppComponent, DecisionTreeNode } from '../component'
	import { Pane, Splitpanes } from 'svelte-splitpanes'

	import { setContext } from 'svelte'
	import InputsSpecEditor from './InputsSpecEditor.svelte'

	import Alert from '$lib/components/common/alert/Alert.svelte'
	import Section from '$lib/components/Section.svelte'
	import { writable } from 'svelte/store'
	import DecisionTreePreview from './decisionTree/DecisionTreePreview.svelte'
	import { addBranch, removeNode } from './decisionTree/utils'

	export let component: AppComponent
	export let nodes: DecisionTreeNode[]
	export let minHeight: number = 0
	export let rebuildOnChange: any = undefined

	let drawer: Drawer | undefined = undefined

	let paneWidth = 0
	let paneHeight = 0

	const selectedNodeId = writable<string | undefined>(undefined)

	$: selectedNode = nodes?.find((node) => node.id == $selectedNodeId)

	setContext('DecisionTreeEditor', { selectedNodeId })

	let renderCount = 0
</script>

<Drawer bind:this={drawer} on:close={() => {}} on:open={() => {}} size="1200px">
	<DrawerContent
		title="Decision tree"
		on:close={drawer.closeDrawer}
		noPadding
		tooltip="Decision tree graph editor"
	>
		<Splitpanes>
			<Pane size={50}>
				<div class="w-full h-full" bind:clientWidth={paneWidth} bind:clientHeight={paneHeight}>
					{#key renderCount}
						<DecisionTreePreview
							bind:nodes
							{minHeight}
							{rebuildOnChange}
							{paneHeight}
							{paneWidth}
							on:render={() => {
								renderCount++
							}}
						/>
					{/key}
				</div>
			</Pane>
			<Pane size={50}>
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
									disabled={!Boolean(
										nodes.find((n) => n.next.find((next) => next.id == selectedNode?.id))
									) ||
										nodes.filter((n) =>
											n.next.find((nn) => {
												return nn.id == selectedNode?.id
											})
										)?.length > 1}
								>
									Delete node
								</Button>
							</svelte:fragment>

							{#if Array.isArray(selectedNode.next) && selectedNode.next.length === 1}
								<Alert type="info" title="This node has only one next node">
									This node goes to the node {selectedNode.next[0].id} if the following You can add a
									new node by clicking on the "Add step" button in the top right corner of the node.
								</Alert>

								<div class="flex flex-start">
									<Button
										size="xs"
										color="dark"
										startIcon={{ icon: Plus }}
										on:click={() => {
											if (selectedNode) {
												nodes = addBranch(nodes, selectedNode)
												renderCount++
											}
										}}
									>
										Add branch
									</Button>
								</div>
							{:else}
								{#each selectedNode.next as subNode (subNode.id)}
									{#if selectedNode.required}
										<div class="flex flex-row gap-4 items-center w-full justify-center">
											<div class="grow">
												<InputsSpecEditor
													key={`Goes to ${subNode.id} if:`}
													bind:componentInput={selectedNode.required}
													id={subNode.id}
													userInputEnabled={false}
													shouldCapitalize={true}
													resourceOnly={false}
													fieldType={selectedNode.required?.['fieldType']}
													subFieldType={selectedNode.required?.['subFieldType']}
													format={selectedNode.required?.['format']}
													selectOptions={selectedNode.required?.['selectOptions']}
													tooltip={selectedNode.required?.['tooltip']}
													fileUpload={selectedNode.required?.['fileUpload']}
													placeholder={selectedNode.required?.['placeholder']}
													customTitle={selectedNode.required?.['customTitle']}
													displayType={false}
												/>
											</div>
										</div>
									{/if}
								{/each}
							{/if}

							{#if selectedNode.next.length > 1}
								<div class="flex flex-row gap-2 mt-4">
									<Button
										size="xs"
										color="light"
										startIcon={{ icon: Network }}
										variant="border"
										on:click={() => {}}
									>
										Add branch
									</Button>
								</div>
							{/if}

							{#if selectedNode.required}
								<InputsSpecEditor
									key={`Can proceed to next step if:`}
									bind:componentInput={selectedNode.required}
									id={'sad'}
									userInputEnabled={false}
									shouldCapitalize={true}
									resourceOnly={false}
									fieldType={selectedNode.required?.['fieldType']}
									subFieldType={selectedNode.required?.['subFieldType']}
									format={selectedNode.required?.['format']}
									selectOptions={selectedNode.required?.['selectOptions']}
									tooltip={selectedNode.required?.['tooltip']}
									fileUpload={selectedNode.required?.['fileUpload']}
									placeholder={selectedNode.required?.['placeholder']}
									customTitle={selectedNode.required?.['customTitle']}
									displayType={false}
								/>
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
		on:click={() => {
			drawer?.openDrawer()
		}}
		size="xs"
		color="light"
		startIcon={{ icon: Network }}
	>
		Graph editor
	</Button>
</div>
