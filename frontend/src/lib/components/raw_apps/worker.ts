import * as esbuild from 'esbuild-wasm'
import { resetFileSystem } from './fs'

let isInitialized = false

export const externalGlobalPlugin = (externals: Record<string, string>) => {
	const namespace = 'external-global'
	return {
		name: namespace,
		setup(build) {
			build.onResolve(
				{
					filter: new RegExp('^(' + Object.keys(externals).join('|') + ')$')
				},
				(args) => ({
					path: args.path,
					namespace: namespace
				})
			)

			build.onLoad(
				{
					filter: /.*/,
					namespace
				},
				(args) => {
					const contents = `module.exports = ${externals[args.path]}`
					return {
						contents
					}
				}
			)
		}
	}
}

async function vuePlugin(files: Record<string, string>, version) {
	if (version != '3.5.13') {
		throw new Error(`Vue version ${version} is not supported, use 3.5.13`)
	}
	// @ts-ignore
	const { parse, compileTemplate, compileScript, compileStyle } = await import(
		// @ts-ignore
		'https://unpkg.com/@vue/compiler-sfc@3.5.13/dist/compiler-sfc.esm-browser.js'
	)

	return {
		name: 'vue',
		setup(build) {
			build.onResolve({ filter: /\.vue\.\d+\.css$/ }, (args) => {
				return {
					path: args.path,
					namespace: 'vue-css'
				}
			})

			const styleMap = new Map()

			build.onLoad({ filter: /.*/, namespace: 'vue-css' }, (args) => {
				const css = styleMap.get(args.path)
				return {
					contents: css,
					loader: 'css'
				}
			})

			build.onLoad({ filter: /\.vue$/ }, async (args) => {
				const source = files[args.path]

				const { descriptor } = parse(source, { filename: args.path })
				const id = args.path.split('/').pop()
				const scopeId = `data-v-${hashString(args.path)}`
				const hasScopedStyles = descriptor.styles.some((s) => s.scoped)

				let scriptContent = ''
				let renderCode = ''
				let cssImports = []

				// Handle script setup
				if (descriptor.scriptSetup) {
					// First compile template separately to ensure proper scoping
					let templateResult
					if (descriptor.template) {
						templateResult = compileTemplate({
							source: descriptor.template.content,
							filename: args.path,
							id,
							scoped: hasScopedStyles,
							scopeId: hasScopedStyles ? scopeId : null,
							preprocessOptions: {},
							compilerOptions: {
								mode: 'module'
							}
						})
					}

					// Then compile script with the template
					const compiledScript = compileScript(descriptor, {
						id,
						templateOptions: {
							scoped: hasScopedStyles,
							scopeId: hasScopedStyles ? scopeId : null
						},
						inlineTemplate: true
					})

					scriptContent = compiledScript.content

					if (hasScopedStyles) {
						// Ensure the compiled component gets the scopeId
						const lastExportIndex = scriptContent.lastIndexOf('export default')
						if (lastExportIndex !== -1) {
							const lastBraceIndex = scriptContent.lastIndexOf('}')
							if (lastBraceIndex !== -1) {
								scriptContent =
									scriptContent.slice(0, lastBraceIndex) +
									`,\n  __scopeId: "${scopeId}"\n` +
									scriptContent.slice(lastBraceIndex)
							}
						}
					}
				}
				// Handle regular script
				else if (descriptor.script) {
					scriptContent = `const _sfc_main = ${descriptor.script.content.replace(
						'export default',
						''
					)}`
				} else {
					scriptContent = 'const _sfc_main = {}'
				}

				// Compile template if not using script setup
				if (descriptor.template && !descriptor.scriptSetup) {
					const templateResult = compileTemplate({
						source: descriptor.template.content,
						filename: args.path,
						id,
						scoped: hasScopedStyles,
						scopeId: hasScopedStyles ? scopeId : null,
						preprocessOptions: {},
						compilerOptions: {
							mode: 'module'
						}
					})
					renderCode = templateResult.code
				}

				// Compile styles
				if (descriptor.styles.length > 0) {
					for (let i = 0; i < descriptor.styles.length; i++) {
						const style = descriptor.styles[i]
						const styleResult = await compileStyle({
							source: style.content,
							filename: args.path,
							id: scopeId,
							scoped: style.scoped,
							modules: !!style.module,
							preprocessLang: style.lang,
							modulesName: style.module ? `style${i}` : undefined
						})

						const virtualStylePath = `${args.path}.${i}.css`
						styleMap.set(virtualStylePath, styleResult.code)
						// @ts-ignore
						cssImports.push(`import "${virtualStylePath}"`)
					}
				}

				// For non-setup scripts, add render and scopeId, then export
				let exports = ''
				if (!descriptor.scriptSetup) {
					exports = `
              ${renderCode ? '_sfc_main.render = render;' : ''}
              ${hasScopedStyles ? `_sfc_main.__scopeId = "${scopeId}";` : ''}
              export default _sfc_main;
            `
				}

				// Combine everything into a proper Vue component
				const contents = `
            ${cssImports.join('\n')}
            
            ${scriptContent}
            
            ${renderCode}
            
            ${exports}
          `.trim()

				return {
					contents,
					loader: 'js'
				}
			})
		}
	}
}

function hashString(str: string): string {
	let hash = 0
	if (str.length === 0) return hash.toString(36)

	for (let i = 0; i < str.length; i++) {
		const char = str.charCodeAt(i)
		hash = (hash << 5) - hash + char
		hash = hash & hash
	}

	return hash.toString(36).replace('-', '_')
}

async function sveltePlugin(files: Record<string, string>, version) {
	if (version != '5.16.1') {
		throw new Error(`Svelte version ${version} is not supported, use 5.16.1`)
	}
	// @ts-ignore
	await import(`https://unpkg.com/svelte@5.16.1/compiler/index.js`)
	// let styleCache = new Map()
	return {
		name: 'svelte',
		setup(build) {
			build.onLoad({ filter: /\.svelte$/ }, async (args) => {
				let convertMessage = ({ message, start, end }) => {
					let location
					if (start && end) {
						let lineText = source.split(/\r\n|\r|\n/g)[start.line - 1]
						let lineEnd = start.line === end.line ? end.column : lineText.length
						location = {
							file: filename,
							line: start.line,
							column: start.column,
							length: lineEnd - start.column,
							lineText
						}
					}
					return { text: message, location }
				}
				// Load the file from the file system
				let source = files[args.path]
				let filename = args.path

				try {
					let { js, warnings } = globalThis.svelte.compile(source, {
						filename,
						dev: true,
						css: 'injected'
					})

					// // Store the CSS in our cache with the file path as the key
					// if (css && css.code) {
					// 	styleCache.set(args.path, css.code)
					// }

					let contents =
						js.code +
						`\n//# sourceMappingURL=` +
						'data:application/json;charset=utf-8;base64,' +
						btoa(JSON.stringify(js.map))

					return {
						contents: contents,
						warnings: warnings.map((w) =>
							convertMessage({ message: w.message, start: w.start, end: w.end })
						)
					}
				} catch (e) {
					return { errors: [convertMessage(e)] }
				}
			})
		}
	}
}

self.onmessage = async (event) => {
	const { id, type, data } = event.data
	if (type === 'init') {
		if (!isInitialized) {
			await esbuild.initialize({
				wasmURL: 'https://unpkg.com/esbuild-wasm/esbuild.wasm',
				worker: false
			})
			isInitialized = true
		}
		postMessage({ id, success: true })
	} else if (type === 'build') {
		try {
			resetFileSystem(data.files)
			let pkg = data.files['/package.json'] ? JSON.parse(data.files['/package.json']) : {}
			let svelteVersion = pkg?.dependencies?.svelte
			let vueVersion = pkg?.dependencies?.vue

			let plugins: any[] = [
				// externalGlobalPlugin({
				// 	react: 'window.React',
				// 	'react-dom': 'window.ReactDOM'
				// })
			]
			if (svelteVersion) {
				plugins = [await sveltePlugin(data.files, svelteVersion)]
			}
			if (vueVersion) {
				plugins = [await vuePlugin(data.files, vueVersion)]
			}
			let entryPoints = data.files['/index.tsx']
				? ['/index.tsx']
				: data.files['/index.js']
				? ['/index.js']
				: ['/index.ts']
			const result = await esbuild.build({
				entryPoints: entryPoints, // required
				// resolveExtensions: resolveExtensions, // Ensure it resolves extensions automatically
				resolveExtensions: ['.js', '.jsx', '.ts', '.tsx'], // Ensure it resolves extensions automatically
				outdir: '/out', // required
				sourcemap: 'inline',
				plugins,
				// plugins: [

				// ],
				// external: ['react', 'react-dom'],
				// external: ['react', 'react-dom', 'react-dom/client'],
				minify: false, // required
				// required
				bundle: true,
				write: false
			})
			const contents = Object.fromEntries(result.outputFiles.map((file) => [file.path, file.text]))
			let output = {
				css: contents?.['/out/index.css'],
				js: contents?.['/out/index.js']
			}
			postMessage({ id, success: true, result: output })
		} catch (error) {
			postMessage({ id, success: false, error: error.message })
		}
	} else if (type == 'stop') {
		esbuild.stop()
	}
}
