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
	cli_command: string[],
	// Set unless the server was told to build with a specific command: the images
	// install the CLI, and using the one that is there needs no npm reachability
	// at deploy time and is the release this server ships with.
	prefer_installed_cli: boolean | undefined
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

	// However the caller spelled the key: `write()` normalises it, so the check
	// has to as well or the app builds with no node_modules.
	const hasPackageJson = Object.keys(files ?? {}).some(
		(p) => p.replace(/^[./]+/, '') === 'package.json'
	)
	// Piped rather than inherited so the output can go in the error too, then
	// echoed either way — the job's log is where someone looks to see what the
	// build did.
	const run = (argv: string[], what: string) => {
		const proc = Bun.spawnSync(argv, { cwd: dir, stdout: 'pipe', stderr: 'pipe' })
		const output = proc.stdout.toString() + proc.stderr.toString()
		console.log(output)
		if (proc.exitCode !== 0) {
			throw new Error(`${what} failed:\n${output}`)
		}
		return output
	}

	if (hasPackageJson) {
		// Installed here rather than left to the CLI so it can be --ignore-scripts:
		// the app's dependencies are compiled, never run, so a package's lifecycle
		// script has no business executing on the worker. The CLI skips its own
		// install once node_modules exists.
		run(['bun', 'install', '--ignore-scripts'], 'bun install')
	} else {
		// The CLI installs when node_modules is missing, and it shells out to npm,
		// which the slim images don't ship. An app with no manifest has nothing to
		// install, so hand it the empty directory it would have produced.
		await fs.mkdir(path.join(dir, 'node_modules'), { recursive: true })
	}

	const installed = prefer_installed_cli ? Bun.which('wmill') : undefined
	const build = installed ? [installed, 'app', 'bundle'] : cli_command

	const outDir = path.join(dir, 'dist')
	const buildOutput = run([...build, dir, '--out', outDir], 'bundle')

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
		throw new Error('bundle produced no javascript:\n' + buildOutput)
	}

	// Gzipped so a large app's bundle stays well inside MAX_RESULT_SIZE_MB, which
	// a deployment can set far below the 500MB default.
	const gz = (s: string) => Buffer.from(Bun.gzipSync(Buffer.from(s, 'utf8'))).toString('base64')
	return { js_gz: gz(js), css_gz: gz(css) }
}
