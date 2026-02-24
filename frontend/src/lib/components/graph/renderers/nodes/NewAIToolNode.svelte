<script lang="ts">
	import { type NewAiToolN } from '../../graphBuilder.svelte'
	import InsertModuleInner from '$lib/components/flows/map/InsertModuleInner.svelte'
	import { Plus } from 'lucide-svelte'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import { Button } from '$lib/components/common'

	let funcDesc = $state('')
	interface Props {
		data: NewAiToolN['data']
	}
	let { data }: Props = $props()

	let open = $state(false)
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<Popover
	bind:isOpen={open}
	portal="#flow-editor"
	contentClasses="p-2 max-w-lg h-[400px] bg-surface"
	class="inline-block"
	usePointerDownOutside
	floatingConfig={{
		placement: 'bottom',
		strategy: 'absolute',
		gutter: 8,
		overflowPadding: 16,
		flip: true,
		fitViewport: true,
		overlap: false
	}}
>
	{#snippet trigger()}
		<Button
			size="xs3"
			variant="default"
			nonCaptureEvent
			selected={open}
			startIcon={{ icon: Plus }}
			wrapperClasses="{open
				? 'bg-surface-secondary'
				: 'bg-surface-tertiary'} transition-colors drop-shadow-base"
			btnClasses="gap-1 text-2xs px-1"
		>
			Tool
		</Button>
	{/snippet}
	{#snippet content({ close })}
		<InsertModuleInner
			bind:funcDesc
			toolMode
			on:close={() => {
				close()
			}}
			on:new={(e) => {
				data.eventHandlers.insert({
					index: -1, // ignored when agentId is set
					agentId: data.agentModuleId,
					...e.detail
				})
				close()
			}}
			on:insert={(e) => {
				data.eventHandlers.insert({
					index: -1, // ignored when agentId is set
					agentId: data.agentModuleId,
					...e.detail
				})
				close()
			}}
			on:pickScript={(e) => {
				data.eventHandlers.insert({
					index: -1, // ignored when agentId is set
					agentId: data.agentModuleId,
					kind: e.detail.kind,
					script: {
						...e.detail,
						summary: e.detail.summary
							? e.detail.summary.replace(/\s/, '_').replace(/[^a-zA-Z0-9_]/g, '')
							: e.detail.path.split('/').pop()
					}
				})
				close()
			}}
			on:pickMcpTool={(e) => {
				data.eventHandlers.insert({
					index: -1,
					agentId: data.agentModuleId,
					kind: 'mcpTool'
				})
				close()
			}}
			on:pickWebsearchTool={(e) => {
				data.eventHandlers.insert({
					index: -1,
					agentId: data.agentModuleId,
					kind: 'websearchTool'
				})
				close()
			}}
			on:pickAiAgentTool={(e) => {
				data.eventHandlers.insert({
					index: -1,
					agentId: data.agentModuleId,
					kind: 'aiAgentTool'
				})
				close()
			}}
		/>
	{/snippet}
</Popover>
