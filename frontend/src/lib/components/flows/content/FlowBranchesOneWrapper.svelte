<script lang="ts">
	import { Badge, Tab, Tabs } from '$lib/components/common'
	import { refreshStateStore } from '$lib/svelte5Utils.svelte'
	import { GripVertical, Plus, Trash2 } from 'lucide-svelte'
	import { getContext } from 'svelte'
	import type { FlowEditorContext } from '../types'
	import {
		addBranch as addBranchOp,
		removeBranch as removeBranchOp,
		reorderBranches as reorderBranchesOp,
		graphBranchIndex
	} from '../branchOps'
	import Button from '$lib/components/common/button/Button.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import { dragHandle, dragHandleZone } from '@windmill-labs/svelte-dnd-action'
	import { randomUUID } from '$lib/utils/uuid'
	import StepSettingsBadges from './StepSettingsBadges.svelte'
	import { tick, untrack } from 'svelte'

	import type { BranchOne, FlowModule } from '$lib/gen'
	import FlowCard from '../common/FlowCard.svelte'
	import BranchPredicateEditor from './BranchPredicateEditor.svelte'
	import FlowRunSettings from './FlowRunSettings.svelte'
	import { useUiIntent } from '$lib/components/copilot/chat/flow/useUiIntent'

	interface Props {
		flowModule: FlowModule
		previousModule: FlowModule | undefined
		parentModule: FlowModule | undefined
		noEditor: boolean
		enableAi?: boolean
	}

	let {
		flowModule = $bindable(),
		previousModule,
		parentModule,
		noEditor,
		enableAi = false
	}: Props = $props()

	let value = $state(flowModule.value as BranchOne)
	$effect(() => {
		value = flowModule.value as BranchOne
	})

	// dnd needs a stable id per item; branches have none and must not gain one (it would
	// land in the saved flow), so ids are held beside them, keyed by object identity.
	const branchIds = new WeakMap<object, string>()
	function idFor(branch: object): string {
		let id = branchIds.get(branch)
		if (!id) {
			id = randomUUID()
			branchIds.set(branch, id)
		}
		return id
	}

	let items = $state(value.branches.map((b) => ({ id: idFor(b), branch: b })))
	// dnd owns `items` for the length of a gesture: mid-drag it holds a shadow placeholder
	// alongside the real entries, so rebuilding from `value.branches` there would splice a
	// second copy of the dragged branch into the list (duplicate keys).
	let dragging = false

	$effect(() => {
		const next = value.branches.map((b) => ({ id: idFor(b), branch: b }))
		// untrack: this reads and writes `items`, which would otherwise re-invalidate itself.
		untrack(() => {
			if (dragging) return
			const same = next.length === items.length && next.every((it, i) => it.id === items[i].id)
			if (!same) items = next
		})
	})

	function handleConsider(e: CustomEvent<{ items: typeof items }>) {
		dragging = true
		items = e.detail.items
	}
	function handleFinalize(e: CustomEvent<{ items: typeof items }>) {
		items = e.detail.items
		reorderBranchesOp(
			flowModule.id,
			items.map((it) => it.branch),
			{ flowStore, history }
		)
		dragging = false
	}

	const { flowStore, flowStateStore, history } = getContext<FlowEditorContext>('FlowEditorContext')

	function addBranch() {
		addBranchOp(flowModule.id, { flowStore, history })
		refreshStateStore(flowStore)
	}
	function removeBranch(arrayIndex: number) {
		// The shared op counts branches the way the graph does; see graphBranchIndex.
		removeBranchOp(flowModule.id, graphBranchIndex(value.type, arrayIndex), {
			flowStore,
			flowStateStore,
			history
		})
		refreshStateStore(flowStore)
	}

	let runSettings: FlowRunSettings | undefined = $state(undefined)
	let selectedTab = $state('branches')

	useUiIntent(`branchone-${flowModule.id}`, {
		openTab: async (tab) => {
			// Every setting the intent can name lives in the other tab, which only mounts
			// `runSettings` once selected.
			selectedTab = 'settings'
			await tick()
			runSettings?.openSetting(tab)
		}
	})
</script>

<div class="h-full" id="flow-editor-branch-one-wrapper">
	<FlowCard
		{noEditor}
		title="Run one branch"
		subtitle="The first branch whose predicate is true runs. The result of this step is that branch's result."
		subtitleDocLink="https://www.windmill.dev/docs/flows/flow_branches#branch-one"
	>
		<div class="flex h-full min-h-0 flex-col">
			<Tabs bind:selected={selectedTab} wrapperClass="shrink-0">
				<Tab value="branches" label="Branches" />
				<Tab value="settings" label="Run settings">
					{#snippet extra()}
						<StepSettingsBadges {flowModule} />
					{/snippet}
				</Tab>
			</Tabs>

			<div
				class="flex min-h-0 flex-1 flex-col gap-6 overflow-auto p-4"
				style="scrollbar-gutter: stable"
			>
				{#if selectedTab === 'branches'}
					<section>
						<div class="flex flex-col gap-3">
							<section
								class="flex flex-col gap-3"
								use:dragHandleZone={{ items, flipDurationMs: 150, dropTargetStyle: {} }}
								onconsider={handleConsider}
								onfinalize={handleFinalize}
							>
								{#each items as item, i (item.id)}
									<!-- The handle and the delete button each own a column, so the predicate
									     below lines up with the summary instead of running under them. -->
									<div
										class="grid grid-cols-[auto_minmax(0,1fr)_auto] items-center gap-x-2 gap-y-0 rounded-md bg-surface-tertiary p-3 shadow-sm"
									>
										<!-- svelte-ignore a11y_no_static_element_interactions -->
										<div
											class="cursor-move text-tertiary hover:text-primary"
											use:dragHandle
											aria-label="Reorder branch"
										>
											<GripVertical size={16} />
										</div>
										<div class="flex min-w-0 items-center gap-2">
											<Badge color="blue" class="text-xs">Branch {i + 1}</Badge>
											<TextInput
												size="sm"
												class="grow"
												bind:value={
													() => item.branch.summary ?? '', (v) => (item.branch.summary = String(v))
												}
												inputProps={{ placeholder: 'Summary' }}
											/>
										</div>
										<Button
											unifiedSize="sm"
											variant="subtle"
											destructive
											iconOnly
											startIcon={{ icon: Trash2 }}
											title="Delete branch"
											on:click={() => removeBranch(i)}
										/>
										<div class="col-start-2 py-2">
											<BranchPredicateEditor
												branch={item.branch}
												parentModule={flowModule}
												{previousModule}
												{enableAi}
											/>
										</div>
									</div>
								{/each}
							</section>
							<div class="flex items-center gap-2 rounded-md bg-surface-tertiary p-3 shadow-sm">
								<Badge color="blue" class="text-xs">Default</Badge>
								<p class="text-xs italic text-tertiary">Runs if none of the above match</p>
							</div>
						</div>
						<Button
							unifiedSize="sm"
							variant="default"
							startIcon={{ icon: Plus }}
							wrapperClasses="mt-4 self-start"
							on:click={addBranch}
						>
							Add branch
						</Button>
					</section>
				{:else}
					<FlowRunSettings
						embedded
						loopSubset
						bind:this={runSettings}
						bind:flowModule
						{parentModule}
						{previousModule}
						selectedId={flowModule.id}
					/>
				{/if}
			</div>
		</div>
	</FlowCard>
</div>
