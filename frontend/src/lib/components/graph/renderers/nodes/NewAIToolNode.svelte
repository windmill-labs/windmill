<script lang="ts">
	import { twMerge } from 'tailwind-merge'
	import { type NewAiToolN } from '../../graphBuilder.svelte'
	import InsertModuleInner from '$lib/components/flows/map/InsertModuleInner.svelte'
	import { Cross } from 'lucide-svelte'
	import PopupV2 from '$lib/components/common/popup/PopupV2.svelte'
	import { flip, offset } from 'svelte-floating-ui/dom'
	import type { ComputeConfig } from 'svelte-floating-ui'

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
		<button
			title={`Add 'tool'
			}`}
			type="button"
			class={twMerge(
				'!w-full h-6 flex items-center justify-center !outline-[1px] outline dark:outline-gray-500 outline-gray-300 text-secondary bg-surface focus:outline-none hover:bg-surface-hover rounded'
			)}
			onpointerdown={() => (open = !open)}
		>
			<div class="flex flex-row items-center gap-1 font-medium text-2xs">
				<Cross size={12} />
				tool
			</div>
		</button>
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
