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
declare const __wmillAppPolicy: {
	updateRawAppPolicy: (
		runnables: Record<string, unknown>,
		current: undefined
	) => Promise<{ triggerables_v2: Record<string, unknown> }>
}

export async function main(
	files: Record<string, string>,
	shared_ui: Record<string, string> | undefined,
	cli_command: string[],
	// Set unless the server was told to build with a specific command.
	prefer_installed_cli: boolean | undefined,
	// The app's `value.runnables`, whose policy is derived here for the same
	// reason the bundle is built here: it has to match what the editor writes.
	runnables: Record<string, unknown> | undefined
): Promise<{ js_gz: string; css_gz: string; triggerables_v2: Record<string, unknown> }> {
	const fs = await import('node:fs/promises')
	const path = await import('node:path')

	const dir = path.join(process.cwd(), 'wm_raw_app')
	await fs.rm(dir, { recursive: true, force: true })

	// Where a key lands, `path.join` normalising `./` and `..` away. Everything
	// that reasons about a file goes through this, so nothing disagrees with what
	// was actually written.
	const target = (rel: string) => {
		const abs = path.join(dir, rel.replace(/^\/+/, ''))
		if (!abs.startsWith(dir + path.sep)) {
			throw new Error(`file path escapes the build directory: ${rel}`)
		}
		return abs
	}
	const write = async (rel: string, content: string) => {
		const abs = target(rel)
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

	const manifest = path.join(dir, 'package.json')
	const hasPackageJson = Object.keys(files ?? {}).some((p) => target(p) === manifest)
	// Piped rather than inherited so the output can go in the error too, then
	// echoed either way — the job's log is where someone looks to see what the
	// build did.
	const spawn = (argv: string[]) => {
		const proc = Bun.spawnSync(argv, { cwd: dir, stdout: 'pipe', stderr: 'pipe' })
		const output = proc.stdout.toString() + proc.stderr.toString()
		console.log(output)
		return { ok: proc.exitCode === 0, output }
	}
	const run = (argv: string[], what: string) => {
		const { ok, output } = spawn(argv)
		if (!ok) {
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

	const outDir = path.join(dir, 'dist')
	// Prefer the CLI the image installed: no npm reachability needed at deploy
	// time. It can predate `app bundle` (the images install it unpinned), and a
	// CLI without the command exits with cliffy's usage text before building
	// anything — so that specific failure, and only it, falls back to fetching
	// the one for this server's release. `wmill --version` isn't used to decide:
	// it reports npm's latest release as well as its own, and it reaches out to
	// npm to do so, which is the cost this branch exists to avoid.
	const installed = prefer_installed_cli ? Bun.which('wmill') : null
	let buildOutput: string | undefined
	if (installed) {
		const attempt = spawn([installed, 'app', 'bundle', dir, '--out', outDir])
		// Colours sit between the words cliffy prints, so match on the stripped text.
		const plain = attempt.output.replace(/\x1b\[[0-9;]*m/g, '')
		if (attempt.ok) {
			buildOutput = attempt.output
		} else if (!/Unknown command|Usage:\s+wmill app\b/.test(plain)) {
			throw new Error(`bundle failed:\n${attempt.output}`)
		}
	}
	if (buildOutput === undefined) {
		buildOutput = run([...cli_command, dir, '--out', outDir], 'bundle')
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
		throw new Error('bundle produced no javascript:\n' + buildOutput)
	}

	// A runnable the derivation can't fully classify still yields a key, just an
	// unusable one (`r:undefined/undefined`, or the hash of an absent script), and
	// the deploy would then succeed with grants no run can ever match. The tool
	// schema describes `runnables` only as an object and the on-disk format has no
	// discriminator at all (`wmill app push` adds it), so these shapes are all
	// reachable: check them before deriving and name the ones at fault. An
	// explicitly empty entry is a runnable nobody configured yet, and needs no
	// grant.
	const nonEmpty = (v: unknown) => typeof v === 'string' && v.length > 0
	// The prefixes `execute_component` resolves a run against; anything else is a
	// grant no run can match.
	const RUN_TYPES = ['script', 'flow', 'hubscript']
	const malformed = Object.entries(runnables ?? {})
		.filter(([, r]) => r != null)
		.filter(([, r]) => {
			if (typeof r !== 'object') return true
			const run = r as Record<string, any>
			if (run.type === 'inline' || run.type === 'runnableByName') {
				return !nonEmpty(run.inlineScript?.content)
			}
			if (run.type === 'path' || run.type === 'runnableByPath') {
				return !RUN_TYPES.includes(run.runType) || !nonEmpty(run.path)
			}
			return true
		})
		.map(([id]) => id)
	if (malformed.length > 0) {
		throw new Error(
			`no policy could be derived for runnable(s) ${malformed.join(', ')}: each must be an ` +
				`object with a \`type\` of "inline" (with \`inlineScript.content\`) or "path" (with ` +
				`\`path\` and a \`runType\` of ${RUN_TYPES.join(', ')})`
		)
	}

	// The policy's `triggerables_v2` is the allowlist the server matches every run
	// against, keyed by a hash of each inline runnable's code. Derived by the
	// frontend's own code, bundled into this script by cli/generate-app-policy.ts,
	// so the keys are the ones the app editor writes: anything else leaves the
	// app's runnables "forbidden by policy". Prepended above as a plain `var`, so
	// it is in this module's scope (a module's top-level `var` is not a global).
	const { triggerables_v2 } = await __wmillAppPolicy.updateRawAppPolicy(
		runnables ?? {},
		undefined
	)

	// Gzipped so a large app's bundle stays well inside MAX_RESULT_SIZE_MB, which
	// a deployment can set far below the 500MB default.
	const gz = (s: string) => Buffer.from(Bun.gzipSync(Buffer.from(s, 'utf8'))).toString('base64')
	return { js_gz: gz(js), css_gz: gz(css), triggerables_v2 }
}
