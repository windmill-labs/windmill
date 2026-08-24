/**
 * The rules for the files of a dbt project, which is what a dbt script's module
 * bundle is.
 *
 * A `__mod` bundle is homogeneous — one language, one extension — and a dbt
 * project is not: models are SQL or Python, schemas and the project file YAML,
 * seeds CSV, doc blocks Markdown. Every module is stored with `language: dbt`
 * because they are dbt's to read, so everything below is decided by the file
 * name.
 */
import type { Preview, ScriptModule } from '$lib/gen'
import { canonicalModulePath, findModulePathClash } from '../scriptModulePath'
import YAML from 'yaml'

/** The descriptor. It is the script's CONTENT, not a module: a module at the
 *  same path would be a second, independent value for one file, since the export
 *  writes the content there and the bundle would emit over it. */
export const DBT_DESCRIPTOR = 'wm_dbt.yaml'

/** What makes the bundle a project. The worker refuses a version without it, so
 *  it is the one file the tree will not let you delete. */
export const DBT_PROJECT_FILE = 'dbt_project.yml'

/** The key a bundle actually holds `dbt_project.yml` under.
 *
 *  Not the constant: nothing on the push path rewrites existing keys, so a
 *  project imported with `./dbt_project.yml` in it holds that spelling, and an
 *  exact lookup would neither find the project nor protect it from deletion —
 *  while every write path canonicalises. Resolved the same way the duplicate
 *  check resolves a clash. */
export function dbtProjectFileKey(
	modules: Record<string, unknown> | null | undefined
): string | undefined {
	return findModulePathClash(modules, DBT_PROJECT_FILE)
}

/**
 * Extensions a dbt project's own files take. `.py` because dbt Python models are
 * first-class on Snowflake, BigQuery and Databricks.
 */
export const DBT_MODULE_EXTENSIONS = ['.sql', '.py', '.yml', '.yaml', '.csv', '.md']

/**
 * The editor language for one project file, by extension. `postgresql` and
 * `ansible` are how Windmill spells "SQL" and "YAML" to the editor.
 */
export function dbtFileLang(path: string): Preview['language'] {
	if (path.endsWith('.sql')) return 'postgresql'
	if (path.endsWith('.yml') || path.endsWith('.yaml')) return 'ansible'
	if (path.endsWith('.py')) return 'python3'
	// `.md` (a doc block) and `.csv` (a seed) have no grammar of their own here;
	// `bash` leaves prose alone, where the default would colour it as TypeScript.
	return 'bash'
}

/** Whether this file may be added to the bundle at all. */
export function dbtModuleLang(filePath: string): ScriptModule['language'] | undefined {
	return DBT_MODULE_EXTENSIONS.some((e) => filePath.endsWith(e))
		? ('dbt' as ScriptModule['language'])
		: undefined
}

/** The canonical key a typed path becomes, or the reason it cannot be one.
 *
 *  Canonicalised before the reserved-name and duplicate checks, not after: the
 *  worker resolves `.` and `//` away when it materialises the bundle, so
 *  `./dbt_project.yml` and `dbt_project.yml` are two keys for one file on disk
 *  and either check is trivially walked past by the redundant spelling. */
export function dbtModulePath(
	path: string,
	modules: Record<string, ScriptModule> | undefined
): { path: string } | { error: string } {
	const canonical = canonicalModulePath(path)
	if ('error' in canonical) return canonical
	if (canonical.path === DBT_DESCRIPTOR) {
		return {
			error: `${DBT_DESCRIPTOR} is the descriptor, edited from the tree — it cannot also be a file`
		}
	}
	if (!dbtModuleLang(canonical.path)) {
		return { error: `File must end with one of: ${DBT_MODULE_EXTENSIONS.join(', ')}` }
	}
	const clash = findModulePathClash(modules, canonical.path)
	if (clash) return { error: `${clash} already exists` }
	return canonical
}

/** Why this path cannot be a file, or `undefined` when it can. */
export function dbtPathError(
	path: string,
	modules: Record<string, ScriptModule> | undefined
): string | undefined {
	if (!path.trim()) return undefined
	const resolved = dbtModulePath(path, modules)
	return 'error' in resolved ? resolved.error : undefined
}

/** A new file's starting content. A model compiles on its own, so it is runnable
 *  before it is edited; anything else starts empty rather than with a guess at
 *  which dbt schema it is. */
export function dbtDefaultContent(filePath: string): string {
	return filePath.endsWith('.sql') ? 'select 1 as id\n' : ''
}

/**
 * What `dbt build --select` should be given for an open file, or `undefined`
 * when the file is not a model.
 *
 * Only files under the project's `model-paths` are models; a project also holds
 * macros, analyses and singular tests, all `.sql`, none of them selectable by
 * name. The selector is package-qualified because a bare leaf name also matches a
 * dependency package's model of the same name.
 */
export function dbtModelSelector(
	modules: Record<string, { content?: string }>,
	filePath: string
): string | undefined {
	// `.py` as well as `.sql`: a dbt Python model is a model, and leaving it out
	// would run the whole project to check one file — the larger warehouse bill
	// this narrowing exists to avoid.
	const ext = ['.sql', '.py'].find((e) => filePath.endsWith(e))
	if (!ext) return undefined
	const projectKey = dbtProjectFileKey(modules)
	let project: any
	try {
		project = YAML.parse((projectKey ? modules[projectKey] : undefined)?.content ?? '')
	} catch {
		return undefined
	}
	if (!project?.name) return undefined
	const modelPaths: string[] = Array.isArray(project['model-paths'])
		? project['model-paths']
		: ['models']
	if (!modelPaths.some((d) => filePath === d || filePath.startsWith(d + '/'))) return undefined
	const name = filePath.split('/').pop()!.slice(0, -ext.length)
	return `${name},package:${project.name}`
}

/** The descriptor's `engine` and `profile.warehouse`, for the editor's header.
 *  Best effort: a descriptor mid-edit is often not valid YAML, and a header that
 *  blanked on every keystroke would be worse than one that lags. */
export function dbtDescriptorSummary(content: string): {
	engine: string
	warehouse: string
} {
	try {
		const d = YAML.parse(content ?? '')
		return {
			engine: typeof d?.engine === 'string' ? d.engine : 'dbt-core-1x',
			// A descriptor naming its own `profiles_yml` and no warehouse has no
			// warehouse identity at all, which is what leaves it with no graph.
			warehouse:
				typeof d?.profile?.warehouse === 'string'
					? d.profile.warehouse
					: d?.profile?.profiles_yml
						? 'own profiles.yml'
						: 'main'
		}
	} catch {
		return { engine: 'dbt-core-1x', warehouse: 'main' }
	}
}
