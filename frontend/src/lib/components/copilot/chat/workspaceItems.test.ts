import { describe, expect, it, vi } from 'vitest'
import { unified } from 'unified'
import remarkParse from 'remark-parse'
import remarkRehype from 'remark-rehype'
import type { Root as MdastRoot, Link, Text } from 'mdast'

vi.mock('$lib/gen', () => ({
	ScriptService: { listScripts: vi.fn() },
	FlowService: { listFlows: vi.fn() },
	AppService: { listApps: vi.fn() },
	VariableService: { listVariable: vi.fn() },
	ResourceService: { listResource: vi.fn() },
	ScheduleService: { listSchedules: vi.fn() },
	HttpTriggerService: { listHttpTriggers: vi.fn() },
	WebsocketTriggerService: { listWebsocketTriggers: vi.fn() },
	KafkaTriggerService: { listKafkaTriggers: vi.fn() },
	NatsTriggerService: { listNatsTriggers: vi.fn() },
	PostgresTriggerService: { listPostgresTriggers: vi.fn() },
	MqttTriggerService: { listMqttTriggers: vi.fn() },
	SqsTriggerService: { listSqsTriggers: vi.fn() },
	GcpTriggerService: { listGcpTriggers: vi.fn() },
	AzureTriggerService: { listAzureTriggers: vi.fn() },
	EmailTriggerService: { listEmailTriggers: vi.fn() }
}))

import {
	extractCandidatePaths,
	itemHref,
	remarkWindmillPaths,
	WINDMILL_PATH_REGEX,
	workspaceItemAction,
	type WorkspaceItemEntry
} from './workspaceItems.svelte'

describe('WINDMILL_PATH_REGEX', () => {
	it('matches simple folder and user paths', () => {
		expect(extractCandidatePaths('Use f/marketing/send_email today')).toEqual([
			'f/marketing/send_email'
		])
		expect(extractCandidatePaths('Check u/admin/cleanup')).toEqual(['u/admin/cleanup'])
	})

	it('matches paths with sub-folders and dotted segments', () => {
		expect(extractCandidatePaths('Pipeline f/etl/jobs/ingest_users.v2 runs nightly')).toEqual([
			'f/etl/jobs/ingest_users.v2'
		])
	})

	it('matches usernames containing dots (e.g. firstname.lastname)', () => {
		expect(extractCandidatePaths('Owned by u/jane.doe/report')).toEqual(['u/jane.doe/report'])
	})

	it('strips trailing sentence punctuation', () => {
		expect(extractCandidatePaths('Look at f/foo/bar.')).toEqual(['f/foo/bar'])
		expect(extractCandidatePaths('Try f/foo/bar, please')).toEqual(['f/foo/bar'])
		expect(extractCandidatePaths('Is f/foo/bar?')).toEqual(['f/foo/bar'])
	})

	it('skips matches inside URLs', () => {
		expect(extractCandidatePaths('See https://example.com/u/me/script for context')).toEqual([])
	})

	it('does not match incomplete paths', () => {
		expect(extractCandidatePaths('I tried f/folder/ but nothing')).toEqual([])
		expect(extractCandidatePaths('Just u/me')).toEqual([])
	})

	it('returns multiple unique paths from one string', () => {
		const paths = extractCandidatePaths('Run f/a/one then f/b/two and again f/a/one')
		expect(paths.sort()).toEqual(['f/a/one', 'f/b/two'])
	})

	it('exposes a global regex', () => {
		expect(WINDMILL_PATH_REGEX.global).toBe(true)
	})
})

describe('itemHref', () => {
	it('routes script/flow/app to /get/{path}', () => {
		expect(itemHref({ kind: 'script', path: 'f/a/b' })).toBe('/scripts/get/f/a/b')
		expect(itemHref({ kind: 'flow', path: 'f/a/b' }, 'admins')).toBe(
			'/flows/get/f/a/b?workspace=admins'
		)
		expect(itemHref({ kind: 'app', path: 'u/me/dash' }, 'ws1')).toBe(
			'/apps/get/u/me/dash?workspace=ws1'
		)
	})

	it('routes variable / resource / schedule to list page with hash fragment', () => {
		expect(itemHref({ kind: 'variable', path: 'u/me/secret' })).toBe('/variables#u/me/secret')
		expect(itemHref({ kind: 'resource', path: 'u/me/db' }, 'ws1')).toBe(
			'/resources?workspace=ws1#/resource/u/me/db'
		)
		expect(itemHref({ kind: 'schedule', path: 'f/etl/daily' })).toBe('/schedules#f/etl/daily')
	})

	it('routes each trigger kind with hash fragment', () => {
		const cases: Array<[WorkspaceItemEntry['kind'], string]> = [
			['http_trigger', '/routes#f/a/b'],
			['websocket_trigger', '/websocket_triggers#f/a/b'],
			['kafka_trigger', '/kafka_triggers#f/a/b'],
			['nats_trigger', '/nats_triggers#f/a/b'],
			['postgres_trigger', '/postgres_triggers#f/a/b'],
			['mqtt_trigger', '/mqtt_triggers#f/a/b'],
			['sqs_trigger', '/sqs_triggers#f/a/b'],
			['gcp_trigger', '/gcp_triggers#f/a/b'],
			['azure_trigger', '/azure_triggers#f/a/b'],
			['email_trigger', '/email_triggers#f/a/b']
		]
		for (const [kind, expected] of cases) {
			expect(itemHref({ kind, path: 'f/a/b' })).toBe(expected)
		}
	})

	it('puts workspace query param before the hash so the router applies it', () => {
		expect(itemHref({ kind: 'variable', path: 'u/me/secret' }, 'ws1')).toBe(
			'/variables?workspace=ws1#u/me/secret'
		)
		expect(itemHref({ kind: 'http_trigger', path: 'f/a/b' }, 'ws1')).toBe(
			'/routes?workspace=ws1#f/a/b'
		)
	})
})

describe('workspaceItemAction', () => {
	it('creates drawer actions for variables and resources', () => {
		expect(workspaceItemAction('variable', 'u/me/secret')).toMatchObject({
			type: 'open_created_resource',
			resource: 'variable',
			path: 'u/me/secret'
		})
		expect(workspaceItemAction('resource', 'u/me/db')).toMatchObject({
			type: 'open_created_resource',
			resource: 'resource',
			path: 'u/me/db'
		})
	})

	it('creates drawer actions for schedules and triggers when target kind is known', () => {
		expect(workspaceItemAction('schedule', 'f/etl/daily', 'flow')).toMatchObject({
			resource: 'schedule',
			targetKind: 'flow'
		})
		expect(workspaceItemAction('http_trigger', 'f/api/route', 'script')).toMatchObject({
			resource: 'trigger',
			triggerKind: 'http',
			targetKind: 'script'
		})
		expect(workspaceItemAction('email_trigger', 'f/mail/inbox', 'script')).toMatchObject({
			resource: 'trigger',
			triggerKind: 'email',
			targetKind: 'script'
		})
	})

	it('skips non-drawerable items and trigger items without target kind', () => {
		expect(workspaceItemAction('script', 'f/a/b')).toBeUndefined()
		expect(workspaceItemAction('flow', 'f/a/b')).toBeUndefined()
		expect(workspaceItemAction('app', 'f/a/b')).toBeUndefined()
		expect(workspaceItemAction('schedule', 'f/a/b')).toBeUndefined()
		expect(workspaceItemAction('http_trigger', 'f/a/b')).toBeUndefined()
	})
})

const SAMPLE_ENTRIES: Record<string, WorkspaceItemEntry> = {
	'f/marketing/send_email': {
		kind: 'script',
		path: 'f/marketing/send_email'
	},
	'u/admin/cleanup_old_jobs': {
		kind: 'flow',
		path: 'u/admin/cleanup_old_jobs'
	},
	'f/ops/dashboard': { kind: 'app', path: 'f/ops/dashboard' },
	'f/etl/daily': {
		kind: 'schedule',
		path: 'f/etl/daily',
		targetKind: 'flow'
	}
}

function buildProcessor(workspace?: string) {
	return unified()
		.use(remarkParse)
		.use(remarkWindmillPaths({ resolve: (p) => SAMPLE_ENTRIES[p], workspace }))
}

function findLinks(tree: MdastRoot): Link[] {
	const out: Link[] = []
	const walk = (node: any) => {
		if (!node) return
		if (node.type === 'link') out.push(node as Link)
		if (Array.isArray(node.children)) node.children.forEach(walk)
	}
	walk(tree)
	return out
}

function findText(tree: MdastRoot): Text[] {
	const out: Text[] = []
	const walk = (node: any) => {
		if (!node) return
		if (node.type === 'text') out.push(node as Text)
		if (Array.isArray(node.children)) node.children.forEach(walk)
	}
	walk(tree)
	return out
}

describe('remarkWindmillPaths (mdast)', () => {
	it('rewrites known script / flow / app paths to link nodes with hProperties', () => {
		const processor = buildProcessor('admins')
		const tree = processor.runSync(
			processor.parse(
				'Use f/marketing/send_email and u/admin/cleanup_old_jobs, also try f/ops/dashboard.'
			)
		) as MdastRoot

		const links = findLinks(tree)
		expect(links).toHaveLength(3)

		const byPath = Object.fromEntries(
			links.map((l) => [(l.children[0] as Text).value, l])
		) as Record<string, Link>

		expect(byPath['f/marketing/send_email'].url).toBe(
			'/scripts/get/f/marketing/send_email?workspace=admins'
		)
		expect(byPath['u/admin/cleanup_old_jobs'].url).toBe(
			'/flows/get/u/admin/cleanup_old_jobs?workspace=admins'
		)
		expect(byPath['f/ops/dashboard'].url).toBe('/apps/get/f/ops/dashboard?workspace=admins')

		expect(byPath['f/marketing/send_email'].title).toBeNull()

		const props = byPath['f/marketing/send_email'].data?.hProperties as Record<string, string>
		expect(props['data-wm-kind']).toBe('script')
		expect(props['data-wm-path']).toBe('f/marketing/send_email')
	})

	it('adds target kind metadata when a drawer action needs it', () => {
		const processor = buildProcessor('admins')
		const tree = processor.runSync(processor.parse('Open f/etl/daily.')) as MdastRoot
		const links = findLinks(tree)
		expect(links).toHaveLength(1)
		const props = links[0].data?.hProperties as Record<string, string>
		expect(props['data-wm-kind']).toBe('schedule')
		expect(props['data-wm-path']).toBe('f/etl/daily')
		expect(props['data-wm-target-kind']).toBe('flow')
	})

	it('leaves unknown paths as plain text', () => {
		const processor = buildProcessor()
		const tree = processor.runSync(
			processor.parse('Looking for f/nope/missing or u/ghost/script')
		) as MdastRoot
		expect(findLinks(tree)).toHaveLength(0)
		const joined = findText(tree)
			.map((t) => t.value)
			.join('')
		expect(joined).toContain('f/nope/missing')
		expect(joined).toContain('u/ghost/script')
	})

	it('rewrites standalone inline-code paths into link pills', () => {
		const processor = buildProcessor('admins')
		const tree = processor.runSync(
			processor.parse('Open `f/marketing/send_email` to see it.')
		) as MdastRoot
		const links = findLinks(tree)
		expect(links).toHaveLength(1)
		expect(links[0].url).toBe('/scripts/get/f/marketing/send_email?workspace=admins')
		expect((links[0].data?.hProperties as Record<string, string>)['data-wm-kind']).toBe('script')
	})

	it('leaves inline code alone when it contains more than just a path', () => {
		const processor = buildProcessor()
		const tree = processor.runSync(
			processor.parse('Like `f/marketing/send_email and friends` should stay code.')
		) as MdastRoot
		expect(findLinks(tree)).toHaveLength(0)
	})

	it('does not rewrite paths inside fenced code blocks', () => {
		const processor = buildProcessor()
		const tree = processor.runSync(
			processor.parse('```\nf/marketing/send_email stays in the block\n```\n')
		) as MdastRoot
		expect(findLinks(tree)).toHaveLength(0)
	})

	it('leaves inline code untouched when the wrapped path is unknown', () => {
		const processor = buildProcessor()
		const tree = processor.runSync(processor.parse('Try `f/nope/missing` instead.')) as MdastRoot
		expect(findLinks(tree)).toHaveLength(0)
	})

	it('does not rewrite paths inside existing links (e.g. autolinked URLs)', () => {
		const processor = buildProcessor()
		// Markdown-explicit link with a URL containing what looks like a Windmill path.
		const tree = processor.runSync(
			processor.parse('See [docs](https://example.com/f/marketing/send_email).')
		) as MdastRoot
		const links = findLinks(tree)
		expect(links).toHaveLength(1)
		// Original docs link preserved, no synthetic Windmill link added.
		expect(links[0].url).toBe('https://example.com/f/marketing/send_email')
		expect(links[0].data?.hProperties).toBeUndefined()
	})

	it('handles bold / italic wrapped paths', () => {
		const processor = buildProcessor()
		const tree = processor.runSync(
			processor.parse('Run **f/marketing/send_email** today, or _u/admin/cleanup_old_jobs_.')
		) as MdastRoot
		const links = findLinks(tree)
		expect(links).toHaveLength(2)
		expect(new Set(links.map((l) => (l.children[0] as Text).value))).toEqual(
			new Set(['f/marketing/send_email', 'u/admin/cleanup_old_jobs'])
		)
	})

	it('preserves data attributes through remark-rehype', () => {
		const processor = buildProcessor('admins').use(remarkRehype, { allowDangerousHtml: true })
		const hast: any = processor.runSync(processor.parse('Use f/marketing/send_email today.'))
		// Walk hast tree to find <a> element.
		const links: any[] = []
		const walk = (node: any) => {
			if (!node) return
			if (node.type === 'element' && node.tagName === 'a') links.push(node)
			if (Array.isArray(node.children)) node.children.forEach(walk)
		}
		walk(hast)
		expect(links).toHaveLength(1)
		expect(links[0].properties.href).toBe('/scripts/get/f/marketing/send_email?workspace=admins')
		// data-* may be normalized by the mdast/hast bridge.
		expect(links[0].properties['dataWmKind'] ?? links[0].properties['data-wm-kind']).toBe('script')
		expect(links[0].properties['dataWmPath'] ?? links[0].properties['data-wm-path']).toBe(
			'f/marketing/send_email'
		)
	})
})
