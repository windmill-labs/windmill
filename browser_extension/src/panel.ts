// Hosts Windmill's chat-only session route in an iframe and executes the browser tools it
// relays (frontend `global/browserTools.ts`) on the active tab of this window.
//
// Click, type and navigate run in two steps around the chat's own approval card: `prepare`
// pins the tab, document and element and describes them for the card, and `act` runs only a
// prepared action, after re-checking that what was described is still there.

type Target = { ref?: number; selector?: string }
type Action =
	| { tool: 'click'; args: Target }
	| { tool: 'type'; args: Target & { text: string; submit?: boolean } }
	| { tool: 'navigate'; args: { url: string } }
type Call =
	| { tool: 'read'; args: {} }
	| { tool: 'screenshot'; args: {} }
	| { tool: 'prepare'; args: Action }
	| { tool: 'act'; args: { approvalId: string } }
type Prepared = Action & { tab: chrome.tabs.Tab; documentId?: string; nonce: string }

const frame = document.getElementById('frame') as HTMLIFrameElement
const prepared = new Map<string, Prepared>()

async function main() {
	const { instanceUrl } = (await chrome.storage.sync.get('instanceUrl')) as { instanceUrl?: string }
	if (!instanceUrl) {
		document.getElementById('setup')!.hidden = false
		document.getElementById('open-options')!.onclick = () => chrome.runtime.openOptionsPage()
		return
	}
	const origin = new URL(instanceUrl).origin
	frame.src = `${instanceUrl}/sessions/browser?nomenubar=true`
	frame.hidden = false

	window.addEventListener('message', async (e) => {
		// Only the instance's own document in our frame may drive the tools: a page the frame
		// navigated to elsewhere fails the origin check.
		if (e.source !== frame.contentWindow || e.origin !== origin) return
		const msg = e.data
		const reply = (m: object) => frame.contentWindow?.postMessage(m, origin)
		if (msg?.type === 'wm-browser:hello') {
			reply({ type: 'wm-browser:ready' })
		} else if (msg?.type === 'wm-browser:cancel') {
			prepared.clear()
		} else if (msg?.type === 'wm-browser:call' && typeof msg.id === 'string') {
			try {
				const result = await run({ tool: msg.tool, args: msg.args ?? {} } as Call)
				reply({ type: 'wm-browser:result', id: msg.id, ok: true, result })
			} catch (err) {
				const error = err instanceof Error ? err.message : String(err)
				reply({ type: 'wm-browser:result', id: msg.id, ok: false, error })
			}
		}
	})
}

async function run(call: Call): Promise<unknown> {
	if (call.tool === 'act') return act(call.args.approvalId)
	const [tab] = await chrome.tabs.query({ active: true, currentWindow: true })
	if (tab?.id === undefined) throw new Error('No active tab')
	const tabId = tab.id
	switch (call.tool) {
		case 'read':
			return (await exec({ tabId }, readPage, [30_000]))[0]
		case 'screenshot':
			return chrome.tabs.captureVisibleTab(tab.windowId, { format: 'jpeg', quality: 80 })
		case 'prepare':
			return prepare(call.args, tab)
		default:
			throw new Error(`Unknown browser tool ${(call as { tool: string }).tool}`)
	}
}

async function prepare(action: Action, tab: chrome.tabs.Tab) {
	const tabId = tab.id!
	const nonce = crypto.randomUUID()
	const host = tab.url ? new URL(tab.url).host : 'the active tab'
	let label: string
	let documentId: string | undefined
	if (action.tool === 'navigate') {
		const url = new URL(action.args.url)
		if (url.protocol !== 'https:' && url.protocol !== 'http:') {
			throw new Error('Only http(s) URLs can be opened')
		}
		// A page the extension cannot script (a browser page) is pinned by its URL alone.
		documentId = await exec({ tabId }, () => ({ result: null }), []).then(
			([, id]) => id,
			() => undefined
		)
		label = `Open ${url.href}`
	} else {
		const [target, id] = await exec({ tabId }, pageAction, ['describe', action.args, nonce])
		documentId = id
		const submit = action.tool === 'type' && action.args.submit ? ' and submit' : ''
		label =
			action.tool === 'click'
				? `Click ${target} on ${host}`
				: `Type "${action.args.text}" into ${target} on ${host}${submit}`
	}
	const approvalId = crypto.randomUUID()
	prepared.set(approvalId, { ...action, tab, documentId, nonce })
	return { approvalId, label }
}

async function act(approvalId: string): Promise<unknown> {
	const p = prepared.get(approvalId)
	prepared.delete(approvalId)
	if (!p) throw new Error('This action was withdrawn. Ask again.')
	await stillApproved(p.tab, p.documentId)
	const tabId = p.tab.id!
	if (p.tool === 'navigate') {
		await chrome.tabs.update(tabId, { url: new URL(p.args.url).href })
		return `Navigating to ${p.args.url}`
	}
	const target = { tabId, documentIds: [p.documentId!] }
	const [result] = await exec(target, pageAction, [p.tool, p.args, p.nonce]).catch((e: Error) => {
		throw /No document with id/.test(e.message) ? new Error(PAGE_CHANGED) : e
	})
	return result
}

// Injected functions run in the page and must be self-contained; they report failure as
// `{ error }` because a throw inside them reaches us without its message.
async function exec<A extends any[]>(
	target: chrome.scripting.InjectionTarget,
	func: (...args: A) => { result?: unknown; error?: string },
	args: A
): Promise<[any, string]> {
	const [injection] = await chrome.scripting.executeScript({ target, func, args })
	const out = injection?.result as { result?: unknown; error?: string } | undefined
	if (!out) throw new Error('The page did not answer')
	if (out.error) throw new Error(out.error)
	return [out.result, injection.documentId]
}

const PAGE_CHANGED = 'The page changed while waiting for approval. Read it again.'

/** An approval holds only for the document it named, while its tab is still the active one. */
async function stillApproved(tab: chrome.tabs.Tab, documentId: string | undefined) {
	const [current] = await chrome.tabs.query({ active: true, windowId: tab.windowId })
	if (current?.id !== tab.id) {
		throw new Error('The user switched tabs while the approval was pending. Ask again.')
	}
	if (current.url !== tab.url) throw new Error(PAGE_CHANGED)
	if (documentId) {
		await exec({ tabId: tab.id!, documentIds: [documentId] }, () => ({ result: null }), []).catch(
			() => {
				throw new Error(PAGE_CHANGED)
			}
		)
	}
}

function readPage(maxChars: number) {
	document.querySelectorAll('[data-wm-ref]').forEach((el) => el.removeAttribute('data-wm-ref'))
	const elements: string[] = []
	// Fields first: on a page with hundreds of links (a mail inbox) the cap would otherwise
	// drop a compose box rendered at the end of the document.
	const fields = document.querySelectorAll<HTMLElement>(
		'input:not([type=hidden]), textarea, select, [contenteditable]:not([contenteditable=false]), [role=textbox]'
	)
	const others = document.querySelectorAll<HTMLElement>(
		'a[href], button, [role=button], [role=link]'
	)
	for (const el of new Set([...fields, ...others])) {
		if (elements.length >= 300) break
		const rect = el.getBoundingClientRect()
		if (!rect.width || !rect.height || getComputedStyle(el).visibility === 'hidden') continue
		const ref = elements.length + 1
		el.setAttribute('data-wm-ref', String(ref))
		const input = el as HTMLInputElement
		// Never a field's value: it can be a password or anything else the user typed.
		const label =
			el.getAttribute('aria-label') ||
			el.innerText ||
			input.placeholder ||
			(input.type === 'submit' || input.type === 'button' ? input.value : '') ||
			el.title ||
			input.name ||
			''
		const type = input.type && el.tagName !== 'BUTTON' ? ` type=${input.type}` : ''
		const href = (el as HTMLAnchorElement).href ? ` href=${(el as HTMLAnchorElement).href}` : ''
		elements.push(
			`[${ref}] ${el.tagName.toLowerCase()}${type}${href} "${label.trim().replace(/\s+/g, ' ').slice(0, 80)}"`
		)
	}
	return {
		result: {
			url: location.href,
			title: document.title,
			text: document.body.innerText.slice(0, maxChars),
			elements
		}
	}
}

function pageAction(
	action: 'describe' | 'click' | 'type',
	a: { ref?: number; selector?: string; text?: string; submit?: boolean },
	nonce: string
) {
	// Kept in the extension's isolated world, which the page's own scripts can neither read
	// nor alter, so the page cannot move an approval onto another element.
	const approved: Map<string, { el: HTMLElement; label: string }> = ((
		globalThis as any
	).__wmApproved ??= new Map())
	const describe = (el: HTMLElement) => {
		const input = el as HTMLInputElement
		const label = (
			el.getAttribute('aria-label') ||
			el.innerText ||
			input.placeholder ||
			(input.type === 'submit' || input.type === 'button' ? input.value : '') ||
			input.name ||
			''
		)
			.trim()
			.replace(/\s+/g, ' ')
			.slice(0, 80)
		return `<${el.tagName.toLowerCase()}> "${label}"`
	}
	if (action === 'describe') {
		let el: HTMLElement | null = null
		try {
			el =
				a.ref !== undefined
					? document.querySelector(`[data-wm-ref="${Number(a.ref)}"]`)
					: a.selector
						? document.querySelector(a.selector)
						: null
		} catch {
			return { error: `Invalid selector ${a.selector}` }
		}
		if (!el) return { error: 'Element not found. Read the page again for fresh element numbers.' }
		const label = describe(el)
		approved.set(nonce, { el, label })
		return { result: label }
	}
	const entry = approved.get(nonce)
	approved.delete(nonce)
	// Re-described, so an element re-rendered in place into something else is not acted on.
	if (!entry || !entry.el.isConnected || describe(entry.el) !== entry.label) {
		return { error: 'The element changed while waiting for approval. Read the page again.' }
	}
	const el = entry.el
	el.scrollIntoView({ block: 'center' })
	if (action === 'click') {
		el.click()
		return { result: 'Clicked' }
	}
	el.focus()
	if (el.isContentEditable) {
		document.execCommand('selectAll')
		document.execCommand('insertText', false, a.text ?? '')
	} else if (el instanceof HTMLInputElement || el instanceof HTMLTextAreaElement) {
		el.value = a.text ?? ''
		el.dispatchEvent(new Event('input', { bubbles: true }))
		el.dispatchEvent(new Event('change', { bubbles: true }))
	} else {
		return { error: 'That element is not a text field' }
	}
	if (a.submit) {
		const form = (el as HTMLInputElement).form
		if (form) form.requestSubmit()
		else el.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }))
	}
	return { result: a.submit ? 'Typed and submitted' : 'Typed' }
}

main()

export {}
