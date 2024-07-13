<script lang="ts">
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import type { FlowCopilotContext } from '$lib/components/copilot/flow'
	import Popover from '$lib/components/Popover.svelte'
	import { classNames } from '$lib/utils'
	import {
		AlertTriangle,
		Bed,
		Database,
		Gauge,
		Move,
		PhoneIncoming,
		Repeat,
		Square,
		Voicemail,
		X
	} from 'lucide-svelte'
	import { createEventDispatcher, getContext } from 'svelte'
	import { fade } from 'svelte/transition'
	import type { FlowInput } from '../types'
	import type { Writable } from 'svelte/store'

	export let selected: boolean = false
	export let deletable: boolean = false
	export let retry: boolean = false
	export let cache: boolean = false
	export let earlyStop: boolean = false
	export let suspend: boolean = false
	export let sleep: boolean = false
	export let mock: boolean = false
	export let bold: boolean = false
	export let id: string | undefined = undefined
	export let label: string
	export let modType: string | undefined = undefined
	export let bgColor: string = ''
	export let concurrency: boolean = false
	export let retries: number | undefined = undefined
	export let warningMessage: string | undefined = undefined

	const { flowInputsStore } = getContext<{ flowInputsStore: Writable<FlowInput | undefined> }>(
		'FlowGraphContext'
	)
	const dispatch = createEventDispatcher()

	const { currentStepStore: copilotCurrentStepStore } =
		getContext<FlowCopilotContext | undefined>('FlowCopilotContext') || {}
</script>

<!-- svelte-ignore a11y-click-events-have-key-events -->
<!-- svelte-ignore a11y-no-static-element-interactions -->
<div
	class={classNames(
		'w-full module flex rounded-sm cursor-pointer',
		selected ? 'outline outline-offset-1 outline-2  outline-gray-600 dark:outline-gray-400' : '',
		'flex relative',
		$copilotCurrentStepStore === id ? 'z-[901]' : ''
	)}
	style="width: 275px; height: 34px; background-color: {bgColor};"
	on:click
>
	<div class="absolute text-sm right-12 -bottom-3 flex flex-row gap-1 z-10">
		{#if retry}
			<Popover notClickable>
				<div
					transition:fade|local={{ duration: 200 }}
					class="center-center rounded border bg-surface border-gray-400 text-secondary px-1 py-0.5"
				>
					{#if retries}<span class="text-red-400 mr-2">{retries}</span>{/if}
					<Repeat size={14} />
				</div>
				<svelte:fragment slot="text">Retries</svelte:fragment>
			</Popover>
		{/if}

		{#if concurrency}
			<Popover notClickable>
				<div
					transition:fade|local={{ duration: 200 }}
					class="center-center rounded border bg-surface border-gray-400 text-secondary px-1 py-0.5"
				>
					<Gauge size={14} />
				</div>
				<svelte:fragment slot="text">Concurrency Limits</svelte:fragment>
			</Popover>
		{/if}
		{#if cache}
			<Popover notClickable>
				<div
					transition:fade|local={{ duration: 200 }}
					class="center-center rounded border bg-surface border-gray-400 text-secondary px-1 py-0.5"
				>
					<Database size={14} />
				</div>
				<svelte:fragment slot="text">Cached</svelte:fragment>
			</Popover>
		{/if}
		{#if earlyStop}
			<Popover notClickable>
				<div
					transition:fade|local={{ duration: 200 }}
					class="center-center bg-surface rounded border border-gray-400 text-secondary px-1 py-0.5"
				>
					<Square size={14} />
				</div>
				<svelte:fragment slot="text">Early stop/break</svelte:fragment>
			</Popover>
		{/if}
		{#if suspend}
			<Popover notClickable>
				<div
					transition:fade|local={{ duration: 200 }}
					class="center-center bg-surface rounded border border-gray-400 text-secondary px-1 py-0.5"
				>
					<PhoneIncoming size={14} />
				</div>
				<svelte:fragment slot="text">Suspend</svelte:fragment>
			</Popover>
		{/if}
		{#if sleep}
			<Popover notClickable>
				<div
					transition:fade|local={{ duration: 200 }}
					class="center-center bg-surface rounded border border-gray-400 text-secondary px-1 py-0.5"
				>
					<Bed size={14} />
				</div>
				<svelte:fragment slot="text">Sleep</svelte:fragment>
			</Popover>
		{/if}
		{#if mock}
			<Popover notClickable>
				<div
					transition:fade|local={{ duration: 200 }}
					class="center-center bg-surface rounded border border-gray-400 text-secondary px-1 py-0.5"
				>
					<Voicemail size={14} />
				</div>
				<svelte:fragment slot="text">Mocked</svelte:fragment>
			</Popover>
		{/if}
	</div>

	<div
		class="flex gap-1 justify-between items-center w-full overflow-hidden rounded-sm
			border border-gray-400 dark:border-gray-600 p-2 text-2xs module text-primary"
	>
		{#if $$slots.icon}
			<slot name="icon" />
			<span class="mr-2" />
		{/if}
		<div class="flex-1 truncate" class:font-bold={bold}>{label}</div>
		<div class="flex items-center space-x-2">
			{#if id}
				<Badge color="indigo">{id}</Badge>
			{/if}
		</div>
	</div>
	{#if deletable}
		<button
			class="absolute -top-[10px] -right-[10px] rounded-full h-[20px] w-[20px] trash center-center text-primary
	border-[1.5px] border-gray-700 bg-surface duration-150 hover:bg-red-400 hover:text-white
	hover:border-red-700 {selected ? '' : '!hidden'}"
			on:click|preventDefault|stopPropagation={(event) =>
				dispatch('delete', { event, id, type: modType })}
		>
			<X class="mx-[3px]" size={14} strokeWidth={2} />
		</button>

		<button
			class="absolute -top-[10px] right-[35px] rounded-full h-[20px] w-[20px] trash center-center text-primary
border-[1.5px] border-gray-700 bg-surface duration-150 hover:bg-blue-400 hover:text-white
hover:border-blue-700 {selected ? '' : '!hidden'}"
			on:click|preventDefault|stopPropagation={(event) => dispatch('move')}
		>
			<Move class="mx-[3px]" size={14} strokeWidth={2} />
		</button>

		{#if (id && !Object.values($flowInputsStore?.[id]?.requiredInputsFilled || {}).every(Boolean)) || Boolean(warningMessage)}
			<div class="absolute -top-[10px] -left-[10px]">
				<Popover>
					<svelte:fragment slot="text"
						>{warningMessage ?? 'At least one required input is not set.'}
					</svelte:fragment>
					<div
						class="flex items-center justify-center h-full w-full rounded-md p-0.5 border border-yellow-600 text-yellow-600 bg-yellow-100 duration-150 hover:bg-yellow-300"
					>
						<AlertTriangle size={14} strokeWidth={2} />
					</div>
				</Popover>
			</div>
		{/if}
	{/if}
</div>

<style>
	.module:hover .trash {
		display: flex !important;
	}
</style>
