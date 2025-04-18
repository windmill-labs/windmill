<script lang="ts">
	import { CornerDownLeft } from 'lucide-svelte'
	import Button from './common/button/Button.svelte'
	import Editor from './Editor.svelte'
	import { runPreviewJobAndPollResult } from './jobs/utils'
	import { workspaceStore } from '$lib/stores'
	import type { ScriptLang } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'

	type Props = {
		resourceType: string
		resourcePath: string
	}
	let { resourcePath, resourceType }: Props = $props()

	let code = $state('SELECT * FROM users')
	let isRunning = $state(false)

	async function run() {
		if (isRunning || !$workspaceStore) return
		isRunning = true
		try {
			const result = await runPreviewJobAndPollResult({
				workspace: $workspaceStore,
				requestBody: {
					language: resourceType as ScriptLang,
					content: code,
					args: {
						database: '$res:' + resourcePath
					}
				}
			})
			sendUserToast('Query executed')
		} catch (e) {
			console.error(e)
			sendUserToast('Error running query: ' + (e.message ?? e.error.message), true)
		} finally {
			isRunning = false
		}
	}
</script>

<Editor bind:code lang="sql" scriptLang="mysql" class="w-full h-full" />
<Button
	wrapperClasses="absolute z-10 bottom-2 right-6"
	color="dark"
	shortCut={{ Icon: CornerDownLeft }}
	on:click={run}
>
	Run query
</Button>
