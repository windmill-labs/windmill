const input = document.getElementById('url') as HTMLInputElement
const status = document.getElementById('status')!

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
		status.textContent = 'Enter an http(s) URL, such as https://app.windmill.dev'
		return
	}
	// Keeps a base path (an instance served under /windmill), drops everything after it.
	const instanceUrl = (url.origin + url.pathname).replace(/\/+$/, '')
	await chrome.storage.sync.set({ instanceUrl })
	input.value = instanceUrl
	status.textContent = 'Saved. Reopen the side panel to use it.'
})

export {}
