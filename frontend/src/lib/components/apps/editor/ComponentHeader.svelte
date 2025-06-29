<script lang="ts">
	import { stopPropagation, createBubbler } from 'svelte/legacy'

	const bubble = createBubbler()
	import { classNames } from '$lib/utils'
	import type { AppViewerContext } from '../types'
	import { Anchor, ArrowDownFromLine, Bug, Expand, Network, Pen, Plug } from 'lucide-svelte'
	import { createEventDispatcher, getContext } from 'svelte'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import { Button } from '$lib/components/common'
	import type { AppComponent } from './component'
	import { twMerge } from 'tailwind-merge'
	import { connectOutput } from './appUtils'

	import TabsDebug from './TabsDebug.svelte'
	import ComponentOutputViewer from './contextPanel/ComponentOutputViewer.svelte'
	import DecisionTreeDebug from './DecisionTreeDebug.svelte'
	import AnimatedButton from '$lib/components/common/button/AnimatedButton.svelte'

	interface Props {
		component: AppComponent
		selected: boolean
		locked?: boolean
		hover?: boolean
		connecting?: boolean
		hasInlineEditor?: boolean
		inlineEditorOpened?: boolean
		errorHandledByComponent?: boolean
		fullHeight?: boolean
		componentContainerWidth: number
	}

	let {
		component,
		selected,
		locked = false,
		hover = false,
		connecting = false,
		hasInlineEditor = false,
		inlineEditorOpened = false,
		errorHandledByComponent = false,
		fullHeight = false,
		componentContainerWidth
	}: Props = $props()

	const DECISION_TREE_THRESHOLD = 300
	const STEPPER_THRESHOLD = 180
	const CONDITIONAL_WRAPPER_THRESHOLD = 200
	const MINIMUM_WIDTH = 20

	let maxWidth = $state(10)
	let isManuallySelected = $state(false)
	let componentIsDebugging = $state(false)
	let id_width = $state(0)

	$effect(() => {
		maxWidth = Math.max(Math.round(0.2 * componentContainerWidth), MINIMUM_WIDTH)
	})

	const dispatch = createEventDispatcher()

	const { errorByComponent, openDebugRun, connectingInput } =
		getContext<AppViewerContext>('AppViewerContext')

	// Function to check if any condition is met
	function checkComponentOptions(): boolean {
		const componentOptions = {
			isEditable: hasInlineEditor,
			isConditionalWrapper: component.type === 'conditionalwrapper',
			hasTabs:
				component.type === 'steppercomponent' ||
				(component.type === 'tabscomponent' &&
					component.configuration.tabsKind.type === 'static' &&
					component.configuration.tabsKind.value === 'invisibleOnView'),
			isDecisionTree: component.type === 'decisiontreecomponent'
		}

		return Object.values(componentOptions).some((value) => value)
	}

	let hoverHeader = $state(false)
</script>

{#if connecting}
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<!-- svelte-ignore a11y_mouse_events_have_key_events -->
	<div
		class="absolute z-50 overflow-auto -top-[18px]"
		style="left: {id_width}px;"
		data-connection-button
	>
		<Popover
			floatingConfig={{ strategy: 'fixed', placement: 'bottom-start' }}
			closeOnOtherPopoverOpen
			contentClasses="p-4"
		>
			{#snippet trigger()}
				<AnimatedButton
					animate={true}
					baseRadius="9999px"
					wrapperClasses="h-full w-full pt-2"
					marginWidth="2px"
					animationDuration="3s"
				>
					<div
						id={`connect-output-${component.id}`}
						class="h-[20px] w-[20px] bg-surface rounded-full center-center text-primary"
						title="Outputs"
						aria-label="Open output"><Plug size={12} /></div
					>
				</AnimatedButton>
			{/snippet}
			{#snippet content()}
				<ComponentOutputViewer
					suffix="connect"
					on:select={({ detail }) =>
						connectOutput(connectingInput, component.type, component.id, detail)}
					componentId={component.id}
				/>
			{/snippet}
		</Popover>
	</div>
{/if}

{#if selected || hover}
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<!-- svelte-ignore a11y_mouse_events_have_key_events -->
	<div class="-top-[18px] -left-[8px] flex flex-row flex-nowrap w-fit h-fit absolute gap-0.5">
		<div
			onmouseover={stopPropagation(() => {
				hoverHeader = true
				dispatch('mouseover')
			})}
			onmouseleave={stopPropagation(() => {
				hoverHeader = false
			})}
			onmousedowncapture={stopPropagation(bubble('mousedown'))}
			draggable="false"
			title={`Id: ${component.id}`}
			class={twMerge(
				'py-0.5 text-2xs w-fit h-full min-h-5 rounded z-50 cursor-move flex flex-row flex-nowrap font-semibold items-center shadow',
				selected
					? 'bg-blue-600/90 text-white'
					: $connectingInput.opened
						? 'bg-[#f8aa4b]/90  text-white'
						: 'bg-blue-400/90 text-white'
			)}
		>
			<div
				class={`px-1 text-2xs w-full min-w-4 h-full truncate`}
				style="max-width: {maxWidth}px;"
				bind:clientWidth={id_width}
			>
				{component.id}
			</div>
			{#if !connecting}
				<div class="flex flex-row px-1 py-0 gap-0.5">
					<button
						title="Fill height"
						class={twMerge(
							'px-1 py-0.5 text-2xs font-bold rounded cursor-pointer w-fit h-full',
							fullHeight
								? 'bg-blue-300 text-blue-800'
								: 'text-white hover:bg-blue-400 hover:text-white'
						)}
						onclick={() => dispatch('fillHeight')}
						onpointerdown={stopPropagation(bubble('pointerdown'))}
					>
						<ArrowDownFromLine aria-label="Full height" size={11} />
					</button>

					<button
						title="Lock Position"
						class={twMerge(
							'px-1 py-0.5 text-2xs font-bold rounded cursor-pointer w-fit h-full',
							locked ? 'bg-blue-300 text-blue-800' : 'text-white hover:bg-blue-400 hover:text-white'
						)}
						onclick={() => dispatch('lock')}
						onpointerdown={stopPropagation(bubble('pointerdown'))}
					>
						{#if locked}
							<Anchor aria-label="Unlock position" size={11} />
						{:else}
							<Anchor aria-label="Lock position" size={11} />
						{/if}
					</button>
					{#if hoverHeader}
						<button
							title="Expand"
							class={twMerge(
								'px-1 py-0.5 text-2xs font-bold rounded cursor-pointer w-fit h-full text-white hover:bg-blue-400 hover:text-white'
							)}
							onclick={() => dispatch('expand')}
							onpointerdown={stopPropagation(bubble('pointerdown'))}
						>
							<Expand aria-label="Expand" size={11} />
						</button>
					{/if}
				</div>
			{/if}
		</div>
		{#if selected && !connecting && checkComponentOptions()}
			<div
				class={twMerge(
					'px-1 py-0.5 text-2xs font-semibold w-fit min-h-5 shadow rounded z-50 flex flex-row items-center flex-nowrap',
					isManuallySelected || componentIsDebugging
						? 'bg-red-100 text-red-600 border-red-500'
						: 'bg-blue-100/90 border-blue-200 text-blue-600'
				)}
			>
				{#if hasInlineEditor}
					<button
						title="Edit"
						class={twMerge(
							'px-1 py-0.5 text-2xs font-bold rounded cursor-pointer w-fit h-full',
							inlineEditorOpened
								? 'bg-blue-300 text-blue-800'
								: 'text-blue-600 hover:bg-blue-300 hover:text-blue-800'
						)}
						onclick={() => dispatch('triggerInlineEditor')}
						onpointerdown={stopPropagation(bubble('pointerdown'))}
					>
						<Pen aria-label="Edit" size={11} />
					</button>
				{/if}
				{#if component.type === 'conditionalwrapper'}
					<TabsDebug
						id={component.id}
						tabs={component.conditions ?? []}
						isConditionalDebugMode
						isSmall={componentContainerWidth < CONDITIONAL_WRAPPER_THRESHOLD}
						bind:isManuallySelected
					/>
				{:else if component.type === 'steppercomponent' || (component.type === 'tabscomponent' && component.configuration.tabsKind.type === 'static' && component.configuration.tabsKind.value === 'invisibleOnView')}
					<TabsDebug
						id={component.id}
						tabs={component.tabs ?? []}
						isSmall={componentContainerWidth < STEPPER_THRESHOLD}
						bind:isManuallySelected
					/>
				{:else if component.type === 'decisiontreecomponent'}
					<button
						title={'Open Decision Tree Editor'}
						class={twMerge(
							'px-1 py-0.5 text-2xs font-bold rounded cursor-pointer w-fit h-full center-center',
							componentIsDebugging
								? 'text-red-600 hover:bg-red-300 hover:text-red-800'
								: 'text-blue-600 hover:bg-blue-300 hover:text-blue-800'
						)}
						onclick={() => {
							const element = document.getElementById(`decision-tree-graph-editor`)
							if (element) {
								element.click()
							}
						}}
						onpointerdown={stopPropagation(bubble('pointerdown'))}
					>
						<Network size={11} />
					</button>
					<DecisionTreeDebug
						id={component.id}
						nodes={component.nodes ?? []}
						isSmall={componentContainerWidth < DECISION_TREE_THRESHOLD}
						bind:componentIsDebugging
					/>
				{/if}

				<!-- {#if willNotDisplay}
				<Popover>
					<svelte:fragment slot="text">
						This component won't render, because an other component above it is set to fill the
						height.
					</svelte:fragment>
					<div
						title="Fill height"
						class={classNames(
							'px-1 text-2xs py-0.5 font-bold w-fit border cursor-pointer rounded-sm',
							'bg-red-100 text-red-600 border-red-500 hover:bg-red-200 hover:text-red-800'
						)}
					>
						<EyeOff aria-label="Expand position" size={14} />
					</div>
				</Popover>
			{/if} -->

				<!-- svelte-ignore a11y_no_static_element_interactions -->
			</div>
		{/if}
	</div>
{/if}

{#if !errorHandledByComponent && $errorByComponent[component.id]}
	{@const error = $errorByComponent[component.id]?.error}
	<span
		title="Error"
		class={classNames(
			'text-red-500 px-1 text-2xs py-0.5 font-bold w-fit absolute border border-red-500 -bottom-1  shadow left-1/2 transform -translate-x-1/2 z-50 cursor-pointer'
		)}
	>
		<Popover notClickable placement="bottom" contentClasses="!bg-surface border w-96 p-4">
			{#snippet trigger()}
				<Bug size={14} />
			{/snippet}
			{#snippet content()}
				<div class="bg-surface">
					<pre class="whitespace-pre-wrap text-red-600 bg-surface border w-full p-4 text-xs mb-2"
						>{error ?? ''}	
							</pre>
				</div>
				<Button
					color="red"
					variant="border"
					on:click={() => $openDebugRun?.($errorByComponent[component.id]?.id ?? '')}
					>Open Debug Runs</Button
				>
			{/snippet}
		</Popover>
	</span>
{/if}
