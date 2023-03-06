<script lang="ts">
	import { Badge } from '$lib/components/common'
	import type { FlowModule } from '$lib/gen'
	import { classNames } from '$lib/utils'
	import { faBolt } from '@fortawesome/free-solid-svg-icons'
	import { X } from 'lucide-svelte'
	import { createEventDispatcher } from 'svelte'
	import { Icon } from 'svelte-awesome'
	import InsertModuleButton from './InsertModuleButton.svelte'

	export let label: string
	export let modules: FlowModule[] | undefined
	export let index: number
	export let insertable: boolean
	export let bgColor: string = ''
	export let selected: boolean
	export let selectable: boolean
	export let whereInsert: 'before' | 'after' = 'after'
	export let deleteBranch: { module: FlowModule; index: number } | undefined = undefined
	export let id: string | undefined = undefined

	const dispatch = createEventDispatcher<{
		insert: {
			detail: 'script' | 'forloop' | 'branchone' | 'branchall' | 'trigger'
			modules: FlowModule[]
			index: number
		}
		select: string
		deleteBranch: { module: FlowModule; index: number }
	}>()
	let openMenu = false
</script>

{#if insertable && deleteBranch}
	<div class="w-7 absolute -top-10 left-[50%] right-[50%] -translate-x-1/2">
		<button
			title="Delete branch"
			on:click|stopPropagation={() => {
				dispatch('deleteBranch', deleteBranch)
			}}
			type="button"
			class=" text-gray-900 bg-white border mx-0.5 border-gray-300 focus:outline-none hover:bg-gray-100 focus:ring-4 focus:ring-gray-200 font-medium rounded-full text-sm w-6 h-6 flex items-center justify-center"
		>
			<X size={14} />
		</button>
	</div>
{/if}
<!-- svelte-ignore a11y-click-events-have-key-events -->
<div
	class={classNames(
		'w-full flex relative overflow-hidden rounded-sm ',
		selectable ? 'cursor-pointer' : '',
		selected ? 'outline outline-offset-1 outline-2  outline-gray-600' : ''
	)}
	style="min-width: 275px; height: 34px; background-color: {bgColor};"
	on:click={() => {
		if (selectable) {
			if (id) {
				dispatch('select', id)
			} else {
				dispatch('select', label)
			}
		}
	}}
>
	<div
		class="flex gap-1 justify-between items-center w-full overflow-hidden rounded-sm 
			border border-gray-400 p-2 text-2xs module"
	>
		{#if $$slots.icon}
			<slot name="icon" />
			<span class="mr-2" />
		{/if}
		<div />
		<div class="flex-1 truncate">{label}</div>
		<div class="flex items-center space-x-2">
			{#if id}
				<Badge color="indigo">{id}</Badge>
			{/if}
		</div>
	</div>
</div>

{#if insertable && modules && (label != 'Input' || modules.length == 0)}
	<div
		class="{openMenu ? 'z-10' : ''} w-7 absolute {whereInsert == 'after'
			? 'top-12'
			: '-top-10'} left-[50%] right-[50%] -translate-x-1/2"
	>
		<InsertModuleButton
			bind:open={openMenu}
			trigger={label == 'Input'}
			on:new={(e) => {
				if (modules) {
					dispatch('insert', { modules, index, detail: e.detail })
				}
			}}
		/>
	</div>
{/if}

{#if insertable && modules && label == 'Input'}
	<div class="w-7 absolute top-12 left-[65%] right-[35%] -translate-x-1/2">
		<button
			title="Add a Trigger"
			on:click={() => {
				if (modules) {
					dispatch('insert', { modules, index: 0, detail: 'trigger' })
				}
			}}
			type="button"
			class=" text-gray-900 bg-white border mx-0.5 rotate-180 border-gray-300 focus:outline-none hover:bg-gray-100 focus:ring-4 focus:ring-gray-200 font-medium rounded-full text-sm w-6 h-6 flex items-center justify-center"
		>
			<Icon data={faBolt} scale={0.8} />
		</button>
	</div>
{/if}
