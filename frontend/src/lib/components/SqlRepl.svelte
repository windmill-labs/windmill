<script module lang="ts">
	// code may be composed of many sql statements separated by ';'
	// this function splits them while taking into account that ';' may not
	// be the end of a statement (string or escaped)
	function splitSqlLastStatement(code: string) {
		const statements: string[] = []
		let currentStatement = ''
		let inSingleQuote = false
		let inDoubleQuote = false
		let inBacktick = false

		for (let i = 0; i < code.length; i++) {
			const char = code[i]
			const prevChar = i > 0 ? code[i - 1] : null

			if (char === "'" && !inDoubleQuote && !inBacktick && prevChar !== '\\') {
				inSingleQuote = !inSingleQuote
			} else if (char === '"' && !inSingleQuote && !inBacktick && prevChar !== '\\') {
				inDoubleQuote = !inDoubleQuote
			} else if (char === '`' && !inSingleQuote && !inDoubleQuote && prevChar !== '\\') {
				inBacktick = !inBacktick
			}

			if (char === ';' && !inSingleQuote && !inDoubleQuote && !inBacktick) {
				statements.push(currentStatement.trim())
				currentStatement = ''
			} else {
				currentStatement += char
			}
		}

		if (currentStatement.trim()) {
			statements.push(currentStatement.trim())
		}

		return statements
	}
</script>

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
		onData: (data: Record<string, any>[]) => void
	}
	let { resourcePath, resourceType, onData }: Props = $props()

	let code = $state('SELECT * FROM users')
	let isRunning = $state(false)

	async function run() {
		if (isRunning || !$workspaceStore) return
		isRunning = true
		try {
			const statements = splitSqlLastStatement(code)
			let result = (await runPreviewJobAndPollResult({
				workspace: $workspaceStore,
				requestBody: {
					language: resourceType as ScriptLang,
					content: code,
					args: {
						database: '$res:' + resourcePath
					}
				}
			})) as any
			if (statements.length > 1) {
				result = result[result.length - 1]
			}
			if (!Array.isArray(result)) {
				sendUserToast('Query result is not an array', true)
				return
			}
			if (statements[statements.length - 1].toUpperCase().trim().startsWith('SELECT')) {
				onData(result)
			}
			sendUserToast('Query executed')
		} catch (e) {
			console.error(e)
			sendUserToast('Error running query: ' + (e.message ?? e.error.message), true)
		} finally {
			isRunning = false
		}
	}
</script>

<Editor bind:code lang="sql" scriptLang="mysql" class="w-full h-full" cmdEnterAction={run} />
<Button
	wrapperClasses="absolute z-10 bottom-2 right-6"
	color={isRunning ? 'red' : 'dark'}
	shortCut={{ Icon: CornerDownLeft }}
	on:click={run}
>
	{isRunning ? 'Running...' : 'Run'}
</Button>
