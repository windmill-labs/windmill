<script lang="ts">
	import { CornerDownLeft, History, Loader2, X } from 'lucide-svelte'
	import Button from './common/button/Button.svelte'
	import { runScriptAndPollResult } from './jobs/utils'
	import { writingJobOptions } from './jobs/writingJob'
	import { sendUserToast } from '$lib/toast'
	import { untrack } from 'svelte'
	import { getLanguageByResourceType } from './apps/components/display/dbtable/utils'
	import StepHistory, { type StepHistoryData } from './flows/propPicker/StepHistory.svelte'
	import { getDatabaseArg, getDbType } from './dbOps'
	import type { DbInput } from './dbTypes'
	import { wrapDucklakeQuery } from './ducklake'
	import { splitSqlStatements, pruneComments } from './sqlDdl'
	import DdlMigrationGuard from './DdlMigrationGuard.svelte'
	import { useOperatingWorkspace } from '$lib/components/operatingWorkspace.svelte'

	const operatingWorkspace = useOperatingWorkspace()

	type Props = {
		input: DbInput
		/** `ranCode` is the editor content that produced `data`, captured when the
		 * run started, so a caller can discard a late response whose query it has
		 * moved past. */
		onData: (data: Record<string, any>[], ranCode: string) => void
		placeholderTableName?: string
		/** Called after a migration is run via the DDL guard, so the schema view
		 * can be refreshed to reflect the applied change. */
		onSchemaChange?: () => void
		/** Workspace the REPL queries and DDL migrations run against; defaults to
		 * the nav workspace. */
		workspace?: string | undefined
		/** Worker tag the queries run on instead of the language's native one. */
		tag?: string | undefined
		/** Editor content to start from instead of the placeholder query. */
		initialCode?: string
		onCodeChange?: (code: string) => void
	}
	let {
		input,
		onData,
		placeholderTableName,
		onSchemaChange,
		workspace = undefined,
		tag = undefined,
		initialCode,
		onCodeChange
	}: Props = $props()
	let ws = $derived(workspace ?? $operatingWorkspace)
	let dbType = $derived(getDbType(input))

	// A datatable REPL targets `datatable://<name>`; surface DDL statements as
	// migration prompts instead of running them ad-hoc.
	let datatableName = $derived(
		input.type === 'database' && input.resourcePath.startsWith('datatable://')
			? input.resourcePath.slice('datatable://'.length).split('/')[0]
			: undefined
	)
	let ddlGuard = $state<DdlMigrationGuard | undefined>(undefined)

	const DEFAULT_SQL = 'SELECT * FROM _'
	let code = $state(untrack(() => initialCode) ?? DEFAULT_SQL)

	/** Seed the editor from outside, e.g. a query composed in a drawer. */
	export function setCode(newCode: string) {
		code = newCode
		editor?.setCode?.(newCode)
	}
	$effect(() => {
		const _code = untrack(() => code)
		if (placeholderTableName && _code === DEFAULT_SQL) {
			code = _code.replace('_', placeholderTableName ?? 'table')
		}
	})
	let isRunning = $state(false)

	let runHistory: (StepHistoryData & { code: string; result: Record<string, any>[] })[] = $state([])

	async function run({ doPostgresRowToJsonFix }: { doPostgresRowToJsonFix?: boolean } = {}) {
		if (isRunning || !ws) return
		const READ_OPS = ['SELECT', 'WITH', 'SHOW', 'EXPLAIN', 'DESCRIBE']

		// On a datatable, intercept DDL statements and offer to make them
		// migrations; the user may strip some, leaving the rest to run.
		if (datatableName && ddlGuard && !doPostgresRowToJsonFix) {
			const res = await ddlGuard.guard(code)
			// A migration that was created and run changes the schema; refresh it.
			if (res.ranMigration) onSchemaChange?.()
			if (!res.proceed) return
			if (res.code !== code) code = res.code
			if (pruneComments(code).trim() === '') return
		}

		// Snapshot the code that will actually run: after the DDL guard has stripped
		// any migrated statements, and before the execution await during which
		// `code` can be re-seeded (setCode). The caller identifies which query a late
		// response belongs to by this value, and history restores exactly it.
		const ranCode = code

		isRunning = true
		try {
			const statements = splitSqlStatements(pruneComments(code))
			if (statements.length === 0) {
				sendUserToast('Nothing to run', true)
				return
			}

			// Transform all to JSON in case of select. This fixes the issue of
			// custom postgres enum type failing to convert to a rust type in the backend.
			// We don't always put the fix by default for row ordering concerns
			let transformedCode = code
			if (doPostgresRowToJsonFix) {
				// Rebuilt from the pruned statements, which drops the leading comment block — and
				// with it the `-- role <name>` annotation that decides which login the query runs
				// as. Carry it over, or the retry connects as the data table's default role and a
				// query the first attempt was denied succeeds on the second.
				const leadingAnnotations = code.match(/^(?:[^\S\n]*\n|[^\S\n]*--[^\n]*\n)*/)?.[0] ?? ''
				transformedCode =
					leadingAnnotations +
					statements
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
					workspace: ws,
					requestBody: {
						language: getLanguageByResourceType(dbType),
						content: transformedCode,
						args: dbArg,
						tag
					}
				},
				// The user types arbitrary SQL here, so treat every run as a write.
				{ withJobData: true, ...writingJobOptions }
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
					// The snapshot, not the live `code`: a mid-run setCode() must not
					// rebind this entry's result to a different query. Selecting it later
					// reseeds the editor with this exact SQL and re-fires onData with it.
					code: ranCode,
					result
				})
				onData(result, ranCode)
			}
			if (doPostgresRowToJsonFix)
				sendUserToast('Query failed but recovered with the row_to_json fix')
			else sendUserToast('Query executed')
		} catch (e) {
			console.error(e)
			if (dbType === 'postgresql' && !doPostgresRowToJsonFix) {
				console.error('Error running query, trying with row_to_json fix')
				isRunning = false
				return await run({ doPostgresRowToJsonFix: true })
			}
			sendUserToast(
				'Error running query: ' + (e?.body ?? e?.message ?? e?.error?.message ?? e),
				true
			)
		} finally {
			isRunning = false
		}
	}
	let editor = $state<any | null>(null)
	let historyOpen = $state(true)
</script>

<div class="relative h-full w-full">
	{#await import('$lib/components/Editor.svelte')}
		<Loader2 class="animate-spin" />
	{:then Module}
		<Module.default
			bind:this={editor}
			bind:code={() => code, (v) => ((code = v), onCodeChange?.(v))}
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
	<!-- Floats over the editor rather than taking a pane of its own, so the query keeps the
		 full width. -->
	{#if historyOpen}
		<div
			class="absolute right-6 top-3 z-10 flex h-96 w-72 flex-col overflow-hidden rounded-lg border bg-surface-tertiary shadow-md"
			style="max-height: calc(100% - 4.5rem)"
		>
			<div class="flex items-center justify-between border-b py-1 pl-3 pr-1">
				<span class="text-xs font-semibold text-emphasis">Run history</span>
				<Button
					variant="subtle"
					unifiedSize="xs"
					iconOnly
					startIcon={{ icon: X }}
					title="Hide run history"
					onClick={() => (historyOpen = false)}
				/>
			</div>
			<div class="min-h-0 flex-1 overflow-auto">
				<StepHistory
					staticInputs={runHistory}
					on:select={(e) => {
						const data = e.detail as (typeof runHistory)[number]
						editor?.setCode(data.code)
						onData(data.result, data.code)
					}}
				/>
			</div>
		</div>
	{:else}
		<div class="absolute right-6 top-3 z-10">
			<Button
				variant="default"
				unifiedSize="sm"
				iconOnly
				btnClasses="bg-surface-tertiary"
				startIcon={{ icon: History }}
				title="Show run history"
				onClick={() => (historyOpen = true)}
			/>
		</div>
	{/if}
</div>

{#if datatableName && ws}
	<DdlMigrationGuard
		bind:this={ddlGuard}
		workspace={ws}
		datatable={datatableName}
		role={input.type === 'database' ? (input.role ?? input.migrationRole) : undefined}
	/>
{/if}
