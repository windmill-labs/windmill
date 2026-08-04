/**
 * Bundles a raw app's sources into the js/css a deployed raw app serves. Runs as
 * a bun job so the compile happens on a worker, not in the API process — see
 * `apps_raw_bundle.rs`, which is the only thing that runs it.
 *
 * The build itself is `wmill app bundle`, the same one `wmill app push` runs, so
 * an app deployed through the API is compiled exactly as the CLI and the editor
 * compile it — entry point, virtual `wmill` module, `/ui/` shared-UI resolution
 * and the Svelte/Vue plugins included. Reimplementing any of that here would be
 * a third bundler to keep in step with the other two.
 */
export async function main(
	files: Record<string, string>,
	shared_ui: Record<string, string> | undefined,
	cli_command: string[]
): Promise<{ js_gz: string; css_gz: string }> {
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
	// `ui/` next to the app is where `wmill app bundle` looks for the shared UI.
	for (const [p, content] of Object.entries(shared_ui ?? {})) {
		await write('ui/' + p.replace(/^\/+/, ''), content)
	}

	if (files['/package.json']) {
		// Installed here rather than left to the CLI so it can be --ignore-scripts:
		// the app's dependencies are compiled, never run, so a package's lifecycle
		// script has no business executing on the worker. The CLI skips its own
		// install once node_modules exists.
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

	const outDir = path.join(dir, 'dist')
	const build = Bun.spawnSync([...cli_command, dir, '--out', outDir], {
		cwd: dir,
		stdout: 'pipe',
		stderr: 'pipe'
	})
	if (build.exitCode !== 0) {
		throw new Error(
			'bundle failed:\n' + build.stdout.toString() + build.stderr.toString()
		)
	}

	const read = async (name: string) => {
		try {
			return await fs.readFile(path.join(outDir, name), 'utf8')
		} catch {
			return ''
		}
	}
	const js = await read('bundle.js')
	const css = await read('bundle.css')
	if (js === '') {
		throw new Error('bundle produced no javascript:\n' + build.stdout.toString())
	}

	// Gzipped so a large app's bundle stays well inside MAX_RESULT_SIZE_MB, which
	// a deployment can set far below the 500MB default.
	const gz = (s: string) => Buffer.from(Bun.gzipSync(Buffer.from(s, 'utf8'))).toString('base64')
	return { js_gz: gz(js), css_gz: gz(css) }
}
