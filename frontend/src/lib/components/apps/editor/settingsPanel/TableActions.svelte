<script lang="ts">
	import { Badge } from '$lib/components/common'
	import Button from '$lib/components/common/button/Button.svelte'
	import { getNextId } from '$lib/components/flows/idUtils'
	import { classNames, generateRandomString } from '$lib/utils'
	import { getContext, onMount } from 'svelte'
	import type { AppViewerContext, BaseAppComponent, RichConfiguration } from '../../types'
	import { appComponentFromType } from '../appUtils'
	import type { ButtonComponent, CheckboxComponent, SelectComponent } from '../component'
	import PanelSection from './common/PanelSection.svelte'
	import { GripVertical, Inspect, List, ToggleRightIcon, ListOrdered } from 'lucide-svelte'
	import { dragHandle, dragHandleZone } from '@windmill-labs/svelte-dnd-action'
	import CloseButton from '$lib/components/common/CloseButton.svelte'
	import { flip } from 'svelte/animate'
	import TableActionsWizard from '$lib/components/wizards/TableActionsWizard.svelte'
	import Alert from '$lib/components/common/alert/Alert.svelte'

	export let components:
		| (BaseAppComponent & (ButtonComponent | CheckboxComponent | SelectComponent))[]
		| undefined

	export let actionsOrder: RichConfiguration | undefined = undefined

	// Migration code:
	onMount(() => {
		if (components === undefined) {
			components = []
		}
	})

	let items =
		components?.map((tab, index) => {
			return { value: tab, id: generateRandomString(), originalIndex: index }
		}) ?? []

	$: components = items.map((item) => item.value)

	export let id: string

	const { selectedComponent, app, errorByComponent, hoverStore } =
		getContext<AppViewerContext>('AppViewerContext')

	function addComponent(typ: 'buttoncomponent' | 'checkboxcomponent' | 'selectcomponent') {
		if (!components) {
			return
		}

		const actionId = getNextId(components.map((x) => x.id.split('_')[1]))

		const newComponent = {
			...appComponentFromType(typ)(`${id}_${actionId}`),
			recomputeIds: []
		}

		if (typ == 'buttoncomponent') {
			if (newComponent?.configuration?.size) {
				// @ts-ignore
				newComponent.configuration.size = { type: 'static', value: 'xs2' }
			}
		}

		items = [
			...items,
			{
				value: newComponent,
				id: generateRandomString(),
				originalIndex: items.length
			}
		]

		components = [...components, newComponent] // $app = $app
	}

	function deleteComponent(cid: string, index: number) {
		if (!components) {
			return
		}
		components = components.filter((x) => x.id !== cid)
		delete $errorByComponent[cid]

		$selectedComponent = [id] // $app = $app
		// Remove the corresponding item from the items array
		items = items.filter((item) => item.originalIndex !== index)
	}

	function handleConsider(e: CustomEvent): void {
		const { items: newItems } = e.detail

		items = newItems
	}
	function handleFinalize(e: CustomEvent): void {
		const { items: newItems } = e.detail

		items = newItems
	}
</script>

{#if components}
	<PanelSection title={`Table Actions`}>
		<svelte:fragment slot="action">
			<TableActionsWizard bind:actionsOrder selectedId={$selectedComponent?.[0] ?? ''} {components}>
				<svelte:fragment slot="trigger">
					<Button
						color="light"
						size="xs2"
						nonCaptureEvent={true}
						btnClasses={actionsOrder ? 'bg-blue-100 dark:bg-blue-900' : 'text-primary'}
						title="Edit order programmatically"
					>
						<div class="flex flex-row items-center gap-2 text-xs font-normal">
							<ListOrdered size={16} />
						</div>
					</Button>
				</svelte:fragment>
			</TableActionsWizard>
		</svelte:fragment>
		{#if components.length == 0}
			<span class="text-xs text-tertiary">No action buttons</span>
		{/if}
		<div class="w-full flex gap-2 flex-col mt-2">
			<section
				use:dragHandleZone={{
					items,
					flipDurationMs: 200,
					dropTargetStyle: {}
				}}
				on:consider={handleConsider}
				on:finalize={handleFinalize}
			>
				{#each items as item, index (item.id)}
					{@const component = items[index].value}

					<div animate:flip={{ duration: 200 }} class="flex flex-row gap-2 items-center mb-2">
						<!-- svelte-ignore a11y-no-static-element-interactions -->
						<!-- svelte-ignore a11y-mouse-events-have-key-events -->
						<div
							class={classNames(
								'w-full text-xs text-semibold truncate py-1.5 px-2 cursor-pointer justify-between flex items-center border rounded-md',
								'bg-surface hover:bg-surface-hover focus:border-primary text-secondary'
							)}
							on:click={() => {
								$selectedComponent = [component.id]
							}}
							on:mouseover={() => {
								$hoverStore = component.id
							}}
							on:keypress
						>
							<div class="flex flex-row gap-2 items-center">
								<Badge color="dark-indigo">
									{component.id}
								</Badge>

								<div>
									{#if component.type == 'buttoncomponent'}
										Button
									{:else if component.type == 'selectcomponent'}
										Select
									{:else if component.type == 'checkboxcomponent'}
										Toggle
									{/if}
								</div>
							</div>
							<div class="flex flex-row items-center gap-1">
								<CloseButton
									small
									on:close={() => deleteComponent(component.id, item.originalIndex)}
								/>
							</div>
						</div>
						{#if actionsOrder === undefined}
							<div use:dragHandle class="handle w-4 h-4" aria-label="drag-handle">
								<GripVertical size={16} />
							</div>
						{/if}
					</div>
				{/each}
			</section>
		</div>
		<div class="w-full flex gap-2">
			<Button
				btnClasses="gap-1 flex items-center text-sm text-tertiary"
				wrapperClasses="w-full"
				color="light"
				variant="border"
				on:click={() => addComponent('buttoncomponent')}
				title="Add Button"
			>
				+ <Inspect size={14} />
			</Button>
			<Button
				btnClasses="gap-1 flex items-center text-sm text-tertiary"
				wrapperClasses="w-full"
				color="light"
				variant="border"
				on:click={() => addComponent('checkboxcomponent')}
				title="Add Toggle"
			>
				+ <ToggleRightIcon size={14} />
			</Button>
			<Button
				btnClasses="gap-1 flex items-center text-sm text-tertiary"
				wrapperClasses="w-full"
				color="light"
				variant="border"
				on:click={() => addComponent('selectcomponent')}
				title="Add Select"
			>
				+ <List size={14} />
			</Button>
		</div>
		<div class="w-full flex flex-col">
			{#if actionsOrder}
				<Alert size="xs" title="Order managed programmatically" type="info">
					Actions order is managed programmatically. Adding or removing an action will require you
					to update the order manually.
					<Button
						btnClasses="mt-2"
						size="xs2"
						on:click={() => {
							actionsOrder = undefined
						}}
					>
						Disable order management
					</Button>
				</Alert>
			{/if}
		</div>
	</PanelSection>
{/if}
