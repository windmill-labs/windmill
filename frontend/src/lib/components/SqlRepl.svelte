<script module lang="ts">
	// code may be composed of many sql statements separated by ';'
	// this function splits them while taking into account that ';' may not
	// be the end of a statement (string or escaped)
	function splitSqlStatements(code: string) {
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

	function pruneComments(code: string) {
		return code
			.replace(/--.*?(\r?\n|$)/g, '')
			.replace(/\/\*[\s\S]*?\*\//g, '')
			.trim()
	}
</script>

<script lang="ts">
	import { CornerDownLeft, Loader2 } from 'lucide-svelte'
	import Button from './common/button/Button.svelte'
	import { runScriptAndPollResult } from './jobs/utils'
	import { workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { untrack } from 'svelte'
	import { getLanguageByResourceType } from './apps/components/display/dbtable/utils'
	import StepHistory, { type StepHistoryData } from './flows/propPicker/StepHistory.svelte'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import { getDatabaseArg, getDbType } from './dbOps'
	import type { DbInput } from './dbTypes'
	import { wrapDucklakeQuery } from './ducklake'
	import ConfirmationModal from './common/confirmationModal/ConfirmationModal.svelte'
	import { containsDdlStatement } from './sqlDdl'

	type Props = {
		input: DbInput
		onData: (data: Record<string, any>[]) => void
		placeholderTableName?: string
	}
	let { input, onData, placeholderTableName }: Props = $props()
	let dbType = $derived(getDbType(input))
	let isDatatable = $derived(
		input?.type === 'database' && input.resourcePath.startsWith('datatable://')
	)

	const DEFAULT_SQL = 'SELECT * FROM _'
	let code = $state(DEFAULT_SQL)
	$effect(() => {
		const _code = untrack(() => code)
		if (placeholderTableName && _code === DEFAULT_SQL) {
			code = _code.replace('_', placeholderTableName ?? 'table')
		}
	})
	let isRunning = $state(false)
	let ddlWarningOpen = $state(false)

	let runHistory: (StepHistoryData & { code: string; result: Record<string, any>[] })[] = $state([])

	async function run({
		doPostgresRowToJsonFix,
		bypassDdlCheck
	}: { doPostgresRowToJsonFix?: boolean; bypassDdlCheck?: boolean } = {}) {
		if (isRunning || !$workspaceStore) return
		const READ_OPS = ['SELECT', 'WITH', 'SHOW', 'EXPLAIN', 'DESCRIBE']

		const statements = splitSqlStatements(pruneComments(code))
		if (statements.length === 0) {
			sendUserToast('Nothing to run', true)
			return
		}

		// On a datatable, schema-changing (DDL) statements should be recorded as
		// migrations rather than run ad-hoc. Warn before running them directly.
		if (isDatatable && !bypassDdlCheck && containsDdlStatement(statements)) {
			ddlWarningOpen = true
			return
		}

		isRunning = true
		try {
			// Transform all to JSON in case of select. This fixes the issue of
			// custom postgres enum type failing to convert to a rust type in the backend.
			// We don't always put the fix by default for row ordering concerns
			let transformedCode = code
			if (doPostgresRowToJsonFix) {
				transformedCode = statements
					.map((statement) => {
						if (READ_OPS.some((op) => statement.trim().toUpperCase().startsWith(op))) {
							return `SELECT row_to_json(__t__) FROM (${statement}) __t__`
						}
						return statement
					})
					.join(';')
			}
			const dbArg = getDatabaseArg(input)

			if (input?.type === 'ducklake') {
				transformedCode = wrapDucklakeQuery(transformedCode, input.ducklake)
			}

			let { job, result } = (await runScriptAndPollResult(
				{
					workspace: $workspaceStore,
					requestBody: {
						language: getLanguageByResourceType(dbType),
						content: transformedCode,
						args: dbArg
					}
				},
				{ withJobData: true }
			)) as any
			if (statements.length > 1) {
				result = result[result.length - 1]
			}
			if (!Array.isArray(result)) {
				sendUserToast('Query result is not an array', true)
				return
			}

			if (doPostgresRowToJsonFix) {
				result = result.map((row: any) => row['row_to_json'])
			}

			if (
				READ_OPS.some((op) => statements[statements.length - 1].toUpperCase().trim().startsWith(op))
			) {
				runHistory.push({
					created_at: new Date().toISOString(),
					created_by: '',
					id: job.id,
					success: true,
					code,
					result
				})
				onData(result)
			}
			if (doPostgresRowToJsonFix)
				sendUserToast('Query failed but recovered with the row_to_json fix')
			else sendUserToast('Query executed')
		} catch (e) {
			console.error(e)
			if (dbType === 'postgresql' && !doPostgresRowToJsonFix) {
				console.error('Error running query, trying with row_to_json fix')
				isRunning = false
				return await run({ doPostgresRowToJsonFix: true, bypassDdlCheck: true })
			}
			sendUserToast('Error running query: ' + (e.message ?? e.error.message), true)
		} finally {
			isRunning = false
		}
	}
	let editor = $state<any | null>(null)
</script>

<Splitpanes>
	<Pane class="relative">
		{#await import('$lib/components/Editor.svelte')}
			<Loader2 class="animate-spin" />
		{:then Module}
			<Module.default
				bind:this={editor}
				bind:code
				scriptLang="mysql"
				class="w-full h-full"
				cmdEnterAction={run}
			/>
		{/await}
		<Button
			wrapperClasses="absolute z-10 bottom-2 right-6"
			variant="accent"
			destructive={isRunning}
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
				const data = e.detail as (typeof runHistory)[number]
				editor?.setCode(data.code)
				onData(data.result)
			}}
		/>
	</Pane>
</Splitpanes>

<ConfirmationModal
	open={ddlWarningOpen}
	title="Schema change detected"
	confirmationText="Run anyway"
	onConfirmed={() => {
		ddlWarningOpen = false
		run({ bypassDdlCheck: true })
	}}
	onCanceled={() => {
		ddlWarningOpen = false
	}}
>
	<p>
		This query contains schema-changing (DDL) statements such as <code>CREATE</code>,
		<code>ALTER</code> or <code>DROP</code>.
	</p>
	<p class="mt-2">
		Running it directly modifies the datatable's structure without being tracked. Schema changes
		should be recorded as migrations so they can be versioned, synced with the CLI and re-applied to
		forks.
	</p>
</ConfirmationModal>
