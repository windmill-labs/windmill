<script lang="ts">
	import Menu from '$lib/components/common/menu/Menu.svelte'
	import type { FlowCopilotContext } from '$lib/components/copilot/flow'
	import Popover from '$lib/components/Popover.svelte'
	import { getContext } from 'svelte'
	import { copilotInfo } from '$lib/stores'
	import { ExternalLink, Wand2 } from 'lucide-svelte'
	import { base } from '$lib/base'

	let openNoCopilot = false
	export let className = ''

	const { drawerStore: copilotDrawerStore } =
		getContext<FlowCopilotContext | undefined>('FlowCopilotContext') || {}
</script>

<div class={className}>
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
					class=" bg-surface text-violet-800 dark:text-violet-400 border mx-0.5 focus:outline-none hover:bg-surface-hover focus:ring-4 focus:ring-gray-200 font-medium rounded-full text-sm w-7 h-7 flex items-center justify-center"
				>
					<Wand2 size={12} />
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
