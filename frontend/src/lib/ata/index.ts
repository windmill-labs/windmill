import { isTypescriptRelativePath } from '$lib/relative_imports'
import {
	getDTSFileForModuleWithVersion,
	getFiletreeForModuleWithVersion,
	getNPMVersionForModuleReference,
	getNPMVersionsForModule,
	isOverlimit,
	limit,
	type NPMTreeMeta,
	type ResLimit
} from './apis'
import { mapModuleNameToModule } from './edgeCases'

export interface ATABootstrapConfig {
	root: string
	scriptPath?: string
	/** A object you pass in to get callbacks */
	delegate: {
		/** The callback which gets called when ATA decides a file needs to be written to your VFS  */
		receivedFile: (code: string, path: string) => void
		localFile: (code: string, path: string) => void
		/** A way to display progress */
		progress?: (downloaded: number, estimatedTotal: number) => void
		/** Note: An error message does not mean ATA has stopped! */
		errorMessage?: (userFacingMessage: string, error: Error) => void
		/** A callback indicating that ATA actually has work to do */
		started?: () => void
		/** The callback when all ATA has finished */
		finished?: (files: Map<string, string>) => void
	}
	/** Passed to fetch as the user-agent */
	projectName: string
	/** code to dependency parser */
	depsParser: (code: string) => string[]
	/** If you need a custom logger instead of the console global */
	logger?: Logger
}

type ModuleMeta = { state: 'loading' }

export type DepsToGet = {
	raw: string
	module: string
	version: string | undefined
}[]

function getVersionFromRaw(d: string) {
	if (d.lastIndexOf('@') > 0) {
		const splitted = d.split('@')
		let version = splitted.pop()
		if (version?.startsWith('^') || version?.startsWith('~')) {
			version = version.slice(1)
		}
		return version
	}
	return 'latest'
}

export function versionRangeToVersion(version: string) {
	if (version.startsWith('^') || version.startsWith('~')) {
		return version.slice(1)
	}
	return version
}
/**
 * The function which starts up type acquisition,
 * returns a function which you then pass the initial
 * source code for the app with.
 *
 * This is effectively the main export, everything else is
 * basically exported for tests and should be considered
 * implementation details by consumers.
 */
export const setupTypeAcquisition = (config: ATABootstrapConfig) => {
	const moduleMap = new Map<string, ModuleMeta>()
	const fsMap = new Map<string, string>()

	limit.clearQueue()
	let estimatedToDownload = 0
	let estimatedDownloaded = 0

	let resLimit = { usage: 0 }

	return async (initialSourceFile: string | DepsToGet) => {
		estimatedToDownload = 0
		estimatedDownloaded = 0

		let todo: (string | DepsToGet)[] = [initialSourceFile]
		let next: string[] = []
		let i = 0
		let nb = 0
		let time = new Date().getTime()
		while (todo.length && nb < 200 && new Date().getTime() - time < 1000 * 15) {
			const current = todo.shift()!
			nb += 1
			const deps = await resolveDeps(current, i, resLimit)
			if (i <= 0) {
				next.push(...deps)
			}
			if (todo.length === 0) {
				i += 1
				todo = next
				next = []
			}
		}
		if (estimatedDownloaded > 0) {
			config.delegate.finished?.(fsMap)
		}
	}

	async function resolveDeps(
		depsSource: string | DepsToGet,
		depth: number,
		resLimit: ResLimit
	): Promise<string[]> {
		let depsToGet =
			typeof depsSource == 'object'
				? depsSource
				: config
						.depsParser(depsSource)
						.map((d: string) => {
							let raw = mapModuleNameToModule(d)
							return {
								raw,
								module: raw.lastIndexOf('@') > 0 ? raw.split('@').slice(0, -1).join('@') : raw,
								version: getVersionFromRaw(raw)
							}
						})
						.filter((f) => !moduleMap.has(f.raw))

		if (depth == 0) {
			const relativeDeps = depsToGet.filter((f) => isTypescriptRelativePath(f.raw))
			relativeDeps.forEach(async (f) => {
				let path = f.raw.startsWith('/')
					? f.raw
					: '/' + config.scriptPath + (f.raw.startsWith('../') ? '/../' : '/.') + f.raw
				let url = config.root + path
				let localPath = f.raw
				if (f.raw.startsWith('.') && !f.raw.endsWith('.ts')) {
					url += '.ts'
					localPath += '.ts'
				}

				console.log('fetching local file', url, f.raw, localPath)
				const res = await fetch(url)
				if (res.ok) {
					config.delegate.localFile?.(await res.text(), localPath)
				}
			})
		}
		depsToGet.forEach((dep) => moduleMap.set(dep.raw, { state: 'loading' }))

		depsToGet = depsToGet.filter((f) => !isTypescriptRelativePath(f.raw))
		if (depsToGet.length === 0) {
			return []
		}
		console.log(
			'dependencies to fetch for type assistant: ',
			depsToGet.map((x) => x.raw)
		)
		// Make it so it won't get re-downloaded

		// Grab the module trees which gives us a list of files to download
		const trees = await Promise.all(
			depsToGet.map((f) => getFileTreeForModuleWithTag(f.module, f.version, f.raw, resLimit))
		)
		const treesOnly = trees.filter((t) => !('error' in t)) as NPMTreeMeta[]

		// These are the modules which we can grab directly
		const hasDTS = treesOnly.filter((t) => t.files.find((f) => f.name.endsWith('.d.ts')))
		const dtsFilesFromNPM = hasDTS.map((t) => treeToDTSFiles(t, `/node_modules/${t.raw}`))
		// console.log(dtsFilesFromNPM, 'dtsFilesFromNPM')
		// These are ones we need to look on DT for (which may not be there, who knows)
		const mightBeOnDT = treesOnly.filter((t) => !hasDTS.includes(t))
		// console.log(mightBeOnDT, 'mightBeOnDT')
		// return

		const dtTrees = await Promise.all(
			// TODO: Switch from 'latest' to the version from the original tree which is user-controlled
			mightBeOnDT.map((f) =>
				getFileTreeForModuleWithTag(
					`@types/${getDTName(f.moduleName)}`,
					'latest',
					`@types/${getDTName(f.raw)}`,
					resLimit
				)
			)
		)

		const dtTreesOnly = dtTrees.filter((t) => !('error' in t)) as NPMTreeMeta[]
		const dtsFilesFromDT = dtTreesOnly.map((t) =>
			treeToDTSFiles(t, `/node_modules/@types/${getDTName(t.raw).replace('types__', '')}`)
		)

		// Collect all the npm and DT DTS requests and flatten their arrays
		const allDTSFiles = dtsFilesFromNPM.concat(dtsFilesFromDT).reduce((p, c) => p.concat(c), [])

		estimatedToDownload += allDTSFiles.length
		if (allDTSFiles.length && depth === 0) {
			config.delegate.started?.()
		}

		// Grab the package.jsons for each dependency
		for (const tree of treesOnly) {
			const pkgJSON = await getDTSFileForModuleWithVersion(
				tree.moduleName,
				tree.version,
				'/package.json'
			)
			let prefix = `/node_modules/${tree.raw}`
			if (dtTreesOnly.includes(tree))
				prefix = `/node_modules/@types/${getDTName(tree.raw).replace('types__', '')}`
			const path = prefix + '/package.json'

			if (typeof pkgJSON == 'string') {
				fsMap.set(path, pkgJSON)
				config.delegate.receivedFile?.(pkgJSON, path)
			} else {
				config.logger?.error(`Could not download package.json for ${tree.moduleName}`)
			}
		}

		// Grab all dts files
		return (
			await Promise.all(
				allDTSFiles.map(async (dts) => {
					if (isOverlimit(resLimit)) {
						console.warn('Exceeded limit of types downloaded for the needs of the assistant')
						return
					}
					const dtsCode = await getDTSFileForModuleWithVersion(
						dts.moduleName,
						dts.moduleVersion,
						dts.path
					)
					estimatedDownloaded++
					if (dtsCode instanceof Error) {
						// TODO?
						config.logger?.error(`Had an issue getting ${dts.path} for ${dts.moduleName}`)
					} else {
						fsMap.set(dts.vfsPath, dtsCode)
						config.delegate.receivedFile?.(dtsCode, dts.vfsPath)

						// Send a progress note every 5 downloads
						if (config.delegate.progress && estimatedDownloaded % 5 === 0) {
							config.delegate.progress(estimatedDownloaded, estimatedToDownload)
						}

						if (dts.moduleName != 'bun-types') {
							return dtsCode
							// Recurse through deps
						}
					}
				})
			)
		).filter((f) => f != undefined) as string[]
	}
}

type ATADownload = {
	moduleName: string
	moduleVersion: string
	vfsPath: string
	path: string
}

function treeToDTSFiles(tree: NPMTreeMeta, vfsPrefix: string) {
	const dtsRefs: ATADownload[] = []

	for (const file of tree.files) {
		if (file.name.endsWith('.d.ts')) {
			dtsRefs.push({
				moduleName: tree.moduleName,
				moduleVersion: tree.version,
				vfsPath: `${vfsPrefix}${file.name}`,
				path: file.name
			})
		}
	}
	return dtsRefs
}

/** The bulk load of the work in getting the filetree based on how people think about npm names and versions */
export const getFileTreeForModuleWithTag = async (
	moduleName: string,
	tag: string | undefined,
	raw: string,
	resLimit: ResLimit
) => {
	let toDownload = tag || 'latest'

	// I think having at least 2 dots is a reasonable approx for being a semver and not a tag,
	// we can skip an API request, TBH this is probably rare
	if (toDownload.split('.').length < 2) {
		// The jsdelivr API needs a _version_ not a tag. So, we need to switch out
		// the tag to the version via an API request.
		const response = await getNPMVersionForModuleReference(moduleName, toDownload, resLimit)
		if (response instanceof Error) {
			return {
				error: response,
				userFacingMessage: `Could not go from a tag to version on npm for ${moduleName} - possible typo?`
			}
		}

		const neededVersion = response.version
		if (!neededVersion) {
			const versions = await getNPMVersionsForModule(moduleName, resLimit)
			if (versions instanceof Error) {
				return {
					error: response,
					userFacingMessage: `Could not get versions on npm for ${moduleName} - possible typo?`
				}
			}

			const tags = Object.entries(versions.tags).join(', ')
			return {
				error: new Error('Could not find tag for module'),
				userFacingMessage: `Could not find a tag for ${moduleName} called ${tag}. Did find ${tags}`
			}
		}

		toDownload = neededVersion
	}

	const res = await getFiletreeForModuleWithVersion(moduleName, toDownload, raw, resLimit)
	if (res instanceof Error) {
		return {
			error: res,
			userFacingMessage: `Could not get the files for ${moduleName}@${toDownload}. Is it possibly a typo?`
		}
	}

	return res
}

interface Logger {
	log: (...args: any[]) => void
	error: (...args: any[]) => void
	groupCollapsed: (...args: any[]) => void
	groupEnd: (...args: any[]) => void
}

// Taken from dts-gen: https://github.com/microsoft/dts-gen/blob/master/lib/names.ts
function getDTName(s: string) {
	if (s.indexOf('@') === 0 && s.indexOf('/') !== -1) {
		// we have a scoped module, e.g. @bla/foo
		// which should be converted to   bla__foo
		s = s.substr(1).replace('/', '__')
	}
	return s
}
