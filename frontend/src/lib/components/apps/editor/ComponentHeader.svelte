<script lang="ts">
	import { classNames } from '$lib/utils'
	import type { AppViewerContext } from '../types'
	import { Anchor, Bug, Expand, Move, Pen, Plug2 } from 'lucide-svelte'
	import { createEventDispatcher, getContext } from 'svelte'
	import Popover from '$lib/components/Popover.svelte'
	import { Button, Popup } from '$lib/components/common'
	import type { AppComponent } from './component'
	import { twMerge } from 'tailwind-merge'
	import { connectOutput } from './appUtils'

	import TabsDebug from './TabsDebug.svelte'
	import ComponentOutputViewer from './contextPanel/ComponentOutputViewer.svelte'

	export let component: AppComponent
	export let selected: boolean
	export let locked: boolean = false
	export let hover: boolean = false
	export let connecting: boolean = false
	export let hasInlineEditor: boolean = false
	export let inlineEditorOpened: boolean = false
	export let errorHandledByComponent: boolean = false

	const dispatch = createEventDispatcher()

	const { errorByComponent, openDebugRun, connectingInput } =
		getContext<AppViewerContext>('AppViewerContext')
</script>

{#if connecting}
	<div class="absolute z-50 left-6 -top-[11px] overflow-auto">
		<Popup floatingConfig={{ strategy: 'fixed', placement: 'bottom-start' }}>
			<svelte:fragment slot="button">
				<button
					id={`connect-output-${component.id}`}
					class="bg-red-500/70 border border-red-600 px-1 py-0.5"
					title="Outputs"
					aria-label="Open output"><Plug2 size={12} /></button
				>
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
	<span
		on:mouseover|stopPropagation={() => {
			dispatch('mouseover')
		}}
		on:mousedown|stopPropagation|capture
		draggable="false"
		title={`Id: ${component.id}`}
		class={twMerge(
			'px-2 text-2xs font-semibold w-fit absolute shadow -top-[9px] -left-[8px] border rounded-sm z-50 cursor-move',
			selected
				? 'bg-indigo-500/90 border-indigo-600 text-white'
				: $connectingInput.opened
				? 'bg-red-500/90 border-red-600 text-white'
				: 'bg-blue-500/90 border-blue-600 text-white'
		)}
	>
		{component.id}
	</span>
{/if}

{#if selected && !connecting}
	<div class="top-[-9px] -right-[8px] flex flex-row absolute gap-1.5 z-50">
		{#if hasInlineEditor}
			<button
				title="Edit"
				class={classNames(
					'px-1 text-2xs py-0.5 font-bold w-fit border cursor-pointer rounded-sm',
					'bg-indigo-100 text-indigo-600 border-indigo-500 hover:bg-indigo-200 hover:text-indigo-800'
				)}
				on:click={() => dispatch('triggerInlineEditor')}
				on:pointerdown|stopPropagation
			>
				{#if inlineEditorOpened}
					<Pen aria-label="Unlock position" size={14} class="text-orange-500" />
				{:else}
					<Pen aria-label="Lock position" size={14} />
				{/if}
			</button>
		{/if}
		{#if component.type === 'conditionalwrapper'}
			<TabsDebug id={component.id} tabs={component.conditions ?? []} isConditionalDebugMode />
		{:else if component.type === 'steppercomponent' || (component.type === 'tabscomponent' && component.configuration.tabsKind.type === 'static' && component.configuration.tabsKind.value === 'invisibleOnView')}
			<TabsDebug id={component.id} tabs={component.tabs ?? []} />
		{/if}
		<button
			title="Expand"
			class={classNames(
				'px-1 text-2xs py-0.5 font-bold w-fit border cursor-pointer rounded-sm',
				'bg-indigo-100 text-indigo-600 border-indigo-500 hover:bg-indigo-200 hover:text-indigo-800'
			)}
			on:click={() => dispatch('expand')}
			on:pointerdown|stopPropagation
		>
			<Expand aria-label="Expand position" size={14} />
		</button>
		<button
			title="Lock Position"
			class={classNames(
				'px-1 text-2xs py-0.5 font-bold w-fit border  rounded-sm cursor-pointer',
				'bg-indigo-100 text-indigo-600 border-indigo-500 hover:bg-indigo-200 hover:text-indigo-800'
			)}
			on:click={() => dispatch('lock')}
			on:pointerdown|stopPropagation
		>
			{#if locked}
				<Anchor aria-label="Unlock position" size={14} class="text-orange-500" />
			{:else}
				<Anchor aria-label="Lock position" size={14} />
			{/if}
		</button>
		<!-- svelte-ignore a11y-no-static-element-interactions -->
		<div
			draggable="false"
			title="Move"
			on:mousedown|stopPropagation|capture
			class={classNames(
				'px-1 text-2xs py-0.5 font-bold w-fit border cursor-move rounded-sm',
				'bg-indigo-100 text-indigo-600 border-indigo-500 hover:bg-indigo-200 hover:text-indigo-800',
				'flex items-center justify-center'
			)}
		>
			<Move size={14} />
		</div>
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
