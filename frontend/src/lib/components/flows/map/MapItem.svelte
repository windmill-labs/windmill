<script lang="ts">
	import type { FlowEditorContext } from '../types'
	import { createEventDispatcher, getContext } from 'svelte'
	import FlowModuleSchemaItem from './FlowModuleSchemaItem.svelte'
	import type { FlowModule } from '$lib/gen'
	import InsertModuleButton from './InsertModuleButton.svelte'
	import { faBarsStaggered, faCodeBranch, faLongArrowDown } from '@fortawesome/free-solid-svg-icons'
	import Icon from 'svelte-awesome'
	import IconedResourceType from '$lib/components/IconedResourceType.svelte'
	import LanguageIcon from '$lib/components/common/languageIcons/LanguageIcon.svelte'
	import { Building, ClipboardCopy, GitBranchPlus, Move, Repeat, Square } from 'lucide-svelte'
	import type { Writable } from 'svelte/store'
	import { Button } from '$lib/components/common'

	export let mod: FlowModule
	export let trigger: boolean
	export let insertable: boolean
	export let insertableEnd = false
	export let annotation: string | undefined = undefined
	export let branchable: boolean = false
	export let bgColor: string = ''
	export let modules: FlowModule[]
	export let moving: string | undefined = undefined

	$: idx = modules.findIndex((m) => m.id === mod.id)

	const { selectedId } = getContext<{ selectedId: Writable<string> }>('FlowGraphContext')
	const dispatch = createEventDispatcher<{
		delete: CustomEvent<MouseEvent>
		insert: {
			modules: FlowModule[]
			index: number
			detail: 'script' | 'forloop' | 'branchone' | 'branchall' | 'move'
		}
		select: string
		newBranch: { module: FlowModule }
		move: { module: FlowModule }
	}>()

	$: itemProps = {
		selected: $selectedId === mod.id,
		retry: mod.retry?.constant != undefined || mod.retry?.exponential != undefined,
		earlyStop: mod.stop_after_if != undefined,
		suspend: Boolean(mod.suspend),
		sleep: Boolean(mod.sleep)
	}

	function onDelete(event: CustomEvent<MouseEvent>) {
		dispatch('delete', event)
	}
	let openMenu: boolean | undefined = undefined
	let openMenu2: boolean | undefined = undefined
</script>

{#if mod}
	{#if insertable}
		<div
			class="{openMenu ? 'z-10' : ''} w-7 absolute -top-9 left-[50%] right-[50%] -translate-x-1/2"
		>
			{#if moving}
				<button
					title="Add branch"
					on:click={() => {
						dispatch('insert', { modules, index: idx, detail: 'move' })
					}}
					type="button"
					class=" text-gray-900 bg-white border mx-0.5 border-gray-300 focus:outline-none hover:bg-gray-100 focus:ring-4 focus:ring-gray-200 font-medium rounded-full text-sm w-6 h-6 flex items-center justify-center"
				>
					<ClipboardCopy size={12} />
				</button>
			{:else}
				<InsertModuleButton
					bind:open={openMenu}
					{trigger}
					on:new={(e) => {
						dispatch('insert', { modules, index: idx, detail: e.detail })
					}}
				/>
			{/if}
		</div>
	{/if}
	<div class="relative">
		{#if moving == mod.id}
			<div class="absolute z-10 right-20 top-0.5 center-center">
				<Button color="dark" on:click={() => dispatch('move')} size="xs" variant="border">
					Cancel move
				</Button>
			</div>
		{/if}

		<div class={moving == mod.id ? 'opacity-50' : ''}>
			{#if mod.value.type === 'forloopflow'}
				<FlowModuleSchemaItem
					deletable={insertable}
					label={mod.summary || 'For loop'}
					id={mod.id}
					on:move={() => dispatch('move')}
					on:delete={onDelete}
					on:click={() => dispatch('select', mod.id)}
					{...itemProps}
					{bgColor}
				>
					<div slot="icon">
						<Repeat size={16} />
					</div>
				</FlowModuleSchemaItem>
			{:else if mod.value.type === 'branchone'}
				<FlowModuleSchemaItem
					deletable={insertable}
					on:delete={onDelete}
					on:move={() => dispatch('move')}
					on:click={() => dispatch('select', mod.id)}
					{...itemProps}
					id={mod.id}
					label={mod.summary || 'Run one branch'}
					{bgColor}
				>
					<div slot="icon">
						<Icon data={faCodeBranch} scale={1} />
					</div>
				</FlowModuleSchemaItem>
			{:else if mod.value.type === 'branchall'}
				<FlowModuleSchemaItem
					deletable={insertable}
					on:delete={onDelete}
					on:move={() => dispatch('move')}
					on:click={() => dispatch('select', mod.id)}
					id={mod.id}
					{...itemProps}
					label={mod.summary || 'Run all branches'}
					{bgColor}
				>
					<div slot="icon">
						<Icon data={faCodeBranch} scale={1} />
					</div>
				</FlowModuleSchemaItem>
			{:else}
				<FlowModuleSchemaItem
					on:click={() => dispatch('select', mod.id)}
					on:delete={onDelete}
					on:move={() => dispatch('move')}
					deletable={insertable}
					id={mod.id}
					{...itemProps}
					modType={mod.value.type}
					{bgColor}
					label={mod.summary ||
						(`path` in mod.value ? mod.value.path : undefined) ||
						(mod.value.type === 'rawscript' ? `Inline ${mod.value.language}` : 'To be defined')}
				>
					<div slot="icon">
						{#if mod.value.type === 'rawscript'}
							<LanguageIcon lang={mod.value.language} width={16} height={16} />
						{:else if mod.summary == 'Terminate flow'}
							<Square size={16} />
						{:else if mod.value.type === 'identity'}
							<Icon data={faLongArrowDown} scale={1.1} />
						{:else if mod.value.type === 'flow'}
							<Icon data={faBarsStaggered} scale={1.0} />
						{:else if mod.value.type === 'script'}
							{#if mod.value.path.startsWith('hub/')}
								<div>
									<IconedResourceType
										width="20px"
										height="20px"
										name={mod.value.path.split('/')[2]}
										silent={true}
									/>
								</div>
							{:else}
								<Building size={14} />
							{/if}
						{/if}
					</div>
				</FlowModuleSchemaItem>
			{/if}
		</div>
	</div>
	{#if insertable && insertableEnd}
		<div
			class="{openMenu2 ? 'z-10' : ''} w-7 absolute top-11 left-[50%] right-[50%] -translate-x-1/2"
		>
			{#if moving}
				<button
					title="Add branch"
					on:click={() => {
						dispatch('insert', { modules, index: idx + 1, detail: 'move' })
					}}
					type="button"
					class=" text-gray-900 bg-white border mx-0.5  border-gray-300 focus:outline-none hover:bg-gray-100 focus:ring-4 focus:ring-gray-200 font-medium rounded-full text-sm w-6 h-6 flex items-center justify-center"
				>
					<ClipboardCopy size={12} />
				</button>
			{:else}
				<InsertModuleButton
					bind:open={openMenu2}
					{trigger}
					on:new={(e) => {
						dispatch('insert', { modules, index: idx + 1, detail: e.detail })
					}}
				/>
			{/if}
		</div>
	{/if}

	{#if insertable && branchable}
		<div class="w-7 absolute top-11 left-[60%] right-[40%] -translate-x-1/2">
			<button
				title="Add branch"
				on:click={() => {
					dispatch('newBranch', { module: mod })
				}}
				type="button"
				class=" text-gray-900 bg-white border mx-0.5 rotate-180 border-gray-300 focus:outline-none hover:bg-gray-100 focus:ring-4 focus:ring-gray-200 font-medium rounded-full text-sm w-6 h-6 flex items-center justify-center"
			>
				<GitBranchPlus size={12} />
			</button>
		</div>
	{/if}
{/if}
