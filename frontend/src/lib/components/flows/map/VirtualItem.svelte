<script lang="ts">
	import { Badge } from '$lib/components/common'
	import type { FlowModule } from '$lib/gen'
	import { classNames } from '$lib/utils'
	import { ClipboardCopy, ExternalLink, Wand2, X } from 'lucide-svelte'
	import { createEventDispatcher, getContext } from 'svelte'
	import InsertModuleButton from './InsertModuleButton.svelte'
	import type { FlowCopilotContext } from '$lib/components/copilot/flow'
	import { copilotInfo } from '$lib/stores'
	import Menu from '$lib/components/common/menu/Menu.svelte'
	import InsertTriggerButton from './InsertTriggerButton.svelte'

	export let label: string
	export let modules: FlowModule[] | undefined
	export let index: number
	export let insertable: boolean
	export let bgColor: string = ''
	export let selected: boolean
	export let selectable: boolean
	export let deleteBranch: { module: FlowModule; index: number } | undefined = undefined
	export let id: string | undefined = undefined
	export let moving: string | undefined = undefined
	export let center = true
	export let disableAi = false

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
	let triggerOpenMenu = false
	let openNoCopilot = false

	const { drawerStore: copilotDrawerStore, currentStepStore: copilotCurrentStepStore } =
		getContext<FlowCopilotContext | undefined>('FlowCopilotContext') || {}
</script>

{#if insertable && deleteBranch}
	<div class="w-[27px] absolute -top-[40px] left-[50%] right-[50%] -translate-x-1/2">
		<button
			title="Delete branch"
			on:click|stopPropagation={() => {
				if (deleteBranch) {
					dispatch('deleteBranch', deleteBranch)
				}
			}}
			type="button"
			class="text-primary bg-surface border mx-[1px] border-gray-300 dark:border-gray-500 focus:outline-none hover:bg-surface-hover focus:ring-4 focus:ring-gray-200 font-medium rounded-full text-sm w-[25px] h-[25px] flex items-center justify-center"
		>
			<X class="m-[5px]" size={15} />
		</button>
	</div>
{/if}
<!-- svelte-ignore a11y-click-events-have-key-events -->
<!-- svelte-ignore a11y-no-static-element-interactions -->
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
	id={`flow-editor-virtual-${label}`}
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
		class="{openMenu
			? 'z-20'
			: ''} w-[27px] absolute top-[49px] left-[50%] right-[50%] -translate-x-1/2"
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
				class="text-primary bg-surface border mx-[1px] border-gray-300 dark:border-gray-500 focus:outline-none hover:bg-gray-100 focus:ring-4 focus:ring-gray-200 font-medium rounded-full text-sm w-[25px] h-[25px] flex items-center justify-center"
			>
				<ClipboardCopy size={12} />
			</button>
		{:else}
			<InsertModuleButton
				{disableAi}
				bind:open={openMenu}
				trigger={label == 'Input'}
				on:new={(e) => {
					if (modules) {
						dispatch('insert', {
							modules,
							index: index,
							detail: e.detail
						})
					}
				}}
				{index}
				modules={modules ?? []}
			/>
		{/if}
	</div>
{/if}

{#if insertable && modules && label == 'Input'}
	{#if !disableAi}
		<div
			class="{openNoCopilot
				? 'z-10'
				: ''} w-9 absolute -top-10 left-[50%] right-[50%] -translate-x-1/2"
		>
			<Menu pointerDown noMinW placement="bottom-center" let:close bind:show={openNoCopilot}>
				<button
					title="AI Flow Builder"
					on:pointerdown={$copilotInfo.exists_openai_resource_path
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
					<Wand2 size={16} />
				</button>
				{#if !$copilotInfo.exists_openai_resource_path}
					<div class="text-primary p-4">
						<p class="text-sm w-80">
							Enable Windmill AI in the
							<a
								href="/workspace_settings?tab=openai"
								target="_blank"
								class="inline-flex flex-row items-center gap-1"
								on:click={() => {
									close()
								}}
							>
								workspace settings
								<ExternalLink size={16} />
							</a>
						</p>
					</div>
				{/if}
			</Menu>
		</div>
	{/if}
	<div
		class="{triggerOpenMenu
			? 'z-10'
			: ''} w-[27px] absolute top-[50px] left-[65%] right-[35%] -translate-x-1/2"
	>
		<InsertTriggerButton
			{disableAi}
			bind:open={triggerOpenMenu}
			on:new={(e) => {
				if (modules) {
					dispatch('insert', {
						modules,
						index: 0,
						detail: e.detail
					})
				}
			}}
			index={0}
			modules={modules ?? []}
		/>
	</div>
{/if}
