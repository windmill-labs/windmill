/**
 * Generates what the Windmill MCP server's toolset (backend/windmill-api/src/ai_session_tools)
 * shows a model, from the same definitions an AI session uses: tool names, descriptions and
 * input schemas, the system prompt, and the `get_instructions` texts. The server implements
 * the tools in Rust; everything the model reads comes from here, so the two cannot drift
 * apart in wording.
 *
 * Run `npm run gen:mcp-session` after changing a GLOBAL tool definition, the global system
 * prompt, or anything under system_prompts/. Without that env var this is the freshness check.
 */
import { describe, expect, it } from 'vitest'
import { existsSync, mkdirSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { join } from 'node:path'
import { createHash } from 'node:crypto'
import {
	ALL_INSTRUCTION_SUBJECTS,
	buildFolderGuidance,
	EXTERNAL_PROMPT,
	externalPromptProfile,
	getInstructions,
	globalToolsFor,
	prepareGlobalSystemMessage
} from './core'
import { renderPromptTemplate } from './promptTemplate'
import type { AuthorLevel, ScriptLang } from '$lib/gen'
import { FULL_CAPABILITIES } from '$lib/roleCapabilities'
import { capabilitiesForRole } from './sessionAccess'
import { USE_MODE_ACCESS } from '../sessionCapabilities'
import {
	BUILDER_FRAMEWORK_TEMPLATES,
	FRAMEWORK_TEMPLATES,
	STARTER_RUNNABLES
} from '$lib/components/raw_apps/templates'
import { RESOURCE_TYPE_WORDS } from '$lib/components/resourceTypeDisplay'

const OUT_DIR = fileURLToPath(
	new URL('../../../../../../../backend/windmill-api/src/ai_session_tools/gen/', import.meta.url)
)

// Sentences that describe the chat's run form, which an MCP client does not have. Each
// must still be found in the description it is listed for: a reworded description fails
// generation rather than shipping a promise of a form.
const DESCRIPTION_REWRITES: [tool: string, from: string, to: string][] = [
	[
		'test_run_script',
		' The user gets an argument form prefilled with `args` and may edit or dismiss it before it runs, so fill in every argument you can infer.',
		' It runs with exactly the `args` you send.'
	],
	[
		'test_run_flow',
		' The user gets an argument form prefilled with `args` and may edit or dismiss it before it runs, so fill in every argument you can infer.',
		' It runs with exactly the `args` you send.'
	],
	[
		'test_run_step',
		' The user gets an argument form prefilled with `args` and may edit or dismiss it before it runs, so fill in every argument you can infer.',
		' It runs with exactly the `args` you send.'
	],
	[
		'run_script',
		" Fill in every argument you can infer: the user gets an argument form prefilled with `args` and decides what runs. In an AI session the user can open that form on the script's own deployed page instead; either way this call waits for them and runs what they confirm.",
		' It runs with exactly the `args` you send.'
	],
	[
		'run_flow',
		" Fill in every argument you can infer: the user gets an argument form prefilled with `args` and decides what runs. In an AI session the user can open that form on the flow's own deployed page instead; either way this call waits for them and runs what they confirm.",
		' It runs with exactly the `args` you send.'
	],
	[
		'test_run_agent',
		' The user gets an argument form prefilled with `args` and may edit or dismiss it before it runs, so fill in every argument you can infer.',
		' It runs with exactly the `args` you send.'
	],
	[
		'run_agent',
		' Fill in every argument you can infer: the user gets an argument form prefilled with `args` and decides what runs.',
		' It runs with exactly the `args` you send.'
	],
	['create_resource_type', ' once the user confirms the schema', ''],
	['set_workspace_settings', ' The user reviews the changes before they apply.', '']
]

// The chat follows a backgrounded job in its jobs tray and tells the model when it ends; an
// MCP client gets the job id back and follows it with get_run. Applied to every tool's
// parameter of that name, and each must match at least once.
const PARAM_REWRITES: [param: string, from: string, to: string][] = [
	[
		'background',
		' — you will be notified when it finishes.',
		': the result gives its job id, to follow with get_run.'
	],
	['wait_seconds', 'detaches into the background jobs tray', 'keeps running in the background']
]

// The step an MCP client should take next, said where the model decides on it: ChatGPT gives
// the server instructions past their first lines little weight. The show tools exist only
// for a client that draws cards, hence "if you have it".
const GET_INSTRUCTIONS_FIRST = ' Call get_instructions first.'
const SHOW_USER_RUN =
	' When the user asked for this run, show it to them with show_run, if you have it.'
const SHOW_DEPLOY_REVIEW =
	' Once the draft works and the user wants to review it, show it with show_deploy_review, if you have it.'
const DESCRIPTION_APPENDS: [tool: string, text: string][] = [
	['write_script', GET_INSTRUCTIONS_FIRST],
	['write_flow', GET_INSTRUCTIONS_FIRST],
	['write_resource', GET_INSTRUCTIONS_FIRST],
	['init_app', GET_INSTRUCTIONS_FIRST],
	['run_script', ' Then show the run to the user with show_run, if you have it.'],
	['run_flow', ' Then show the run to the user with show_run, if you have it.'],
	['run_agent', ' Then show the run to the user with show_run, if you have it.'],
	['test_run_script', SHOW_USER_RUN + SHOW_DEPLOY_REVIEW],
	['test_run_flow', SHOW_USER_RUN + SHOW_DEPLOY_REVIEW],
	['test_run_agent', SHOW_USER_RUN],
	['list_runs', ' To show runs to the user, pass their ids to show_runs, if you have it.']
]

// A rewrite only proves its old sentence is still there; this catches one added next to it.
const CHAT_ONLY_PROMISE = /argument form|the user (confirms|reviews|approves)/i

function toolDefs() {
	const unmatched = new Set(PARAM_REWRITES)
	const unappended = new Set(DESCRIPTION_APPENDS.map(([name]) => name))
	const defs = globalToolsFor({ sessionPreview: false }).map((tool) => {
		const fn = tool.def.function
		let description = fn.description ?? ''
		for (const [name, text] of DESCRIPTION_APPENDS) {
			if (name !== fn.name) continue
			description += text
			unappended.delete(name)
		}
		for (const [name, from, to] of DESCRIPTION_REWRITES) {
			if (name !== fn.name) continue
			if (!description.includes(from)) {
				throw new Error(`${name}: its description no longer contains "${from}"`)
			}
			description = description.replace(from, to)
		}
		if (CHAT_ONLY_PROMISE.test(description)) {
			throw new Error(`${fn.name}: its description promises a chat-only confirmation`)
		}
		const parameters = structuredClone(fn.parameters) as {
			properties?: Record<string, { description?: string }>
		}
		for (const rewrite of PARAM_REWRITES) {
			const [param, from, to] = rewrite
			const prop = parameters.properties?.[param]
			if (!prop?.description?.includes(from)) continue
			prop.description = prop.description.replace(from, to)
			unmatched.delete(rewrite)
		}
		return {
			name: fn.name,
			description,
			parameters,
			// The session's own read-only marker: what plan mode lets through.
			read_only: tool.planModeSafe === true
		}
	})
	for (const [param, from] of unmatched) {
		throw new Error(`no ${param} parameter contains "${from}" any more`)
	}
	for (const name of unappended) {
		throw new Error(`${name} is no longer a session tool`)
	}
	return defs
}

const WRITABLE = '{{writable_folders}}'
const READONLY = '{{readonly_folders}}'

type PromptOpts = Parameters<typeof prepareGlobalSystemMessage>[1]

function prompt(
	user: { is_admin: boolean; folders?: string[]; folders_read?: string[] },
	extra: PromptOpts = {},
	instructions?: { workspace?: string }
): string {
	const content = prepareGlobalSystemMessage(instructions, {
		user: { username: '{{username}}', ...user } as any,
		external: true,
		...extra
	}).content as string
	// The builder wraps each folder as `f/<name>`; the server substitutes a whole list.
	return content
		.replaceAll('`f/' + WRITABLE + '`', WRITABLE)
		.replaceAll('`f/' + READONLY + '`', READONLY)
}

function suffixOf(longer: string, base: string, what: string): string {
	if (!longer.startsWith(base)) throw new Error(`${what} is no longer appended to the prompt`)
	return longer.slice(base.length)
}

/** The `{{folder_guidance}}` the server puts in the template, once per folder situation, as
 * `buildGlobalSystemPrompt` inserts it: with its leading newline. */
function folderGuidance(canCreateFolder: boolean) {
	const variant = (isAdmin: boolean, folders: string[], foldersRead = folders) => {
		const text = buildFolderGuidance(
			'{{username}}',
			{ isAdmin, folders, foldersRead },
			canCreateFolder,
			true
		)
		return (text ? `\n${text}` : '')
			.replaceAll('`f/' + WRITABLE + '`', WRITABLE)
			.replaceAll('`f/' + READONLY + '`', READONLY)
	}
	return {
		admin: variant(true, [WRITABLE]),
		admin_no_folders: variant(true, []),
		member: variant(false, [WRITABLE]),
		member_no_folders: variant(false, []),
		member_readonly: variant(false, [WRITABLE], [WRITABLE, READONLY]),
		member_no_folders_readonly: variant(false, [], [READONLY])
	}
}

const LEVELS: AuthorLevel[] = ['none', 'compose', 'full']
const bit = (b: boolean) => (b ? '1' : '0')

/** Every access profile the server can resolve a caller to, keyed as `prompt_profile` in
 * `ai_session_tools/instructions.rs` builds the key: the token's scope, then for a full token
 * the caller's role as `caller_capabilities` and `check_deploy_rules` give it. */
function profiles(): [key: string, extra: PromptOpts][] {
	const out: [string, PromptOpts][] = [
		['read_only', { access: USE_MODE_ACCESS, runsDeployed: false }],
		['read_run', { access: USE_MODE_ACCESS, runsDeployed: true }]
	]
	for (const isAdmin of [true, false])
		for (const deploy of [true, false])
			for (const code of [true, false])
				for (const flows of LEVELS)
					for (const apps of LEVELS)
						for (const manage_schedules of [true, false])
							for (const manage_triggers of [true, false]) {
								const caps = {
									...FULL_CAPABILITIES,
									code,
									flows,
									apps,
									manage_schedules,
									manage_triggers
								}
								// Full flows or apps without code is refused at write.
								if (!code && (flows === 'full' || apps === 'full')) continue
								// An admin holds every capability, so only the full role is reachable.
								if (isAdmin && JSON.stringify(caps) !== JSON.stringify(FULL_CAPABILITIES)) continue
								const key = `full:${bit(isAdmin)}${bit(deploy)}:${bit(code)}:${flows}:${apps}:${bit(manage_schedules)}${bit(manage_triggers)}`
								const access = capabilitiesForRole({
									isAdmin,
									deployRulesPass: deploy,
									capabilities: caps
								})
								out.push([key, { access }])
							}
	return out
}

function promptParts() {
	const member = prompt({ is_admin: false, folders: [WRITABLE] })
	const skills = suffixOf(
		prompt(
			{ is_admin: false, folders: [WRITABLE] },
			{ skills: [{ path: '{{skill_path}}', name: '', description: '{{skill_description}}' }] }
		),
		member,
		'the skills section'
	)
	const skillLine = skills.split('\n').pop()!
	const index: Record<string, { flags: string; vars: Record<string, string> }> = {}
	// What each profile renders to before the server fills the runtime variables, so a Rust
	// test can check its renderer against this one.
	const renderedSha256: Record<string, string> = {}
	for (const [key, extra] of profiles()) {
		const { flags, vars } = externalPromptProfile(extra?.access, extra?.runsDeployed ?? true)
		index[key] = { flags: [...flags].sort().join(' '), vars }
		renderedSha256[key] = createHash('sha256')
			.update(renderPromptTemplate(EXTERNAL_PROMPT, flags, vars))
			.digest('hex')
	}
	return {
		template: EXTERNAL_PROMPT,
		profiles: index,
		folder_guidance: {
			create_folder: folderGuidance(true),
			no_create_folder: folderGuidance(false)
		},
		rendered_sha256: renderedSha256,
		skills_header: skills.slice(0, skills.length - skillLine.length),
		skill_line: skillLine,
		workspace_instructions: suffixOf(
			prompt(
				{ is_admin: false, folders: [WRITABLE] },
				{},
				{ workspace: '{{workspace_instructions}}' }
			),
			member,
			'the workspace instructions'
		)
	}
}

/** What `get_trigger_schema` and `get_schedule_schema` answer: derived from the generated
 * request schemas, so the tools themselves are the only faithful source. */
async function schemas() {
	const tools = globalToolsFor({ sessionPreview: false })
	const tool = (name: string) => tools.find((t) => t.def.function.name === name)!
	const run = (name: string, args: unknown) =>
		tool(name).fn({
			args,
			workspace: '',
			toolId: '',
			toolCallbacks: {} as any,
			helpers: {} as any
		})
	const kinds: string[] = (tool('get_trigger_schema').def.function.parameters as any).properties
		.kind.enum
	const trigger: Record<string, string> = {}
	for (const kind of kinds) {
		trigger[kind] = await run('get_trigger_schema', { kind })
	}
	return { trigger, schedule: await run('get_schedule_schema', {}) }
}

/** Every file to write, keyed by name. */
async function generate(): Promise<Record<string, string>> {
	const tools = toolDefs()
	const files: Record<string, string> = {
		'tools.json': JSON.stringify(tools, null, '\t') + '\n',
		'prompt.json': JSON.stringify(promptParts(), null, '\t') + '\n',
		// How a slug's words read, for matching a query against a hub integration the hub
		// gives no display name.
		'display_words.json': JSON.stringify(RESOURCE_TYPE_WORDS, null, '\t') + '\n'
	}
	const languages: (ScriptLang | undefined)[] = [
		undefined,
		...(tools.find((t) => t.name === 'get_instructions')!.parameters as any).properties.language
			.enum
	]
	// Keyed by subject then language ('' when none was given); identical texts are stored once.
	const index: Record<string, Record<string, string>> = {}
	const texts: Record<string, string> = {}
	for (const subject of ALL_INSTRUCTION_SUBJECTS) {
		index[subject] = {}
		const byText = new Map<string, string>()
		for (const language of languages) {
			const text = await getInstructions(subject, language, undefined, false, true)
			let id = byText.get(text)
			if (!id) {
				id = `${subject}${language ? '.' + language : ''}`
				byText.set(text, id)
				texts[id] = text
			}
			index[subject][language ?? ''] = id
		}
	}
	// What `init_app` seeds a new raw app with; an operator with builder rights starts from
	// a static page, since they can save no inline runnable.
	files['app_templates.json'] =
		JSON.stringify(
			{
				templates: FRAMEWORK_TEMPLATES,
				builder_templates: BUILDER_FRAMEWORK_TEMPLATES,
				starter_runnables: STARTER_RUNNABLES
			},
			null,
			'\t'
		) + '\n'
	files['schemas.json'] = JSON.stringify(await schemas(), null, '\t') + '\n'
	files['instructions.json'] = JSON.stringify({ index, texts }, null, '\t') + '\n'
	return files
}

function committedFiles(dir = OUT_DIR, prefix = ''): string[] {
	if (!existsSync(dir)) return []
	return readdirSync(dir, { withFileTypes: true }).flatMap((e) =>
		e.isDirectory() ? committedFiles(join(dir, e.name), `${prefix}${e.name}/`) : [prefix + e.name]
	)
}

describe('MCP session server generated definitions', () => {
	it('match the session tools and prompts (npm run gen:mcp-session)', async () => {
		const files = await generate()
		expect(Object.keys(files)).toContain('tools.json')
		if (process.env.WM_GEN_MCP_SESSION) {
			rmSync(OUT_DIR, { recursive: true, force: true })
			mkdirSync(OUT_DIR, { recursive: true })
			for (const [name, content] of Object.entries(files)) {
				writeFileSync(join(OUT_DIR, name), content)
			}
			return
		}
		expect(committedFiles().sort()).toEqual(Object.keys(files).sort())
		for (const [name, content] of Object.entries(files)) {
			expect(readFileSync(join(OUT_DIR, name), 'utf8'), name).toBe(content)
		}
	})

	// A capability that changes the prompt but is missing from the profile key would serve every
	// role the prompt of whatever value `profiles()` fixed it at.
	it('key every role capability that changes the prompt', () => {
		const keyed = ['code', 'flows', 'apps', 'manage_schedules', 'manage_triggers']
		const render = (capabilities: typeof FULL_CAPABILITIES) => {
			const access = capabilitiesForRole({ isAdmin: false, deployRulesPass: true, capabilities })
			const { flags, vars } = externalPromptProfile(access, true)
			return renderPromptTemplate(EXTERNAL_PROMPT, flags, vars)
		}
		const roles = [
			FULL_CAPABILITIES,
			{
				...FULL_CAPABILITIES,
				code: false,
				flows: 'compose' as AuthorLevel,
				apps: 'none' as AuthorLevel
			}
		]
		for (const role of roles)
			for (const [field, value] of Object.entries(role)) {
				if (keyed.includes(field)) continue
				const others =
					typeof value === 'boolean'
						? [!value]
						: typeof value === 'string'
							? LEVELS.filter((l) => l !== value)
							: [Object.fromEntries(Object.keys(value).map((k) => [k, false]))]
				for (const other of others) {
					expect(
						render({ ...role, [field]: other }),
						`role capability "${field}" changes the MCP prompt: add it to the profile key in profiles() and prompt_profile (instructions.rs)`
					).toBe(render(role))
				}
			}
	})
})
