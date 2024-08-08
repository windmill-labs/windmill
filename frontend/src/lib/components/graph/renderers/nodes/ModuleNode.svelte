<script lang="ts">
	// @ts-ignore
	import { NodeToolbar, Position } from '@xyflow/svelte'
	import MapItem from '$lib/components/flows/map/MapItem.svelte'
	import type { FlowModule, FlowModuleValue } from '$lib/gen/types.gen'
	import { ClipboardCopy, GitBranchPlus } from 'lucide-svelte'
	import { createEventDispatcher } from 'svelte'
	import NodeWrapper from './NodeWrapper.svelte'
	import InsertModuleButton from '$lib/components/flows/map/InsertModuleButton.svelte'

	export let data: {
		offset: number
		value: FlowModuleValue
		module: FlowModule
		trigger: boolean
		insertable: boolean
		insertableEnd: boolean
		annotation: string | undefined
		branchable: boolean
		bgColor: string
		modules: FlowModule[]
		moving: string | undefined
		duration_ms: number | undefined
		disableAi: boolean
		wrapperId: string | undefined
		retries: number | undefined
		flowJobs:
			| { flowJobs: string[]; selected: number; flowJobsSuccess: (boolean | undefined)[] }
			| undefined
	}

	$: idx = data.modules?.findIndex((m) => m.id === data.module.id)

	const dispatch = createEventDispatcher()
	let openMenu: boolean | undefined = undefined
</script>

<NodeWrapper offset={data.offset} let:darkMode>
	<MapItem
		mod={data.module}
		trigger={data.trigger}
		insertable={data.insertable}
		insertableEnd={data.insertableEnd}
		annotation={data.annotation}
		branchable={data.branchable}
		bgColor={darkMode ? '#2e3440' : '#dfe6ee'}
		modules={data.modules ?? []}
		moving={data.moving}
		duration_ms={data.duration_ms}
		disableAi={data.disableAi}
		wrapperId={data.wrapperId}
		retries={data.retries}
		flowJobs={data.flowJobs}
		on:delete
		on:insert
		on:move
		on:newBranch
		on:select
		on:selectedIteration
	/>
</NodeWrapper>

<NodeToolbar isVisible position={Position.Top} align="end">
	{#if data.insertable}
		<div
			class="{openMenu
				? 'z-20'
				: ''} w-[27px] absolute -top-[35px] left-[50%] right-[50%] -translate-x-1/2"
		>
			{#if data.moving}
				<button
					title="Add branch"
					on:click={() => {
						dispatch('insert', { modules: data.modules, index: idx, detail: 'move' })
					}}
					type="button"
					disabled={data.wrapperId === data.moving}
					class=" text-primary bg-surface border mx-[1px] border-gray-300 dark:border-gray-500 focus:outline-none hover:bg-gray-100 focus:ring-4 focus:ring-gray-200 font-medium rounded-full text-sm w-[25px] h-[25px] flex items-center justify-center"
				>
					<ClipboardCopy class="m-[5px]" size={15} />
				</button>
			{:else}
				<InsertModuleButton
					bind:disableAi={data.disableAi}
					bind:open={openMenu}
					bind:trigger={data.trigger}
					on:insert={(e) => {
						dispatch('insert', {
							modules: data.modules,
							index: idx + 1,
							detail: 'script',
							script: e.detail
						})
					}}
					on:new={(e) => {
						dispatch('insert', { modules: data.modules, index: idx, detail: e.detail })
					}}
					index={idx}
					modules={data.modules}
				/>
			{/if}
		</div>
	{/if}
</NodeToolbar>

<NodeToolbar isVisible position={Position.Bottom} align="center">
	{#if data.value.type === 'branchall' && data.insertable}
		<button
			class="rounded-full border hover:bg-surface-hover bg-surface p-1"
			on:click={() => {
				dispatch('addBranch')
			}}
		>
			<GitBranchPlus size={16} />
		</button>
	{/if}
</NodeToolbar>
