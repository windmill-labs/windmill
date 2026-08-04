/**
 * Bundles a raw app's sources into the js/css a deployed raw app serves. Runs as
 * a bun job so the compile happens on a worker, not in the API process — see
 * `apps_raw_bundle.rs`, which is the only thing that runs it.
 *
 * Mirrors the two bundlers that already exist (the editor's, in the ui_builder
 * iframe, and the CLI's `createBundle`): same entry-point pick, the same virtual
 * `wmill` module, `/ui/` resolved against the workspace's shared UI, and
 * NODE_ENV pinned to production — without which React ships its dev build and
 * the bundle doubles in size.
 */
export async function main(
	files: Record<string, string>,
	wmill_ts: string,
	shared_ui: Record<string, string> | undefined
): Promise<{ js: string; css: string }> {
	const fs = await import('node:fs/promises')
	const path = await import('node:path')

	const dir = path.join(process.cwd(), 'wm_raw_app')
	await fs.rm(dir, { recursive: true, force: true })

	const write = async (rel: string, content: string) => {
		const abs = path.join(dir, rel.replace(/^\/+/, ''))
		if (!abs.startsWith(dir + path.sep)) {
			throw new Error(`file path escapes the build directory: ${rel}`)
		}
		await fs.mkdir(path.dirname(abs), { recursive: true })
		await fs.writeFile(abs, content)
	}
	for (const [p, content] of Object.entries(files ?? {})) {
		await write(p, content)
	}
	for (const [p, content] of Object.entries(shared_ui ?? {})) {
		await write('ui/' + p.replace(/^\/+/, ''), content)
	}

	if (files['/package.json']) {
		// --ignore-scripts: the app's dependencies are compiled, never run, so a
		// package's lifecycle script has no business executing on the worker.
		const install = Bun.spawnSync(['bun', 'install', '--ignore-scripts'], {
			cwd: dir,
			stdout: 'pipe',
			stderr: 'pipe'
		})
		if (install.exitCode !== 0) {
			throw new Error(
				'bun install failed:\n' + install.stderr.toString() + install.stdout.toString()
			)
		}
	}

	const entry = ['/index.tsx', '/index.ts', '/index.js'].find((e) => files?.[e])
	if (!entry) {
		throw new Error('no entry point: the app needs one of /index.tsx, /index.ts, /index.js')
	}

	const wmillPlugin = {
		name: 'wmill-virtual',
		setup(build: any) {
			build.onResolve({ filter: /^(\.\.\/)+wmill(\.ts)?$|^(\.\/|\/)?wmill(\.ts)?$/ }, () => ({
				path: 'wmill-virtual',
				namespace: 'wmill-virtual'
			}))
			build.onLoad({ filter: /.*/, namespace: 'wmill-virtual' }, () => ({
				contents: wmill_ts,
				loader: 'ts'
			}))
		}
	}

	const result = await Bun.build({
		entrypoints: [path.join(dir, entry.replace(/^\/+/, ''))],
		outdir: path.join(dir, 'dist'),
		target: 'browser',
		minify: true,
		define: { 'process.env.NODE_ENV': '"production"' },
		plugins: [wmillPlugin]
	})
	if (!result.success) {
		throw new Error('bundle failed:\n' + (result.logs ?? []).map((l: any) => String(l)).join('\n'))
	}

	let js = ''
	let css = ''
	for (const out of result.outputs) {
		const text = await out.text()
		if (out.path.endsWith('.css')) css += text
		else if (out.path.endsWith('.js')) js += text
	}
	if (js === '') {
		throw new Error('bundle produced no javascript')
	}
	return { js, css }
}
