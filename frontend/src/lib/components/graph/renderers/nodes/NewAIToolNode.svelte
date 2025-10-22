<script lang="ts">
	import { type NewAiToolN } from '../../graphBuilder.svelte'
	import InsertModuleInner from '$lib/components/flows/map/InsertModuleInner.svelte'
	import { Plus } from 'lucide-svelte'
	import PopupV2 from '$lib/components/common/popup/PopupV2.svelte'
	import { flip, offset } from 'svelte-floating-ui/dom'
	import type { ComputeConfig } from 'svelte-floating-ui'
	import { Button } from '$lib/components/common'

	let funcDesc = $state('')
	interface Props {
		data: NewAiToolN['data']
	}
	let { data }: Props = $props()

	let floatingConfig: ComputeConfig = {
		strategy: 'fixed',
		// @ts-ignore
		placement: 'bottom-center',
		middleware: [offset(8), flip()],
		autoUpdate: true
	}

	let open = $state(false)
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<PopupV2 bind:open {floatingConfig} target="#flow-editor">
	{#snippet button()}
		<Button
			size="xs3"
			variant="default"
			onpointerdown={() => (open = !open)}
			selected={open}
			startIcon={{ icon: Plus }}
			wrapperClasses="{open
				? 'bg-surface-secondary'
				: 'bg-surface-tertiary'} transition-colors drop-shadow-base"
			btnClasses="gap-1 "
		>
			Tool
		</Button>
	{/snippet}
	{#snippet children({ close })}
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
		/>
	{/snippet}
</PopupV2>
