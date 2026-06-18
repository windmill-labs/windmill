import React, { useState } from 'react'
import { backend } from 'wmill'

type Message = {
	role: 'user' | 'assistant'
	content: string
}

const SYSTEM_PROMPT =
	'You are Boris, the internal AI assistant for Acme Corporation. Be helpful, friendly, and concise.'

function generateSessionId(): string {
	return crypto.randomUUID()
}

function getSessionId(): string {
	let sessionId = sessionStorage.getItem('chat_session_id')
	if (!sessionId) {
		sessionId = generateSessionId()
		sessionStorage.setItem('chat_session_id', sessionId)
	}
	return sessionId
}

const App = () => {
	const [messages, setMessages] = useState<Message[]>([])
	const [input, setInput] = useState('')
	const [loading, setLoading] = useState(false)
	const [sessionId, setSessionId] = useState<string>(getSessionId())

	async function sendMessage(e: React.FormEvent) {
		e.preventDefault()
		if (!input.trim() || loading) return

		const userMessage = input.trim()
		setInput('')
		setLoading(true)
		setMessages((prev) => [...prev, { role: 'user', content: userMessage }])

		try {
			const response = await backend.a({
				session_id: sessionId,
				message: userMessage,
				system_prompt: SYSTEM_PROMPT
			})
			const content = typeof response === 'string' ? response : response.content
			setMessages((prev) => [...prev, { role: 'assistant', content }])
		} finally {
			setLoading(false)
		}
	}

	function startNewChat() {
		const newSessionId = generateSessionId()
		sessionStorage.setItem('chat_session_id', newSessionId)
		setSessionId(newSessionId)
		setMessages([])
	}

	return (
		<div className="chat-app">
			<header>
				<h1>Boris</h1>
				<p>Session: {sessionId}</p>
				<button onClick={startNewChat}>New Chat</button>
			</header>

			<main>
				{messages.length === 0 ? (
					<p>Ask Boris anything about the company.</p>
				) : (
					<ul>
						{messages.map((message, index) => (
							<li key={index}>
								<strong>{message.role}:</strong> {message.content}
							</li>
						))}
					</ul>
				)}
			</main>

			<form onSubmit={sendMessage}>
				<input
					type="text"
					value={input}
					onChange={(e) => setInput(e.target.value)}
					placeholder="Type your message..."
					disabled={loading}
				/>
				<button type="submit" disabled={loading || !input.trim()}>
					Send
				</button>
			</form>
		</div>
	)
}

export default App
