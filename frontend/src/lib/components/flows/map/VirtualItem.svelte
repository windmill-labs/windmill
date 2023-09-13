<script lang="ts">
	import { Badge } from '$lib/components/common'
	import type { FlowModule } from '$lib/gen'
	import { classNames } from '$lib/utils'
	import { faBolt, faMagicWandSparkles } from '@fortawesome/free-solid-svg-icons'
	import { ClipboardCopy, ExternalLink, X } from 'lucide-svelte'
	import { createEventDispatcher, getContext } from 'svelte'
	import { Icon } from 'svelte-awesome'
	import InsertModuleButton from './InsertModuleButton.svelte'
	import type { FlowCopilotContext } from '$lib/components/copilot/flow'
	import { existsOpenaiResourcePath } from '$lib/stores'
	import Menu from '$lib/components/common/menu/Menu.svelte'

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
	export let moving: string | undefined = undefined
	export let center = true

	const dispatch = createEventDispatcher<{
		insert: {
			detail: 'script' | 'forloop' | 'branchone' | 'branchall' | 'trigger' | 'move'
			modules: FlowModule[]
			index: number
		}
		select: string
		deleteBranch: { module: FlowModule; index: number }
	}>()
	let openMenu = false
	let openNoCopilot = false

	const { drawerStore: copilotDrawerStore, currentStepStore: copilotCurrentStepStore } =
		getContext<FlowCopilotContext | undefined>('FlowCopilotContext') || {}
</script>

{#if insertable && deleteBranch}
	<div class="w-7 absolute -top-10 left-[50%] right-[50%] -translate-x-1/2">
		<button
			title="Delete branch"
			on:click|stopPropagation={() => {
				dispatch('deleteBranch', deleteBranch)
			}}
			type="button"
			class="text-primary bg-surface border mx-0.5 border-gray-300 focus:outline-none hover:bg-surface-hover focus:ring-4 focus:ring-gray-200 font-medium rounded-full text-sm w-6 h-6 flex items-center justify-center"
		>
			<X size={14} />
		</button>
	</div>
{/if}
<!-- svelte-ignore a11y-click-events-have-key-events -->
<div
	class={classNames(
		'w-full flex relative overflow-hidden rounded-sm',
		selectable ? 'cursor-pointer' : '',
		selected ? 'outline outline-offset-1 outline-2  outline-gray-600' : '',
		label === 'Input' && $copilotCurrentStepStore === 'Input' ? 'z-[901]' : ''
	)}
	style="min-width: 275px; max-height: 80px; background-color: {bgColor};"
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
		class="flex gap-1 justify-between {center
			? 'items-center'
			: 'items-baseline'} w-full overflow-hidden rounded-sm border p-2 text-2xs module text-primary border-gray-400"
	>
		{#if $$slots.icon}
			<slot name="icon" />
			<span class="mr-2" />
		{/if}
		<div />
		<div class="flex-1 truncate"><pre>{label}</pre></div>
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
		{#if moving}
			<button
				title="Add branch"
				on:click={() => {
					if (modules) {
						dispatch('insert', {
							modules,
							index,
							detail: 'move'
						})
					}
				}}
				type="button"
				class="text-primary bg-surface border mx-0.5 border-gray-300 focus:outline-none hover:bg-gray-100 focus:ring-4 focus:ring-gray-200 font-medium rounded-full text-sm w-6 h-6 flex items-center justify-center"
			>
				<ClipboardCopy size={12} />
			</button>
		{:else}
			<InsertModuleButton
				bind:open={openMenu}
				trigger={label == 'Input'}
				on:new={(e) => {
					if (modules) {
						dispatch('insert', {
							modules,
							index: whereInsert == 'after' ? index : index - 1,
							detail: e.detail
						})
					}
				}}
				index={whereInsert == 'after' ? index : index - 1}
				modules={modules ?? []}
			/>
		{/if}
	</div>
{/if}

{#if insertable && modules && label == 'Input'}
	<div
		class="{openNoCopilot
			? 'z-10'
			: ''} w-9 absolute -top-10 left-[50%] right-[50%] -translate-x-1/2"
	>
		<Menu pointerDown noMinW placement="bottom-center" let:close bind:show={openNoCopilot}>
			<button
				title="AI Flow Builder"
				on:pointerdown={$existsOpenaiResourcePath
					? (ev) => {
							ev.preventDefault()
							ev.stopPropagation()
							$copilotDrawerStore?.openDrawer()
					  }
					: undefined}
				slot="trigger"
				type="button"
				class="text-primary bg-surface border mx-0.5 focus:outline-none hover:bg-surface-hover focus:ring-4 focus:ring-gray-200 font-medium rounded-full text-sm w-8 h-8 flex items-center justify-center"
			>
				<Icon data={faMagicWandSparkles} scale={1} />
			</button>
			{#if !$existsOpenaiResourcePath}
				<div class="text-primary p-4">
					<p class="text-sm w-80"
						>Enable Windmill AI in the <a
							href="/workspace_settings?tab=openai"
							target="_blank"
							class="inline-flex flex-row items-center gap-1"
							on:click={() => {
								close()
							}}
							>workspace settings
							<ExternalLink size={16} /></a
						></p
					>
				</div>
			{/if}
		</Menu>
	</div>
	<div class="w-7 absolute top-12 left-[65%] right-[35%] -translate-x-1/2">
		<button
			title="Add a Trigger"
			on:click={() => {
				if (modules) {
					dispatch('insert', { modules, index: 0, detail: 'trigger' })
				}
			}}
			type="button"
			class="text-primary bg-surface border mx-0.5 rotate-180 focus:outline-none hover:bg-surface-hover focus:ring-4 focus:ring-gray-200 font-medium rounded-full text-sm w-6 h-6 flex items-center justify-center"
		>
			<Icon data={faBolt} scale={0.8} />
		</button>
	</div>
{/if}
