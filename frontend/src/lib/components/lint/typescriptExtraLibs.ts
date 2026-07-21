import { typescriptDefaults } from '@codingame/monaco-vscode-standalone-typescript-language-features'
import { ResourceService, WorkspaceService } from '$lib/gen'
import { formatResourceTypes } from '../copilot/chat/script/core'

// Extra libs that shape TypeScript diagnostics for Windmill code. They are registered
// under fixed URIs in the global typescriptDefaults, so a headless linter and a mounted
// editor necessarily see the same declarations — as long as both go through here.

const RESOURCE_TYPE_LANGS = ['bun', 'deno', 'bunnative', 'nativets'] as const
type ResourceTypeLang = (typeof RESOURCE_TYPE_LANGS)[number]

export async function ensureResourceTypeNamespace(
	workspace: string,
	scriptLang: string | undefined
): Promise<void> {
	if (!RESOURCE_TYPE_LANGS.includes(scriptLang as ResourceTypeLang)) {
		return
	}
	const lang = scriptLang as ResourceTypeLang
	const resourceTypes = await ResourceService.listResourceType({ workspace })
	const namespace = formatResourceTypes(resourceTypes, lang === 'bunnative' ? 'bun' : lang)
	typescriptDefaults.addExtraLib(namespace, 'rt.d.ts')
}

export interface CustomWmillTypesData {
	datatables: string[]
	ducklakes: string[]
}

export async function fetchCustomWmillTypesData(workspace: string): Promise<CustomWmillTypesData> {
	const datatables = (await WorkspaceService.listDataTables({ workspace })).map((d) => d.name)
	const ducklakes = await WorkspaceService.listDucklakes({ workspace })
	return { datatables, ducklakes }
}

export function applyCustomWmillTypes(data: CustomWmillTypesData): () => void {
	const { datatables: datatableNames, ducklakes: ducklakeNames } = data

	const ducklakeNameType = ducklakeNames.length
		? ducklakeNames.map((name) => JSON.stringify(name)).join(' | ')
		: 'string'
	const datatableNameType = datatableNames.length
		? datatableNames.map((name) => JSON.stringify(name)).join(' | ')
		: 'string'
	const isDucklakeOptional = ducklakeNames.includes('main')
	const isDataTableOptional = datatableNames.includes('main')

	const disposeTs = typescriptDefaults.addExtraLib(
		`export {};
			declare module 'windmill-client' {
				import { type DatatableSqlTemplateFunction, type SqlTemplateFunction } from 'windmill-client';
				export function ducklake(name${isDucklakeOptional ? '?' : ''}: ${ducklakeNameType}): SqlTemplateFunction;
				export function datatable(name${isDataTableOptional ? '?' : ''}: ${datatableNameType}): DatatableSqlTemplateFunction;
			}`,
		'file:///custom_wmill_types.d.ts'
	)
	return () => {
		disposeTs.dispose()
	}
}

export async function ensureCustomWmillTypes(workspace: string): Promise<void> {
	applyCustomWmillTypes(await fetchCustomWmillTypesData(workspace))
}

// The two libs above are keyed by workspace but installed at these fixed, global URIs — an
// editor and a headless lint of *different* workspaces would otherwise overwrite each other's
// declarations. A headless lint snapshots them, installs its own, and restores afterwards so a
// mounted editor keeps validating against its own workspace's types.
const WORKSPACE_LIB_URIS = ['rt.d.ts', 'file:///custom_wmill_types.d.ts'] as const
const NEUTRAL_LIB = 'export {};'

export function snapshotWorkspaceExtraLibs(): () => void {
	const libs = typescriptDefaults.getExtraLibs()
	const saved = WORKSPACE_LIB_URIS.map((uri) => ({ uri, content: libs[uri]?.content }))
	return () => {
		const now = typescriptDefaults.getExtraLibs()
		for (const { uri, content } of saved) {
			if (content !== undefined) {
				// Restore the prior content (a no-op when the lint left it unchanged).
				typescriptDefaults.addExtraLib(content, uri)
			} else if (now[uri] !== undefined) {
				// The lint installed a lib where none existed; there is no editor declaration to
				// restore, so neutralize it rather than leave one workspace's types resident.
				typescriptDefaults.addExtraLib(NEUTRAL_LIB, uri)
			}
		}
	}
}
