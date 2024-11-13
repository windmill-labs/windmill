<script lang="ts">
	import { classNames } from '$lib/utils'
	import type { AppViewerContext } from '../types'
	import { Anchor, ArrowDownFromLine, Bug, Network, Pen, Plug } from 'lucide-svelte'
	import { createEventDispatcher, getContext } from 'svelte'
	import Popover from '$lib/components/Popover.svelte'
	import { Button, Popup } from '$lib/components/common'
	import type { AppComponent } from './component'
	import { twMerge } from 'tailwind-merge'
	import { connectOutput } from './appUtils'

	import TabsDebug from './TabsDebug.svelte'
	import ComponentOutputViewer from './contextPanel/ComponentOutputViewer.svelte'
	import DecisionTreeDebug from './DecisionTreeDebug.svelte'
	import AnimatedButton from '$lib/components/common/button/AnimatedButton.svelte'

	export let component: AppComponent
	export let selected: boolean
	export let locked: boolean = false
	export let hover: boolean = false
	export let connecting: boolean = false
	export let hasInlineEditor: boolean = false
	export let inlineEditorOpened: boolean = false
	export let errorHandledByComponent: boolean = false
	export let fullHeight: boolean = false
	export let componentContainerWidth: number
	//export let willNotDisplay: boolean = false

	const DECISION_TREE_THRESHOLD = 300
	const STEPPER_THRESHOLD = 180
	const CONDITIONAL_WRAPPER_THRESHOLD = 200
	const MINIMUM_WIDTH = 20

	let maxWidth = 10
	let isManuallySelected = false
	let componentIsDebugging = false
	let id_width = 0

	$: maxWidth = Math.max(Math.round(0.2 * componentContainerWidth), MINIMUM_WIDTH)

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
</script>

{#if connecting}
	<div
		class="absolute z-50 overflow-auto -top-[18px]"
		style="left: {id_width}px;"
		data-connection-button
	>
		<Popup floatingConfig={{ strategy: 'fixed', placement: 'bottom-start' }}>
			<svelte:fragment slot="button">
				<AnimatedButton
					animate={true}
					baseRadius="9999px"
					wrapperClasses="h-full w-full pt-2"
					marginWidth="2px"
					animationDuration="3s"
				>
					<button
						id={`connect-output-${component.id}`}
						class="h-[20px] w-[20px] bg-blue-800 rounded-full center-center text-white"
						title="Outputs"
						aria-label="Open output"><Plug size={12} /></button
					>
				</AnimatedButton>
			</svelte:fragment>
			<ComponentOutputViewer
				suffix="connect"
				on:select={({ detail }) =>
					connectOutput(connectingInput, component.type, component.id, detail)}
				componentId={component.id}
			/>
		</Popup>
	</div>
{/if}

{#if selected || hover}
	<!-- svelte-ignore a11y-no-static-element-interactions -->
	<!-- svelte-ignore a11y-mouse-events-have-key-events -->
	<div class="-top-[18px] -left-[8px] flex flex-row flex-nowrap w-fit h-fit absolute gap-0.5">
		<div
			on:mouseover|stopPropagation={() => {
				dispatch('mouseover')
			}}
			on:mousedown|stopPropagation|capture
			draggable="false"
			title={`Id: ${component.id}`}
			class={twMerge(
				'py-0.5 text-2xs w-fit h-full min-h-5 border rounded z-50 cursor-move flex flex-row flex-nowrap font-semibold items-center shadow',
				selected
					? 'bg-indigo-500/90 border-indigo-500 text-white'
					: $connectingInput.opened
					? 'bg-red-500/90 border-red-600 text-white'
					: 'bg-blue-500/90 border-blue-600 text-white'
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
								? ' bg-indigo-800 text-indigo-200'
								: 'text-white hover:bg-indigo-700 hover:text-indigo-200'
						)}
						on:click={() => dispatch('fillHeight')}
						on:pointerdown|stopPropagation
					>
						<ArrowDownFromLine aria-label="Expand position" size={11} />
					</button>

					<button
						title="Lock Position"
						class={twMerge(
							'px-1 py-0.5 text-2xs font-bold rounded cursor-pointer w-fit h-full',
							locked
								? ' bg-indigo-800 text-indigo-200'
								: 'text-white hover:bg-indigo-700 hover:text-indigo-200'
						)}
						on:click={() => dispatch('lock')}
						on:pointerdown|stopPropagation
					>
						{#if locked}
							<Anchor aria-label="Unlock position" size={11} />
						{:else}
							<Anchor aria-label="Lock position" size={11} />
						{/if}
					</button>
				</div>
			{/if}
		</div>
		{#if selected && !connecting && checkComponentOptions()}
			<div
				class={twMerge(
					'px-1 py-0.5 text-2xs font-semibold w-fit min-h-5 border shadow rounded z-50 flex flex-row items-center flex-nowrap',
					isManuallySelected || componentIsDebugging
						? 'bg-red-100 text-red-600 border-red-500'
						: 'bg-indigo-100/90 border-indigo-200 text-indigo-600'
				)}
			>
				{#if hasInlineEditor}
					<button
						title="Edit"
						class={twMerge(
							'px-1 py-0.5 text-2xs font-bold rounded cursor-pointer w-fit h-full',
							inlineEditorOpened
								? 'bg-indigo-300 text-indigo-800'
								: 'text-indigo-600 hover:bg-indigo-300 hover:text-indigo-800'
						)}
						on:click={() => dispatch('triggerInlineEditor')}
						on:pointerdown|stopPropagation
					>
						{#if inlineEditorOpened}
							<Pen aria-label="Unlock position" size={11} />
						{:else}
							<Pen aria-label="Lock position" size={11} />
						{/if}
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
							'px-1 py-0.5 text-2xs font-bold rounded cursor-pointer w-fit h-full',
							componentIsDebugging
								? 'text-red-600 hover:bg-red-300 hover:text-red-800'
								: 'text-indigo-600 hover:bg-indigo-300 hover:text-indigo-800'
						)}
						on:click={() => {
							const element = document.getElementById(`decision-tree-graph-editor`)
							if (element) {
								element.click()
							}
						}}
						on:pointerdown|stopPropagation
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

				<!-- svelte-ignore a11y-no-static-element-interactions -->
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
		<Popover notClickable placement="bottom" popupClass="!bg-surface border w-96">
			<Bug size={14} />
			<span slot="text">
				<div class="bg-surface">
					<pre class=" whitespace-pre-wrap text-red-600 bg-surface border w-full p-4 text-xs mb-2"
						>{error ?? ''}	
								</pre>
				</div>
				<Button
					color="red"
					variant="border"
					on:click={() => $openDebugRun?.($errorByComponent[component.id]?.id ?? '')}
					>Open Debug Runs</Button
				>
			</span>
		</Popover>
	</span>
{/if}
