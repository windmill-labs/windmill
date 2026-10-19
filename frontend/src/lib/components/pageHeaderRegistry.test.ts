import { describe, expect, it, afterEach } from 'vitest'
import { pageHeader, type PageHeaderContent } from './pageHeaderRegistry.svelte'
import type { Snippet } from 'svelte'

// The merge rule every registration site depends on: a page sets the frame and something nested
// in it fills the breadcrumb, so whether a component registers at all decides whose name the band
// shows. A viewer that registered wherever it was mounted renamed the band of the session hosting
// it.
const snippet = (name: string) => name as unknown as Snippet

const ids: number[] = []
function register(c: PageHeaderContent) {
	ids.push(pageHeader.register(() => c))
}
afterEach(() => {
	while (ids.length) pageHeader.release(ids.pop()!)
})

describe('pageHeader', () => {
	it('is empty with nothing registered', () => {
		expect(pageHeader.content).toBeUndefined()
	})

	it('lets a later registration win field by field', () => {
		register({ section: { label: 'Sessions' }, barRightInset: 400 })
		register({ item: { path: 'u/admin/app' } })
		expect(pageHeader.content).toEqual({
			section: { label: 'Sessions' },
			item: { path: 'u/admin/app' },
			barRightInset: 400
		})
	})

	it('gives the fields of a released registration back', () => {
		register({ section: { label: 'Sessions' } })
		register({ item: { path: 'u/admin/app' } })
		pageHeader.release(ids.pop()!)
		expect(pageHeader.content?.item).toBeUndefined()
		expect(pageHeader.content?.section).toEqual({ label: 'Sessions' })
	})

	it('orders actions by actionsOrder, then by registration', () => {
		register({ actions: snippet('page'), actionsOrder: 10 })
		register({ actions: snippet('nested') })
		register({ actions: snippet('alsoNested') })
		expect(pageHeader.actions.map((a) => a.render)).toEqual(['nested', 'alsoNested', 'page'])
	})

	it('carries the contexts of each registration with its actions', () => {
		const pageContexts = new Map([['TriggerContext', 1]])
		register({ actions: snippet('page'), contexts: pageContexts })
		register({ actions: snippet('nested') })
		expect(pageHeader.actions.map((a) => a.contexts)).toEqual([pageContexts, undefined])
	})

	// The bar draws one session button, so which registration it reads decides where the session
	// opens; a nested registration that names no source must not hide the page's.
	it('takes the session source of the innermost registration that has one, with its contexts', () => {
		const routeContexts = new Map([['route', 1]])
		const editorContexts = new Map([['editor', 2]])
		const route = { page: () => '/runs' }
		const editor = { target: { kind: 'script' as const, path: 'u/admin/s' } }
		register({ sessionSource: route, contexts: routeContexts })
		register({ sessionSource: editor, contexts: editorContexts })
		register({ section: { label: 'Nested' } })
		expect(pageHeader.session).toEqual({ source: editor, contexts: editorContexts })
		pageHeader.release(ids.splice(1, 1)[0])
		expect(pageHeader.session).toEqual({ source: route, contexts: routeContexts })
	})

	it('has no session source once its registration is released', () => {
		register({ sessionSource: { page: () => '/runs' } })
		pageHeader.release(ids.pop()!)
		expect(pageHeader.session).toBeUndefined()
	})
})
