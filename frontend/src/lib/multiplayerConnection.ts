import { WebsocketProvider } from 'y-websocket'
import type * as Y from 'yjs'
import { buildWsUrl } from '$lib/wsUrl'
import { signMultiplayerRequest } from '$lib/components/debug/debugUtils'

const BASE_DELAY_MS = 1000
const MAX_DELAY_MS = 60_000

/**
 * A y-websocket provider for the multiplayer server that reconnects with its own
 * exponential backoff and a freshly signed token on every attempt.
 *
 * y-websocket's built-in reconnect is not usable here: it reuses the token it was
 * built with, which lives 60s, and the server rejects a bad token only after the
 * socket opened, which resets y-websocket's backoff to 100ms. Any drop past the
 * first minute would otherwise retry ten times a second, forever. An attempt only
 * counts as successful once the document synced.
 */
export class MultiplayerConnection {
	readonly provider: WebsocketProvider
	#workspace: string
	#failures = 0
	#timer: ReturnType<typeof setTimeout> | undefined
	#destroyed = false

	private constructor(workspace: string, provider: WebsocketProvider) {
		this.#workspace = workspace
		this.provider = provider
		provider.on('sync', (synced: boolean) => {
			if (synced) this.#failures = 0
		})
		provider.on('connection-close', () => {
			if (this.#destroyed) return
			// Stops y-websocket's own reconnect timer from firing setupWS.
			provider.shouldConnect = false
			this.#scheduleReconnect()
		})
	}

	/** Throws when the first token cannot be signed. */
	static async open(workspace: string, room: string, doc: Y.Doc): Promise<MultiplayerConnection> {
		const token = await signMultiplayerRequest(workspace)
		const provider = new WebsocketProvider(buildWsUrl('/ws_mp/'), room, doc, {
			connect: false,
			params: { token }
		})
		const connection = new MultiplayerConnection(workspace, provider)
		provider.connect()
		return connection
	}

	#scheduleReconnect() {
		clearTimeout(this.#timer)
		const ceiling = Math.min(BASE_DELAY_MS * 2 ** this.#failures, MAX_DELAY_MS)
		this.#failures++
		const delay = ceiling / 2 + Math.random() * (ceiling / 2)
		this.#timer = setTimeout(() => this.#reconnect(), delay)
	}

	async #reconnect() {
		let token: string
		try {
			token = await signMultiplayerRequest(this.#workspace)
		} catch (e) {
			if (this.#destroyed) return
			console.error('Failed to sign multiplayer request:', e)
			this.#scheduleReconnect()
			return
		}
		if (this.#destroyed) return
		const url = new URL(this.provider.url)
		url.searchParams.set('token', token)
		this.provider.url = url.toString()
		this.provider.connect()
	}

	destroy() {
		this.#destroyed = true
		clearTimeout(this.#timer)
		this.provider.destroy()
	}
}
