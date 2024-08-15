<script lang="ts">
	import VirtualItem from '$lib/components/flows/map/VirtualItem.svelte'
	import { ExternalLink, Wand2 } from 'lucide-svelte'
	import NodeWrapper from './NodeWrapper.svelte'
	import Popover from '$lib/components/Popover.svelte'
	import type { GraphEventHandlers } from '../../graphBuilder'
	import type { FlowModule } from '$lib/gen'
	import { getStateColor } from '../../util'
	import Menu from '$lib/components/common/menu/Menu.svelte'
	import type { FlowCopilotContext } from '$lib/components/copilot/flow'
	import { getContext } from 'svelte'
	import { copilotInfo } from '$lib/stores'
	import { base } from '$lib/base'
	import type { Writable } from 'svelte/store'

	export let data: {
		insertable: boolean
		eventHandlers: GraphEventHandlers
		modules: FlowModule[]
		disableAi: boolean
	}
	let openNoCopilot = false

	const { drawerStore: copilotDrawerStore } =
		getContext<FlowCopilotContext | undefined>('FlowCopilotContext') || {}

	const { selectedId } = getContext<{
		selectedId: Writable<string | undefined>
	}>('FlowGraphContext')
</script>

<NodeWrapper enableTargetHandle={false} let:darkMode>
	{#if data.insertable && !data.disableAi}
		<div class="absolute -top-10 left-1/2 transform -translate-x-1/2 z-10">
			<Popover>
				<div class={openNoCopilot ? 'z-10' : ''}>
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
							class=" bg-surface text-violet-800 dark:text-violet-400 border mx-0.5 focus:outline-none hover:bg-surface-hover focus:ring-4 focus:ring-gray-200 font-medium rounded-full text-sm w-8 h-8 flex items-center justify-center"
						>
							<Wand2 size={16} />
						</button>
						{#if !$copilotInfo.exists_openai_resource_path}
							<div class="text-primary p-4">
								<p class="text-sm w-80">
									Enable Windmill AI in the
									<a
										href="{base}/workspace_settings?tab=openai"
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

				<svelte:fragment slot="text">AI Flow builder</svelte:fragment>
			</Popover>
		</div>
	{/if}
	<VirtualItem
		label="Input"
		modules={data.modules}
		selectable
		selected={$selectedId === 'Input'}
		insertable={false}
		bgColor={getStateColor(undefined, darkMode)}
		on:insert={(e) => {
			data.eventHandlers?.insert(e.detail)
		}}
		on:select={(e) => {
			data.eventHandlers?.select(e.detail)
		}}
	/>
</NodeWrapper>
