<script lang="ts">
	import { twMerge } from 'tailwind-merge'
	import { type NewAiToolN } from '../../graphBuilder.svelte'
	import InsertModuleInner from '$lib/components/flows/map/InsertModuleInner.svelte'
	import { Cross } from 'lucide-svelte'
	import PopupV2 from '$lib/components/common/popup/PopupV2.svelte'
	import { flip, offset } from 'svelte-floating-ui/dom'
	import type { ComputeConfig } from 'svelte-floating-ui'
	import Button from '$lib/components/common/button/Button.svelte'
	import Label from '$lib/components/Label.svelte'

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
	let showMcpInput = $state(false)
	let mcpResourcePath = $state('')

	$inspect(open, showMcpInput, mcpResourcePath)

	function addMcpTool() {
		if (mcpResourcePath.trim()) {
			data.eventHandlers.insert({
				index: -1,
				agentId: data.agentModuleId,
				kind: 'mcp_tool',
				mcpResource: mcpResourcePath.trim()
			})
			mcpResourcePath = ''
			showMcpInput = false
			open = false
		}
	}
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
		{#if showMcpInput}
			<div class="p-4 flex flex-col gap-2 w-[400px]">
				<Label label="MCP Resource Path">
					<input
						type="text"
						bind:value={mcpResourcePath}
						placeholder="u/admin/my_mcp_server"
						class="windmillapp-input"
						onkeydown={(e) => {
							if (e.key === 'Enter') {
								addMcpTool()
							} else if (e.key === 'Escape') {
								showMcpInput = false
								mcpResourcePath = ''
							}
						}}
					/>
				</Label>
				<div class="flex gap-2 justify-end">
					<Button
						size="xs"
						color="light"
						on:click={() => {
							showMcpInput = false
							mcpResourcePath = ''
						}}
					>
						Cancel
					</Button>
					<Button size="xs" on:click={addMcpTool} disabled={!mcpResourcePath.trim()}>
						Add MCP Tool
					</Button>
				</div>
			</div>
		{:else}
			<div class="flex flex-col">
				<InsertModuleInner
					bind:funcDesc
					scriptOnly
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
				/>
				<div class="border-t border-gray-200 dark:border-gray-700 mt-2 pt-2 px-2">
					<Button
						size="xs"
						color="light"
						btnClasses="w-full"
						on:click={() => {
							showMcpInput = true
						}}
					>
						+ MCP Server
					</Button>
				</div>
			</div>
		{/if}
	{/snippet}
</PopupV2>
