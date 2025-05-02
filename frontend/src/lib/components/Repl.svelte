<script lang="ts">
	import { CornerDownLeft } from 'lucide-svelte'
	import Button from './common/button/Button.svelte'
	import Editor from './Editor.svelte'
	import { runPreviewJobAndPollResult } from './jobs/utils'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import { sendUserToast } from '$lib/utils'
	import { workspaceStore } from '$lib/stores'
	import StepHistory, { type StepHistoryData } from './flows/propPicker/StepHistory.svelte'

	type Props = {
		resourceType: string
		resourcePath: string
		onData: (data: Record<string, any>[]) => void
		placeholderTableName?: string
	}
	let output = $state<string>('') // New: capture output
	let { resourcePath, onData }: Props = $props()

	let code = $state('# Replace with your Bash command')
	let isRunning = $state(false)

	let runHistory: (StepHistoryData & { code: string; result: Record<string, any>[] })[] = $state([])

	let editor = $state<Editor | null>(null)

	async function run({ doPostgresRowToJsonFix }: { doPostgresRowToJsonFix?: boolean } = {}) {
		if (isRunning || !$workspaceStore) return
		isRunning = true
		output = ''
		try {
			const { job, result } = (await runPreviewJobAndPollResult(
				{
					workspace: $workspaceStore,
					requestBody: {
						language: 'bash',
						content: code,
						args: {
							database: '$res:' + resourcePath
						}
					}
				},
				{ withJobData: true }
			)) as any

			console.log({ result, job })
			output = result
				.map((line: any) => (typeof line === 'string' ? line : JSON.stringify(line, null, 2)))
				.join('\n')
			sendUserToast('Command executed')
		} catch (e) {
			const message = 'Error running command: ' + (e.message ?? e.error?.message)
			output = message
			sendUserToast(message, true)
		} finally {
			isRunning = false
		}
	}
</script>

<Splitpanes class="h-full">
	<Pane class="relative">
		<Editor
			bind:this={editor}
			bind:code
			scriptLang="bash"
			class="w-full h-full"
			cmdEnterAction={run}
		/>
		<Button
			wrapperClasses="absolute z-10 bottom-2 right-6"
			color={isRunning ? 'red' : undefined}
			variant="border"
			shortCut={{ Icon: CornerDownLeft }}
			on:click={() => run()}
		>
			{isRunning ? 'Running...' : 'Run'}
		</Button>
	</Pane>

	<Pane size={24} minSize={16}>
		<StepHistory
			staticInputs={runHistory}
			on:select={(e) => {
				const data = e.detail
				editor?.setCode(data.code)
				onData(data.result)
				output = data.result
					.map((r: any) => (typeof r === 'string' ? r : JSON.stringify(r, null, 2)))
					.join('\n')
			}}
		/>
	</Pane>

	<Pane size={35} minSize={20}>
		<div
			class="p-4 font-mono text-sm whitespace-pre-wrap bg-black text-green-400 h-full overflow-auto"
		>
			{#if isRunning}
				<span class="text-yellow-400">Running...</span>
			{:else if output}
				<hr class="my-2 border-gray-600" />
				<pre>{output}</pre>
			{:else}
				<span class="text-gray-500">No output yet</span>
			{/if}
		</div>
	</Pane>
</Splitpanes>
