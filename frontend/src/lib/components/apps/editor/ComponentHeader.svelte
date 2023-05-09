<script lang="ts">
	import { classNames } from '$lib/utils'
	import type { AppViewerContext } from '../types'
	import { Anchor, Bug, Expand, Move, Pen } from 'lucide-svelte'
	import { createEventDispatcher, getContext } from 'svelte'
	import Popover from '$lib/components/Popover.svelte'
	import { Alert, Button } from '$lib/components/common'
	import type { AppComponent } from './component'
	import { twMerge } from 'tailwind-merge'
	import { getErrorFromLatestResult } from './appUtils'
	import ButtonDropdown from '$lib/components/common/button/ButtonDropdown.svelte'
	import { MenuItem } from '@rgossiaux/svelte-headlessui'

	export let component: AppComponent
	export let selected: boolean
	export let locked: boolean = false
	export let hover: boolean = false
	export let shouldHideActions: boolean = false
	export let hasInlineEditor: boolean = false
	export let inlineEditorOpened: boolean = false

	let isConditionalWrapperManuallySelected: boolean = false

	const dispatch = createEventDispatcher()

	const { errorByComponent, openDebugRun, jobs, connectingInput, componentControl } =
		getContext<AppViewerContext>('AppViewerContext')

	$: error = getErrorFromLatestResult(component.id, $errorByComponent, $jobs)

	function openDebugRuns() {
		if ($openDebugRun) {
			$openDebugRun(component.id)
		}
	}
</script>

{#if selected || hover}
	<span
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

{#if selected && !shouldHideActions}
	<div class="top-[-9px] -right-[8px] flex flex-row absolute gap-1.5 z-50">
		{#if hasInlineEditor}
			<button
				title="Expand"
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
			<button
				title="Conditions"
				class={classNames(
					'text-2xs py-0.5 font-bold w-fit border cursor-pointer rounded-sm',
					isConditionalWrapperManuallySelected
						? 'bg-red-100 text-red-600 border-red-500 hover:bg-red-200 hover:text-red-800'
						: 'bg-indigo-100 text-indigo-600 border-indigo-500 hover:bg-indigo-200 hover:text-indigo-800'
				)}
				on:click={() => dispatch('triggerInlineEditor')}
				on:pointerdown|stopPropagation
			>
				<ButtonDropdown hasPadding={false}>
					<svelte:fragment slot="items">
						{#each component.conditions ?? [] as { }, index}
							<MenuItem
								on:click={() => {
									$componentControl?.[component.id]?.setTab?.(index)
									isConditionalWrapperManuallySelected = true
								}}
							>
								<div
									class={classNames(
										'!text-gray-600 text-left px-4 py-2 gap-2 cursor-pointer hover:bg-gray-100 !text-xs font-semibold'
									)}
								>
									{#if index === component.conditions.length - 1}
										{`Debug default condition`}
									{:else}
										{`Debug condition ${index + 1}`}
									{/if}
								</div>
							</MenuItem>
						{/each}
						<MenuItem
							on:click={() => {
								$componentControl?.[component.id]?.setTab?.(-1)
								isConditionalWrapperManuallySelected = false
							}}
						>
							<div
								class={classNames(
									'!text-red-600 text-left px-4 py-2 gap-2 cursor-pointer hover:bg-gray-100 !text-xs font-semibold'
								)}
							>
								{`Reset debug mode`}
							</div>
						</MenuItem>
					</svelte:fragment>
				</ButtonDropdown>
			</button>
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

{#if error}
	{@const json = JSON.parse(JSON.stringify(error))}
	<span
		title="Error"
		class={classNames(
			'text-red-500 px-1 text-2xs py-0.5 font-bold w-fit absolute border border-red-500 -bottom-1  shadow left-1/2 transform -translate-x-1/2 z-50 cursor-pointer',
			'bg-red-100/80'
		)}
	>
		<Popover notClickable placement="bottom" popupClass="!bg-white border w-96">
			<Bug size={14} />
			<span slot="text">
				<div class="bg-white">
					<Alert type="error" title={`${json?.name}: ${json?.message}`}>
						<div class="flex flex-col gap-2">
							<div>
								<pre class=" whitespace-pre-wrap text-gray-900 bg-white border w-full p-4 text-xs">
									{json?.stack ?? ''}	
								</pre>
							</div>
							<Button color="red" variant="border" on:click={openDebugRuns}>Open Debug Runs</Button>
						</div>
					</Alert>
				</div>
			</span>
		</Popover>
	</span>
{/if}
