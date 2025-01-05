<script lang="ts">
	import { get, set } from './idb'
	import type { InstalledPackage } from './npm_install'
	// import { files, installed } from '../stores'
	import { read_tarball } from './tarball'
	import { writable } from 'svelte/store'

	const installed = writable<InstalledPackage[]>([])
	export let logs: string = ''

	async function npm_install(name: string, spec: string): Promise<string | undefined> {
		if (!name) return

		if ($installed.some((e) => e.name === name)) {
			// Already installed.
			return
		}
		// let cached = g

		logs += `Installing ${name} …\n`
		let version
		try {
			version = await resolve_version(name, spec)
		} catch (err) {
			logs += `Failed to resolve ${name}@${spec}: ${err.message}`
			throw err
		}
		logs += `Resolved ${name}@${version}\n`
		let cached = await get(name, version)
		let files
		if (!cached) {
			let buffer: ArrayBuffer | Uint8Array
			let url = `https://registry.npmjs.org/${name}/-/${name.split('/').pop()}-${version}.tgz`

			try {
				buffer = await fetch(url).then((r) => r.arrayBuffer())
			} catch (err) {
				logs += `Failed to fetch ${url}: ${err.message}`
				throw err
			}

			logs += `Extracting ${name}@${version} …\n`
			files = await read_tarball(new Uint8Array(buffer))
			logs += `Extracted ${files.length} files for ${name}@${version}\n`
			await set(name, version, files)
		} else {
			logs += `Using idb cache for ${name}@${version} …\n`
			files = cached
		}

		if (!$installed.some((e) => e.name === name)) {
			$installed.push({ name, version, files })
			$installed = $installed
		}

		const package_json = files.find((e) => e.path === 'package.json')
		if (package_json) {
			return package_json.content
		}
	}

	const semver_loose =
		/^[\s=v]*(\d+)\.(\d+)\.(\d+)(?:-?((?:\d+|\d*[A-Za-z-][\dA-Za-z-]*)(?:\.(?:\d+|\d*[A-Za-z-][\dA-Za-z-]*))*))?(?:\+([\dA-Za-z-]+(?:\.[\dA-Za-z-]+)*))?$/

	const cachedResolutions = {}

	async function resolve_version(name: string, spec: string): Promise<string> {
		const key = `${name}@${spec}`
		let cached = cachedResolutions[key]
		if (cached) {
			logs += `Using cached resolution for ${key}: ${cached}\n`
			return cached
		}
		if (!semver_loose.test(spec)) {
			spec = await follow_redirects(`https://unpkg.com/${name}@${spec}/package.json`)
			spec = spec.slice(18 + name.length + 1)
			spec = spec.slice(0, spec.indexOf('/'))
		}
		cachedResolutions[key] = spec
		return spec
	}

	const fetch_cache = new Map<string, Promise<string>>()
	async function follow_redirects(url: string): Promise<string> {
		if (fetch_cache.has(url)) {
			return fetch_cache.get(url)!
		}

		const promise = fetch(url)
			.then(async (r) => {
				if (r.ok) return r.url
				else throw new Error(await r.text())
			})
			.catch((err) => {
				fetch_cache.delete(url)
				throw err
			})

		fetch_cache.set(url, promise)
		return promise
	}

	export async function parsePackageJson(package_json: string, reset: boolean = false) {
		if (reset) {
			$installed = []
			logs += '\n\n\n=== PACKAGE.JSON INSTALL ===\n\n'
		}
		try {
			const pkg = JSON.parse(package_json)
			let recs = await Promise.all(
				Object.keys(pkg.dependencies || {}).map(async (name) => {
					return await npm_install(name, pkg.dependencies[name])
				})
			)
			for (const rec of recs) {
				if (rec) {
					await parsePackageJson(rec)
				}
			}

			return $installed
		} catch (err) {
			console.error(err)
		}
	}
</script>
