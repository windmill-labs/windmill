chrome.sidePanel.setPanelBehavior({ openPanelOnActionClick: true })

chrome.runtime.onInstalled.addListener(async () => {
	const { instanceUrl } = await chrome.storage.sync.get('instanceUrl')
	if (!instanceUrl) chrome.runtime.openOptionsPage()
})

export {}
