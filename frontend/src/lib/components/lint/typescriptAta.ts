import { BROWSER } from 'esm-env'
import * as vscode from 'vscode'
import { editor as meditor, Uri as mUri } from 'monaco-editor'
import { typescriptDefaults } from '@codingame/monaco-vscode-standalone-typescript-language-features'
import { get } from 'svelte/store'
import { setupTypeAcquisition, type DepsToGet } from '$lib/ata/index'
import { initWasmTs } from '$lib/infer'
import { parseTypescriptDeps } from '$lib/relative_imports'
import { UserService } from '$lib/gen'
import { lspTokenStore } from '$lib/stores'

// Automatic Type Acquisition: fetches npm and relative-import declarations into the
// global typescriptDefaults. Without it, TypeScript reports every third-party import
// as unresolved, so headless linting and the editor must acquire types the same way.

export async function genAtaRoot(workspace: string): Promise<string> {
	let token = get(lspTokenStore)
	if (!token) {
		const expiration = new Date()
		expiration.setHours(expiration.getHours() + 72)
		const newToken = await UserService.createToken({
			requestBody: { label: 'Ephemeral lsp token', expiration: expiration.toISOString() }
		})
		lspTokenStore.set(newToken)
		token = newToken
	}
	const hostname = BROWSER ? window.location.protocol + '//' + window.location.host : 'SSR'
	return hostname + '/api/scripts_u/tokened_raw/' + workspace + '/' + token
}

export interface WindmillAtaOptions {
	root: string
	/** Script path passed to ATA for resolving relative imports server-side. */
	scriptPath: string | undefined
	/** URI of the model being typed; relative import paths resolve against it. */
	modelUri: string
	absolutePathExtraLibs: Map<string, { dispose: () => void }>
	isCancelled?: () => boolean
	/** Whether relative-import files should be materialized as Monaco models. */
	registerLocalModels?: () => boolean
	/**
	 * Whether a fetched relative-import file may overwrite an existing model. Default true
	 * (the editor keeps import models in sync). Pass false when the caller already owns those
	 * models — e.g. a raw app's own files — so a fetched sibling can't replace one.
	 */
	overwriteLocalModels?: () => boolean
	/** Called after a relative-import model is registered, to nudge revalidation. */
	onLocalFileRegistered?: () => void
}

export async function createWindmillAta(
	opts: WindmillAtaOptions
): Promise<(source: string | DepsToGet) => Promise<void>> {
	const addLibraryToRuntime = async (code: string, _path: string) => {
		const path = 'file://' + _path
		const uri = mUri.parse(path)
		console.log('adding library to runtime', path)
		typescriptDefaults.addExtraLib(code, path)
		try {
			await vscode.workspace.fs.writeFile(uri, new TextEncoder().encode(code))
		} catch (e) {
			console.log('error writing file', e)
		}
	}

	const addLocalFile = async (code: string, _path: string) => {
		if (opts.isCancelled?.()) return
		const p = new URL(_path, opts.modelUri).href
		const nuri = mUri.parse(p)
		console.log('adding local file', _path, nuri.toString())
		// Monaco's TS service resolves relative imports against the importer's URI (finding the
		// model), but absolute paths like "/u/admin/foo" are looked up as raw paths and miss the
		// `file://` model. Register them as extra libs so TS can resolve them.
		if (_path.startsWith('/')) {
			opts.absolutePathExtraLibs.get(_path)?.dispose()
			opts.absolutePathExtraLibs.set(_path, typescriptDefaults.addExtraLib(code, _path))
		}
		if (opts.registerLocalModels?.() ?? true) {
			const localModel = meditor.getModel(nuri)
			if (localModel) {
				if (opts.overwriteLocalModels?.() ?? true) localModel.setValue(code)
			} else {
				meditor.createModel(code, 'typescript', nuri)
			}
			opts.onLocalFileRegistered?.()
		}
	}

	await initWasmTs()
	console.log('SETUP TYPE ACQUISITION', { root: opts.root, path: opts.scriptPath })
	return setupTypeAcquisition({
		projectName: 'Windmill',
		depsParser: (c) => parseTypescriptDeps(c),
		root: opts.root,
		scriptPath: opts.scriptPath,
		logger: console,
		delegate: {
			receivedFile: addLibraryToRuntime,
			localFile: addLocalFile,
			progress: (_downloaded: number, _total: number) => {},
			started: () => {
				console.log('ATA start')
			},
			finished: (_f) => {
				console.log('ATA done')
			}
		}
	})
}
