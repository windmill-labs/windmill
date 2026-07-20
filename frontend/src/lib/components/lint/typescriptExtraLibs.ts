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

export async function ensureCustomWmillTypes(workspace: string): Promise<() => void> {
	return applyCustomWmillTypes(await fetchCustomWmillTypesData(workspace))
}
