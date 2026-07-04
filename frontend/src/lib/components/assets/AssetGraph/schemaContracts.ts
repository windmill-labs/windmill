// Client-side mirror of the save-time schema-contract check (pipelines gap
// #2b, backend/windmill-common/src/schema_contracts.rs). The backend endpoint
// (`checkSchemaContracts`) is the authoritative check run on save; this mirror
// drives the *live* editor surface (Monaco warning markers + completions) from
// the WASM parse that already runs on the open buffer, so the two must apply
// the same rules: ducklake-only, case-insensitive column names, `columns`
// absent ⇒ skip, `_wm_partition` whitelisted, `{partition}` token stripped,
// annotation-declared lineage only, asset without captured schema ⇒ silent.

import { AssetService, ScriptService, type ContractWarning, type ScriptLang } from '$lib/gen'
import { sendUserToast } from '$lib/toast'
import type { AssetWithAltAccessType } from '../lib'
import {
	parsePipelineAnnotations,
	type ColumnLineage,
	type DataTest,
	type MaterializeSpec
} from './parsePipelineAnnotations'

// Columns the materialize engine manages; excluded from the captured schema on
// purpose, so reads of them must not warn.
const RESERVED_COLUMNS = ['_wm_partition']

const PARTITION_TOKEN = '{partition}'

export type CapturedSchemaLite = {
	columns: { name: string; type: string }[]
	version: number
	capturedAt: string
}

// Strip the `{partition}` token a declared URI may carry so lookups hit the
// captured path (mirrors `normalize_asset_path`).
export function normalizeAssetPath(path: string): string {
	return path
		.replaceAll('/' + PARTITION_TOKEN, '')
		.replaceAll(PARTITION_TOKEN, '')
		.replace(/\/+$/, '')
}

function isReserved(name: string): boolean {
	return RESERVED_COLUMNS.some((r) => r.toLowerCase() === name.toLowerCase())
}

function findColumn(
	schema: CapturedSchemaLite,
	name: string
): { name: string; type: string } | undefined {
	const lower = name.toLowerCase()
	return schema.columns.find((c) => c.name.toLowerCase() === lower)
}

export type SchemaContractInputs = {
	// Per-asset column reads/writes from the WASM parse (entries without a
	// `columns` map are skipped — wildcard/unknown access).
	assets: AssetWithAltAccessType[]
	// Annotation-declared `// column` lineage ONLY (not merged AST-inferred
	// lineage — redundant with body reads and alias-attribution can misfire).
	columnLineage: ColumnLineage[]
	dataTests: DataTest[]
	materialize?: MaterializeSpec
	// Latest captured schema per normalized ducklake path (after any
	// `_current` → base-table fallback the caller resolved).
	schemas: Map<string, CapturedSchemaLite>
	// Normalized paths whose producer declares `on_schema_change=ignore`.
	ignored: Set<string>
}

// Mirrors backend `diff_contract` — same warning kinds and suppression
// semantics, minus the human message wording (the editor renders its own).
export function diffSchemaContracts(input: SchemaContractInputs): ContractWarning[] {
	const { assets, columnLineage, dataTests, materialize, schemas, ignored } = input
	const warnings: ContractWarning[] = []

	// W1 — body-read/written columns missing from the captured schema.
	for (const a of assets) {
		if (a.kind !== 'ducklake' || a.columns == undefined) continue
		const path = normalizeAssetPath(a.path)
		const schema = schemas.get(path)
		if (!schema) continue
		for (const col of Object.keys(a.columns)) {
			if (col === '*' || isReserved(col)) continue
			if (!findColumn(schema, col)) {
				warnings.push({
					kind: 'missing_column',
					asset_path: path,
					column: col,
					schema_version: schema.version,
					captured_at: schema.capturedAt,
					message: `column \`${col}\` of ducklake://${path} is not in its captured schema (v${schema.version}, columns: ${schema.columns.map((c) => c.name).join(', ')})`
				})
			}
		}
	}

	// W2 — `// column` lineage source refs.
	for (const cl of columnLineage) {
		for (const input of cl.inputs) {
			if (input.from_kind !== 'ducklake' || isReserved(input.from_column)) continue
			const path = normalizeAssetPath(input.from_path)
			const schema = schemas.get(path)
			if (!schema) continue
			if (!findColumn(schema, input.from_column)) {
				warnings.push({
					kind: 'missing_lineage_source',
					asset_path: path,
					column: input.from_column,
					schema_version: schema.version,
					captured_at: schema.capturedAt,
					message: `\`// column ${cl.column}\` reads \`${input.from_column}\` from ducklake://${path}, which is not in its captured schema (v${schema.version})`
				})
			}
		}
	}

	// W3 — relationships refs: missing column, and captured-type difference
	// when the consumer's own materialize target has a capture. Types still
	// coerce at run time, so a difference is "differs", never "will fail".
	const ownSchema =
		materialize?.targetKind === 'ducklake'
			? schemas.get(normalizeAssetPath(materialize.targetPath))
			: undefined
	for (const dt of dataTests) {
		if (dt.type !== 'relationships' || dt.to_kind !== 'ducklake') continue
		const path = normalizeAssetPath(dt.to_path)
		const schema = schemas.get(path)
		if (!schema) continue
		const refCol = findColumn(schema, dt.to_column)
		if (!refCol) {
			warnings.push({
				kind: 'missing_relationship_column',
				asset_path: path,
				column: dt.to_column,
				schema_version: schema.version,
				captured_at: schema.capturedAt,
				message: `\`// data_test relationships ${dt.column}\` references ducklake://${path}.${dt.to_column}, which is not in its captured schema (v${schema.version})`
			})
		} else {
			const ownCol = ownSchema && findColumn(ownSchema, dt.column)
			if (ownCol && ownCol.type.toLowerCase() !== refCol.type.toLowerCase()) {
				warnings.push({
					kind: 'relationship_type_mismatch',
					asset_path: path,
					column: dt.to_column,
					expected_type: ownCol.type,
					found_type: refCol.type,
					schema_version: schema.version,
					captured_at: schema.capturedAt,
					message: `\`// data_test relationships ${dt.column}\` joins \`${dt.column}\` (${ownCol.type}) to ducklake://${path}.${dt.to_column} (${refCol.type}) — captured types differ`
				})
			}
		}
	}

	// W4 — producer `on_schema_change=ignore`: drop the asset's warnings,
	// leaving one informational entry per suppressed asset.
	if (ignored.size > 0) {
		const suppressed: string[] = []
		const kept = warnings.filter((w) => {
			if (ignored.has(w.asset_path)) {
				if (!suppressed.includes(w.asset_path)) suppressed.push(w.asset_path)
				return false
			}
			return true
		})
		warnings.length = 0
		warnings.push(...kept)
		for (const path of suppressed) {
			warnings.push({
				kind: 'suppressed',
				asset_path: path,
				message: `schema mismatches on ducklake://${path} suppressed by its producer's \`on_schema_change=ignore\``
			})
		}
	}

	return warnings
}

// The ducklake paths a buffer references in ways the contract check inspects —
// what the editor needs captured schemas for.
export function referencedDucklakePaths(
	input: Pick<SchemaContractInputs, 'assets' | 'columnLineage' | 'dataTests' | 'materialize'>
): string[] {
	const paths = new Set<string>()
	for (const a of input.assets) {
		if (a.kind === 'ducklake' && a.columns != undefined) paths.add(normalizeAssetPath(a.path))
	}
	for (const cl of input.columnLineage) {
		for (const i of cl.inputs) {
			if (i.from_kind === 'ducklake') paths.add(normalizeAssetPath(i.from_path))
		}
	}
	for (const dt of input.dataTests) {
		if (dt.type === 'relationships' && dt.to_kind === 'ducklake')
			paths.add(normalizeAssetPath(dt.to_path))
	}
	if (input.materialize?.targetKind === 'ducklake')
		paths.add(normalizeAssetPath(input.materialize.targetPath))
	return [...paths]
}

// --- Captured-schema cache ------------------------------------------------

// Short-TTL cache so per-keystroke recomputes and completion requests don't
// re-fetch. Captured schemas only change when a producer materializes, so a
// briefly stale hit is fine — the authoritative save-time check re-reads.
const SCHEMA_TTL_MS = 30_000
const schemaCache = new Map<string, { at: number; value: CapturedSchemaLite | undefined }>()

export async function fetchLatestSchema(
	workspace: string,
	path: string
): Promise<CapturedSchemaLite | undefined> {
	const key = `${workspace}:${path}`
	const hit = schemaCache.get(key)
	if (hit && Date.now() - hit.at < SCHEMA_TTL_MS) return hit.value
	let value: CapturedSchemaLite | undefined = undefined
	try {
		const versions = await AssetService.listAssetSchemas({ workspace, path })
		const latest = versions[0]
		if (latest) {
			value = {
				columns: latest.columns,
				version: latest.version,
				capturedAt: latest.captured_at
			}
		}
	} catch (e) {
		console.error('failed to fetch captured asset schema', path, e)
	}
	schemaCache.set(key, { at: Date.now(), value })
	return value
}

// Resolve the schema map for a set of referenced paths, applying the scd2
// `<dim>_current` → base-table fallback when the graph identifies the view's
// producer as a managed scd2 materializer (the view is `SELECT * … WHERE
// is_current`, so columns are identical).
export async function fetchSchemasForPaths(
	workspace: string,
	paths: string[],
	scd2CurrentBase?: (path: string) => string | undefined
): Promise<Map<string, CapturedSchemaLite>> {
	const out = new Map<string, CapturedSchemaLite>()
	await Promise.all(
		paths.map(async (p) => {
			let schema = await fetchLatestSchema(workspace, p)
			if (!schema && p.endsWith('_current')) {
				const base = scd2CurrentBase?.(p)
				if (base) schema = await fetchLatestSchema(workspace, base)
			}
			if (schema) out.set(p, schema)
		})
	)
	return out
}

// --- Pipeline-graph context ---------------------------------------------------

// Producer-side facts the contract mirror needs but cannot derive from the
// open buffer: which assets are muted (`on_schema_change=ignore`) and which
// `<dim>_current` views map to an scd2 base table. Built by the pipeline page
// from the resolved graph; absent outside the pipeline editor (standalone
// script editor), where suppression simply doesn't apply client-side — the
// save-time server check remains authoritative either way.
export type SchemaContractGraphContext = {
	// Normalized asset paths whose producer declares `on_schema_change=ignore`.
	ignoredAssets: string[]
	// `<base>_current` → base for managed scd2 producers in the graph.
	scd2CurrentBases: Record<string, string>
}

export function buildSchemaContractContext(
	runnables: Pick<
		import('./types').AssetGraphRunnableNode,
		'materialize_target' | 'materialize_strategy' | 'materialize_on_schema_change'
	>[]
): SchemaContractGraphContext {
	const ignoredAssets: string[] = []
	const scd2CurrentBases: Record<string, string> = {}
	for (const r of runnables) {
		const t = r.materialize_target
		if (!t || t.kind !== 'ducklake') continue
		const base = normalizeAssetPath(t.path)
		if (r.materialize_on_schema_change === 'ignore') {
			ignoredAssets.push(base, `${base}_current`)
		}
		if (r.materialize_strategy === 'scd2') {
			scd2CurrentBases[`${base}_current`] = base
		}
	}
	return { ignoredAssets, scd2CurrentBases }
}

// --- Save-time surface ------------------------------------------------------

// Run the authoritative backend check for just-deployed content and toast the
// result. Never throws — a failed check must not taint a successful deploy.
export async function notifyContractWarnings(
	workspace: string,
	language: ScriptLang,
	content: string
): Promise<void> {
	// Every checkable ref carries the `ducklake` token (URIs and the bare
	// default-syntax shorthand alike) — skip the round-trip for the vast
	// majority of saves that can't produce a warning.
	if (!content.includes('ducklake')) return
	try {
		const { warnings } = await ScriptService.checkSchemaContracts({
			workspace,
			requestBody: { language, content }
		})
		const real = warnings.filter((w) => w.kind !== 'suppressed')
		if (real.length === 0) return
		sendUserToast(
			`Schema contract: ${real.length} warning${real.length > 1 ? 's' : ''}`,
			'warning',
			[],
			real.map((w) => `• ${w.message}`).join('\n'),
			10000
		)
	} catch (e) {
		console.error('schema-contract check failed', e)
	}
}

// End-to-end live-editor check: parse the buffer's annotations, resolve the
// captured schemas for everything it references, diff, and anchor the result
// to source positions. Cheap per keystroke — the annotation parse is a line
// scan and schema fetches hit the short-TTL cache.
export async function computeContractMarkers(
	workspace: string,
	code: string,
	assets: AssetWithAltAccessType[],
	context?: SchemaContractGraphContext
): Promise<ContractMarker[]> {
	const ann = parsePipelineAnnotations(code)
	const inputs = {
		assets,
		// Annotation-declared lineage only — body-inferred lineage is redundant
		// with the body-read check and its alias attribution can misfire.
		columnLineage: ann.columnLineage,
		dataTests: ann.dataTests,
		materialize: ann.materialize
	}
	const refs = referencedDucklakePaths(inputs)
	if (refs.length === 0) return []
	const schemas = await fetchSchemasForPaths(
		workspace,
		refs,
		context ? (p) => context.scd2CurrentBases[p] : undefined
	)
	if (schemas.size === 0) return []
	const warnings = diffSchemaContracts({
		...inputs,
		schemas,
		ignored: new Set(context?.ignoredAssets ?? [])
	})
	return mapWarningsToMarkers(code, warnings)
}

// --- Editor marker mapping ---------------------------------------------------

export type ContractMarker = {
	message: string
	startLineNumber: number
	startColumn: number
	endLineNumber: number
	endColumn: number
}

// Best-effort source anchoring: annotation-family warnings anchor to their
// annotation line; body-read warnings anchor to the first occurrence of the
// column identifier; fallback is the `// on`/first line mentioning the asset.
export function mapWarningsToMarkers(code: string, warnings: ContractWarning[]): ContractMarker[] {
	const lines = code.split('\n')

	function lineMatching(pred: (line: string) => boolean): number | undefined {
		const idx = lines.findIndex(pred)
		return idx >= 0 ? idx + 1 : undefined
	}

	function tokenRange(
		lineNumber: number,
		token: string
	): { startColumn: number; endColumn: number } {
		const line = lines[lineNumber - 1] ?? ''
		const idx = line.toLowerCase().indexOf(token.toLowerCase())
		if (idx < 0) return { startColumn: 1, endColumn: line.length + 1 }
		return { startColumn: idx + 1, endColumn: idx + 1 + token.length }
	}

	return warnings
		.filter((w) => w.kind !== 'suppressed')
		.map((w) => {
			let lineNumber: number | undefined
			let token: string | undefined = w.column ?? undefined
			switch (w.kind) {
				case 'missing_lineage_source':
					lineNumber = lineMatching(
						(l) => /^\s*(\/\/|--|#)\s*column\s/.test(l) && !!w.column && l.includes(w.column)
					)
					break
				case 'missing_relationship_column':
				case 'relationship_type_mismatch':
					lineNumber = lineMatching(
						(l) =>
							/^\s*(\/\/|--|#)\s*data_test\s+relationships\s/.test(l) && l.includes(w.asset_path)
					)
					break
				case 'missing_column': {
					// first body occurrence of the column identifier
					const re = new RegExp(`\\b${w.column?.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}\\b`, 'i')
					lineNumber = w.column ? lineMatching((l) => re.test(l)) : undefined
					break
				}
			}
			if (lineNumber == undefined) {
				// fallback: the `// on …` (or any) line mentioning the asset path
				lineNumber = lineMatching((l) => l.includes(w.asset_path)) ?? 1
				token = w.asset_path
			}
			const range = token
				? tokenRange(lineNumber, token)
				: { startColumn: 1, endColumn: (lines[lineNumber - 1]?.length ?? 0) + 1 }
			return {
				message: w.message,
				startLineNumber: lineNumber,
				endLineNumber: lineNumber,
				...range
			}
		})
}
