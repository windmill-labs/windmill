<script lang="ts">
	import { BROWSER } from 'esm-env'
	import Toggle from '../Toggle.svelte'
	import TextInput from '../text_input/TextInput.svelte'
	import { Button } from '../common'
	import { CheckCircle2, XCircle, Loader2 } from 'lucide-svelte'
	import type { Writable } from 'svelte/store'

	interface Props {
		values: Writable<Record<string, any>>
	}

	let { values }: Props = $props()

	let enabled = $derived($values['ws_base_url'] != null)

	interface ServiceResult {
		name: string
		http: 'pending' | 'ok' | 'fail'
		ws: 'pending' | 'ok' | 'fail'
		error?: string
	}

	let testing = $state(false)
	let results = $state<ServiceResult[]>([])

	const SERVICES = [
		{ name: 'LSP', httpPath: '/ws/health', wsPath: '/ws/ping' },
		{ name: 'Multiplayer', httpPath: '/ws_mp/health', wsPath: '/ws_mp/__ping__' },
		{ name: 'Debugger', httpPath: '/ws_debug/health', wsPath: '/ws_debug/ping' }
	]

	function buildTestUrl(path: string): string {
		const override = $values['ws_base_url']
		if (override) {
			const base = override.endsWith('/') ? override.slice(0, -1) : override
			return `${base}${path}`
		}
		if (!BROWSER) return `ws://localhost:3003${path}`
		const wsProtocol = window.location.protocol === 'https:' ? 'wss' : 'ws'
		return `${wsProtocol}://${window.location.host}${path}`
	}

	function wsToHttp(url: string): string {
		return url.replace(/^wss:/, 'https:').replace(/^ws:/, 'http:')
	}

	async function testHttp(wsUrl: string): Promise<boolean> {
		try {
			const httpUrl = wsToHttp(wsUrl)
			const response = await fetch(httpUrl, { signal: AbortSignal.timeout(5000) })
			if (!response.ok) return false
			const data = await response.json()
			return data.status === 'ok'
		} catch {
			return false
		}
	}

	function testWs(url: string): Promise<boolean> {
		return new Promise((resolve) => {
			try {
				const ws = new WebSocket(url)
				const timeout = setTimeout(() => {
					ws.close()
					resolve(false)
				}, 5000)
				ws.onmessage = (e) => {
					clearTimeout(timeout)
					try {
						const data = JSON.parse(e.data)
						ws.close()
						resolve(data.type === 'pong')
					} catch {
						ws.close()
						resolve(false)
					}
				}
				ws.onerror = () => {
					clearTimeout(timeout)
					resolve(false)
				}
				ws.onclose = () => {
					clearTimeout(timeout)
					resolve(false)
				}
			} catch {
				resolve(false)
			}
		})
	}

	async function runTests() {
		testing = true
		results = SERVICES.map((s) => ({
			name: s.name,
			http: 'pending' as const,
			ws: 'pending' as const
		}))

		await Promise.all(
			SERVICES.map(async (service, i) => {
				const wsUrl = buildTestUrl(service.wsPath)
				const healthUrl = buildTestUrl(service.httpPath)

				// HTTP test first
				const httpOk = await testHttp(healthUrl)
				results[i] = { ...results[i], http: httpOk ? 'ok' : 'fail' }

				// WebSocket test
				const wsOk = await testWs(wsUrl)
				results[i] = { ...results[i], ws: wsOk ? 'ok' : 'fail' }
			})
		)

		testing = false
	}

	function toggleEnabled() {
		if (enabled) {
			$values['ws_base_url'] = null
		} else {
			$values['ws_base_url'] = ''
		}
	}
</script>

<div class="flex flex-col gap-3">
	<div>
		<Button variant="default" unifiedSize="sm" onclick={runTests} disabled={testing}>
			{#if testing}
				Test connectivity...
			{:else}
				Test connectivity
			{/if}
		</Button>
	</div>

	{#if results.length > 0}
		<div class="flex flex-col gap-1.5 text-xs">
			{#each results as result (result.name)}
				<div class="flex items-center gap-3">
					<span class="w-20 font-medium text-primary">{result.name}</span>
					<span class="flex items-center gap-1">
						{#if result.http === 'pending'}
							<Loader2 size={14} class="animate-spin text-tertiary" />
						{:else if result.http === 'ok'}
							<CheckCircle2 size={14} class="text-green-600" />
						{:else}
							<XCircle size={14} class="text-red-500" />
						{/if}
						<span class="text-secondary">HTTP</span>
					</span>
					<span class="flex items-center gap-1">
						{#if result.ws === 'pending'}
							<Loader2 size={14} class="animate-spin text-tertiary" />
						{:else if result.ws === 'ok'}
							<CheckCircle2 size={14} class="text-green-600" />
						{:else}
							<XCircle size={14} class="text-red-500" />
						{/if}
						<span class="text-secondary">WebSocket</span>
					</span>
				</div>
			{/each}
		</div>
	{/if}

	<div>
		<Toggle
			size="xs"
			options={{ right: 'Custom websocket base url from frontend to multiplayer/lsp/debugger' }}
			checked={enabled}
			on:change={toggleEnabled}
		/>
	</div>

	{#if enabled}
		<TextInput
			inputProps={{
				type: 'text',
				id: 'ws_base_url',
				placeholder: 'wss://ws.example.com'
			}}
			bind:value={$values['ws_base_url']}
			class="max-w-lg"
		/>
		{@const val = $values['ws_base_url']}
		{#if val && (!val.startsWith('ws') || !val.includes('://') || val.endsWith('/') || val.endsWith(' '))}
			<span class="text-red-600 dark:text-red-400 text-xs">
				Must start with ws:// or wss:// and not end with / or a space
			</span>
		{/if}
	{/if}
</div>
