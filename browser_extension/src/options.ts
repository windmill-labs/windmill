const input = document.getElementById('url') as HTMLInputElement
const save = document.getElementById('save') as HTMLButtonElement
const status = document.getElementById('status')!

function show(text: string, kind: 'ok' | 'error' | '' = '') {
	status.textContent = text
	status.className = `status ${kind}`
}

chrome.storage.sync
	.get('instanceUrl')
	.then(({ instanceUrl }) => (input.value = (instanceUrl as string) ?? ''))

document.getElementById('form')!.addEventListener('submit', async (e) => {
	e.preventDefault()
	let url: URL
	try {
		url = new URL(input.value.trim())
		if (url.protocol !== 'https:' && url.protocol !== 'http:') throw new Error()
	} catch {
		return show('Enter an http(s) URL, such as https://app.windmill.dev', 'error')
	}
	// Keeps a base path (an instance served under /windmill), drops everything after it.
	const instanceUrl = (url.origin + url.pathname).replace(/\/+$/, '')
	input.value = instanceUrl
	save.disabled = true
	show('Checking…')
	try {
		const res = await fetch(`${instanceUrl}/api/version`)
		const version = res.ok ? (await res.text()).trim() : ''
		if (!version) throw new Error()
		await chrome.storage.sync.set({ instanceUrl })
		show(`Connected to Windmill ${version}. Open the side panel from the toolbar.`, 'ok')
	} catch {
		show(`No Windmill instance answered at ${instanceUrl}.`, 'error')
	} finally {
		save.disabled = false
	}
})

export {}
