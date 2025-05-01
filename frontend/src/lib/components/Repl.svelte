<script lang="ts">
	import { CornerDownLeft } from 'lucide-svelte'
	import Button from './common/button/Button.svelte'
	import Editor from './Editor.svelte'
	import { runPreviewJobAndPollResult } from './jobs/utils'
	import { getLanguageByResourceType } from './apps/components/display/dbtable/utils'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import { _ } from 'ag-grid-community'
	import { sendUserToast } from '$lib/utils'
	import { workspaceStore } from '$lib/stores'
	import StepHistory, { type StepHistoryData } from './flows/propPicker/StepHistory.svelte'

	type Props = {
		resourceType: string
		resourcePath: string
		onData: (data: Record<string, any>[]) => void
		placeholderTableName?: string
	}
	let { resourcePath, resourceType, onData }: Props = $props()

	let code = $state('#Replace with your command')

	let isRunning = $state(false)

	let runHistory: (StepHistoryData & { code: string; result: Record<string, any>[] })[] = $state([])

	async function run({ doPostgresRowToJsonFix }: { doPostgresRowToJsonFix?: boolean } = {}) {
		if (isRunning || !$workspaceStore) return
		isRunning = true
		try {
			let { job, result } = (await runPreviewJobAndPollResult(
				{
					workspace: $workspaceStore,
					requestBody: {
						language: getLanguageByResourceType(resourceType),
						content: '',
						args: {
							database: '$res:' + resourcePath
						}
					}
				},
				{ withJobData: true }
			)) as any

			sendUserToast('Command executed')
		} catch (e) {
			sendUserToast('Error running query: ' + (e.message ?? e.error.message), true)
		} finally {
			isRunning = false
		}
	}
	let editor = $state<Editor | null>(null)
</script>

<Splitpanes>
	<Pane class="relative">
		<Editor
			bind:this={editor}
			bind:code
			lang="sql"
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
			}}
		/>
	</Pane>
</Splitpanes>
