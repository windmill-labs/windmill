// Hosts Windmill's chat-only session route in an iframe and executes the browser tools it
// relays (frontend `global/browserTools.ts`) on the active tab of this window.

type Target = { ref?: number; selector?: string }
type Call =
	| { tool: 'read'; args: {} }
	| { tool: 'screenshot'; args: {} }
	| { tool: 'click'; args: Target }
	| { tool: 'type'; args: Target & { text: string; submit?: boolean } }
	| { tool: 'navigate'; args: { url: string } }

const frame = document.getElementById('frame') as HTMLIFrameElement

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
			generation++
			declineOpen?.()
		} else if (msg?.type === 'wm-browser:call' && typeof msg.id === 'string') {
			try {
				const result = await run({ tool: msg.tool, args: msg.args ?? {} } as Call, generation)
				reply({ type: 'wm-browser:result', id: msg.id, ok: true, result })
			} catch (err) {
				const error = err instanceof Error ? err.message : String(err)
				reply({ type: 'wm-browser:result', id: msg.id, ok: false, error })
			}
		}
	})
}

async function run(call: Call, gen: number): Promise<unknown> {
	const [tab] = await chrome.tabs.query({ active: true, currentWindow: true })
	if (tab?.id === undefined) throw new Error('No active tab')
	const tabId = tab.id
	switch (call.tool) {
		case 'read':
			return (await exec({ tabId }, readPage, [30_000]))[0]
		case 'screenshot':
			return chrome.tabs.captureVisibleTab(tab.windowId, { format: 'jpeg', quality: 80 })
		case 'click': {
			const [label, documentId] = await exec({ tabId }, pageAction, ['describe', call.args])
			await confirmAction(`Click ${label}`, tab, gen)
			return execConfirmed(tabId, documentId, ['click', call.args])
		}
		case 'type': {
			const [label, documentId] = await exec({ tabId }, pageAction, ['describe', call.args])
			const submit = call.args.submit ? ' and submit' : ''
			await confirmAction(`Type "${call.args.text}" into ${label}${submit}`, tab, gen)
			return execConfirmed(tabId, documentId, ['type', call.args])
		}
		case 'navigate': {
			const url = new URL(call.args.url)
			if (url.protocol !== 'https:' && url.protocol !== 'http:') {
				throw new Error('Only http(s) URLs can be opened')
			}
			await confirmAction(`Open ${url.href}`, tab, gen)
			await chrome.tabs.update(tabId, { url: url.href })
			return `Navigating to ${url.href}`
		}
		default:
			throw new Error(`Unknown browser tool ${(call as { tool: string }).tool}`)
	}
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

/** Pinned to the document the user confirmed: if the tab navigated meanwhile, this fails. */
async function execConfirmed(
	tabId: number,
	documentId: string,
	args: Parameters<typeof pageAction>
): Promise<unknown> {
	const target = { tabId, documentIds: [documentId] }
	const [result] = await exec(target, pageAction, args).catch((e: Error) => {
		throw /No document with id/.test(e.message)
			? new Error('The page changed while waiting for approval. Read it again.')
			: e
	})
	return result
}

let confirmQueue: Promise<unknown> = Promise.resolve()
// Bumped when the chat stops its turn: a confirmation asked before that is never shown, and
// the open one is declined.
let generation = 0
let declineOpen: (() => void) | undefined

/** Shown outside the Windmill frame, so neither the chat nor the page can answer it. */
function confirmAction(text: string, tab: chrome.tabs.Tab, gen: number): Promise<void> {
	const ask = () =>
		new Promise<void>((resolve, reject) => {
			if (gen !== generation) return reject(new Error('Stopped by the user'))
			const box = document.getElementById('confirm')!
			document.getElementById('confirm-text')!.textContent = text
			document.getElementById('confirm-host')!.textContent = `On ${tab.url ?? 'the active tab'}`
			box.hidden = false
			const allow = document.getElementById('allow') as HTMLButtonElement
			const deny = document.getElementById('deny') as HTMLButtonElement
			const done = (ok: boolean) => {
				box.hidden = true
				allow.onclick = deny.onclick = null
				declineOpen = undefined
				if (ok) resolve()
				else reject(new Error('The user declined this action'))
			}
			allow.onclick = () => done(true)
			deny.onclick = () => done(false)
			declineOpen = () => done(false)
			// Nothing is focused: a keystroke meant for the chat composer must not approve.
		})
	const next = confirmQueue.then(ask, ask)
	confirmQueue = next.catch(() => {})
	return next
}

function readPage(maxChars: number) {
	document.querySelectorAll('[data-wm-ref]').forEach((el) => el.removeAttribute('data-wm-ref'))
	const elements: string[] = []
	const candidates = document.querySelectorAll<HTMLElement>(
		'a[href], button, input, textarea, select, [role=button], [role=link], [contenteditable=true]'
	)
	for (const el of candidates) {
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
	a: { ref?: number; selector?: string; text?: string; submit?: boolean }
) {
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
	const input = el as HTMLInputElement
	if (action === 'describe') {
		const label = (
			el.getAttribute('aria-label') ||
			el.innerText ||
			input.placeholder ||
			input.name ||
			''
		)
			.trim()
			.replace(/\s+/g, ' ')
			.slice(0, 80)
		return { result: `<${el.tagName.toLowerCase()}> "${label}"` }
	}
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
		const form = input.form
		if (form) form.requestSubmit()
		else el.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }))
	}
	return { result: a.submit ? 'Typed and submitted' : 'Typed' }
}

main()

export {}
