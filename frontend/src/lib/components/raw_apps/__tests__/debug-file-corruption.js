/**
 * Browser console script to debug file corruption
 * 
 * USAGE:
 * 1. Open the raw app editor page
 * 2. Paste this script in the browser console
 * 3. Ask AI to create/edit a file
 * 4. Watch the console for detailed logs
 */

(function() {
	console.log('[CORRUPTION DEBUG] Installing debugging hooks...')
	
	// Store original postMessage
	const originalPostMessage = window.postMessage
	const iframeMessages = []
	const parentMessages = []
	
	// Hook window.postMessage to track messages to iframe
	window.postMessage = function(message, targetOrigin, transfer) {
		if (message && message.type === 'setFiles') {
			console.log('[DEBUG] Parent -> Iframe: setFiles', {
				fileCount: Object.keys(message.files || {}).length,
				timestamp: Date.now(),
				files: Object.fromEntries(
					Object.entries(message.files || {}).map(([path, content]) => [
						path,
						{
							length: content.length,
							hash: hashString(content),
							preview: content.substring(0, 100)
						}
					])
				)
			})
			iframeMessages.push({
				type: 'outgoing',
				timestamp: Date.now(),
				message: JSON.parse(JSON.stringify(message))
			})
		}
		return originalPostMessage.call(this, message, targetOrigin, transfer)
	}
	
	// Hook addEventListener to track messages from iframe
	const originalAddEventListener = window.addEventListener
	window.addEventListener = function(type, listener, options) {
		if (type === 'message') {
			const wrappedListener = function(event) {
				if (event.data && event.data.type === 'setFiles') {
					console.log('[DEBUG] Iframe -> Parent: setFiles', {
						fileCount: Object.keys(event.data.files || {}).length,
						timestamp: Date.now(),
						files: Object.fromEntries(
							Object.entries(event.data.files || {}).map(([path, content]) => [
								path,
								{
									length: content.length,
									hash: hashString(content),
									preview: content.substring(0, 100)
								}
							])
						)
					})
					parentMessages.push({
						type: 'incoming',
						timestamp: Date.now(),
						message: JSON.parse(JSON.stringify(event.data))
					})
					
					// Check for corruption
					if (iframeMessages.length > 0) {
						const lastSent = iframeMessages[iframeMessages.length - 1].message
						checkForCorruption(lastSent.files, event.data.files)
					}
				}
				return listener.call(this, event)
			}
			return originalAddEventListener.call(this, type, wrappedListener, options)
		}
		return originalAddEventListener.call(this, type, listener, options)
	}
	
	function hashString(str) {
		let hash = 0
		for (let i = 0; i < str.length; i++) {
			const char = str.charCodeAt(i)
			hash = ((hash << 5) - hash) + char
			hash = hash & hash // Convert to 32bit integer
		}
		return hash
	}
	
	function checkForCorruption(sent, received) {
		console.group('[CORRUPTION CHECK]')
		
		const sentPaths = Object.keys(sent || {})
		const receivedPaths = Object.keys(received || {})
		
		console.log('Files sent:', sentPaths.length)
		console.log('Files received:', receivedPaths.length)
		
		// Check each file
		for (const path of sentPaths) {
			const sentContent = sent[path]
			const receivedContent = received[path]
			
			if (!receivedContent) {
				console.warn(`❌ File missing in response: ${path}`)
				continue
			}
			
			const sentHash = hashString(sentContent)
			const receivedHash = hashString(receivedContent)
			
			if (sentHash !== receivedHash) {
				console.error(`🔴 CORRUPTION DETECTED: ${path}`)
				console.log('Sent length:', sentContent.length)
				console.log('Received length:', receivedContent.length)
				console.log('Sent hash:', sentHash)
				console.log('Received hash:', receivedHash)
				
				// Find first difference
				const maxLen = Math.min(sentContent.length, receivedContent.length)
				for (let i = 0; i < maxLen; i++) {
					if (sentContent[i] !== receivedContent[i]) {
						console.log(`First diff at position ${i}:`)
						console.log(`  Sent:     "${sentContent.substring(Math.max(0, i-20), i+50)}"`)
						console.log(`  Received: "${receivedContent.substring(Math.max(0, i-20), i+50)}"`)
						break
					}
				}
				
				// Save for inspection
				window.__corruptedFile = {
					path,
					sent: sentContent,
					received: receivedContent
				}
				console.log('💾 Saved to window.__corruptedFile for inspection')
			} else {
				console.log(`✅ ${path}: No corruption`)
			}
		}
		
		console.groupEnd()
	}
	
	// Export utilities
	window.__debugFileCorruption = {
		iframeMessages,
		parentMessages,
		getLastSent: () => iframeMessages[iframeMessages.length - 1],
		getLastReceived: () => parentMessages[parentMessages.length - 1],
		compare: (sentIdx = -1, receivedIdx = -1) => {
			const sent = iframeMessages[sentIdx < 0 ? iframeMessages.length + sentIdx : sentIdx]
			const received = parentMessages[receivedIdx < 0 ? parentMessages.length + receivedIdx : receivedIdx]
			
			if (!sent || !received) {
				console.error('Invalid indices')
				return
			}
			
			checkForCorruption(sent.message.files, received.message.files)
		},
		clear: () => {
			iframeMessages.length = 0
			parentMessages.length = 0
			console.log('Cleared message history')
		}
	}
	
	console.log('[CORRUPTION DEBUG] Hooks installed! ✅')
	console.log('Use window.__debugFileCorruption to inspect messages')
	console.log('  .iframeMessages - Messages sent to iframe')
	console.log('  .parentMessages - Messages received from iframe')
	console.log('  .compare() - Compare last sent/received')
	console.log('  .clear() - Clear history')
})()
