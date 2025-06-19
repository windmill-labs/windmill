<script lang="ts">
	import { AgentWorkersService } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import { Copy } from 'lucide-svelte'
	import { Alert } from './common'
	import Section from './Section.svelte'
	import TagsToListenTo from './TagsToListenTo.svelte'
	import { enterpriseLicense, superadmin } from '$lib/stores'
	import CollapseLink from './CollapseLink.svelte'

	type Props = {
		customTags: string[] | undefined
	}
	let { customTags = $bindable() }: Props = $props()
	let selectedTags: string[] = $state(!$enterpriseLicense ? ['agent_test'] : [])
	let workerGroup: string = $state('agent')
	let token: string = $state('')
	let blacklistToken: string = $state('')
	let blacklistExpiry: string = $state('')

	async function refreshToken(workerGroup: string, selectedTags: string[]) {
		try {
			const newToken = await AgentWorkersService.createAgentToken({
				requestBody: {
					worker_group: workerGroup,
					tags: selectedTags,
					exp: Math.floor(Date.now() / 1000) + 60 * 60 * 24 * 365 * 3 // 3 years
				}
			})

			token = newToken
		} catch (error) {
			sendUserToast('Error creating agent token: ' + error.toString(), true)
		}
	}

	async function addToBlacklist() {
		if (!blacklistToken.trim()) {
			sendUserToast('Please enter a token to blacklist', true)
			return
		}

		try {
			const requestBody: { token: string; expires_at?: string } = {
				token: blacklistToken.trim()
			}
			
			// Only include expires_at if a date is provided
			if (blacklistExpiry) {
				requestBody.expires_at = new Date(blacklistExpiry).toISOString()
			}
			
			const response = await fetch('/api/agent_workers/blacklist_token', {
				method: 'POST',
				headers: {
					'Content-Type': 'application/json'
				},
				body: JSON.stringify(requestBody)
			})

			if (!response.ok) {
				const errorText = await response.text()
				throw new Error(errorText || 'Failed to blacklist token')
			}

			sendUserToast('Token successfully added to blacklist')
			blacklistToken = ''
			blacklistExpiry = ''
		} catch (error) {
			sendUserToast('Error blacklisting token: ' + error.toString(), true)
		}
	}

	$effect(() => {
		if (selectedTags.length > 0 && $superadmin) {
			refreshToken(workerGroup, selectedTags)
		}
	})
</script>

<div class="flex flex-col gap-y-4">
	<Alert type="info" title="HTTP agent workers "
		>Use HTTP agent workers only when the workers need to be deployed remotely OR with only HTTP
		connectivity OR in untrusted environments. HTTP agent workers have more latency and less
		capabilities than normal workers.</Alert
	>
	<Section
		label="Worker group"
		tooltip="This is only used to give a name prefix to the agent worker and to group workers in the workers page, no worker group config is passed to an agent worker."
	>
		<input class="max-w-md" type="text" bind:value={workerGroup} />
	</Section>
	<Section label="Tags to listen to" eeOnly>
		{#if !$enterpriseLicense}
			<div class="text-sm text-secondary mb-2 max-w-md">
				Agent workers are only available in the enterprise edition. For evaluation purposes, you can
				only use the tag `agent_test` tag and it is limited to 100 jobs.
			</div>
		{/if}
		<TagsToListenTo disabled={!$enterpriseLicense} bind:worker_tags={selectedTags} {customTags} />
	</Section>

	<Section label="Generated JWT token" primary>
		{#if !$enterpriseLicense}
			<div class="text-sm text-secondary mb-2 max-w-md">
				Agent workers are only available in the enterprise edition. For evaluation purposes, you can
				only use the tag `agent_test` tag and it is limited to 100 jobs.
			</div>
		{/if}
		<div class="relative max-w-md group">
			<input
				onclick={(e) => {
					e.preventDefault()
					e.stopPropagation()
					if (token) {
						navigator.clipboard.writeText(token)
						sendUserToast('Copied to clipboard')
					}
				}}
				placeholder="Select tags to generate a JWT token"
				type="text"
				disabled
				value={token}
				class="w-full pr-10 pl-3 py-2 text-sm text-gray-600 bg-gray-50 border border-gray-300 rounded-lg cursor-pointer hover:bg-gray-100 transition truncate"
			/>

			<button
				class="absolute right-2 top-1/2 -translate-y-1/2 text-gray-500 group-hover:text-blue-600 hover:scale-105 transition"
				aria-label="Copy token to clipboard"
				onclick={(e) => {
					e.preventDefault()
					e.stopPropagation()
					if (token) {
						navigator.clipboard.writeText(token)
						sendUserToast('Copied to clipboard')
					}
				}}
			>
				<Copy size={18} />
			</button>
		</div>

		<div class="flex flex-col gap-2 text-sm mt-3 leading-relaxed">
			Set the following environment variables:
			<ul class="list-disc list-inside mt-1">
				<li><code>MODE=agent</code></li>
				<li><code>AGENT_TOKEN=&lt;token&gt;</code></li>
				<li><code>BASE_INTERNAL_URL=&lt;base url&gt;</code></li>
			</ul>
			<p class="text-sm leading-relaxed">
				to a worker to have it act as an HTTP agent worker.
				<code>INIT_SCRIPT</code>, if needed, must be passed as an env variable.
			</p>
			<Alert type="warning" size="sm" title="Agent Worker Limitations">
				Ensure at least one normal worker is running and listening to the tags
				<code>flow</code> and <code>dependency</code>
				(or <code>flow-&lt;workspace&gt;</code> and <code>dependency-&lt;workspace&gt;</code> if
				using workspace-specific default tags), because agent workers
				<strong>cannot run dependency jobs</strong>
				nor execute the
				<strong>flow state machine</strong>. They can, however, run subjobs within flows.
			</Alert>
			<CollapseLink text="Automate JWT token generation" small>
				<div class="text-xs mt-2">
					Use the following API endpoint with a superadmin bearer token:
					<code class="block mt-1 mb-2">POST /api/agent_workers/create_agent_token</code>
					<pre class=" p-2 rounded-lg text-xs overflow-auto">
	<code
							>{`
	  "worker_group": "agent",
	  "tags": ["tag1", "tag2"],
	  "exp": 1717334400
	`}</code
						>
					</pre>
					The JSON response will contain the generated JWT token.
				</div>
			</CollapseLink>
		</div>
	</Section>

	<Section label="Token Blacklist Management" eeOnly>
		{#if !$enterpriseLicense}
			<div class="text-sm text-secondary mb-2 max-w-md">
				Token blacklist management is only available in the enterprise edition.
			</div>
		{:else}
			<div class="text-sm text-secondary mb-4 max-w-md">
				Add tokens to the blacklist to prevent them from being used by agent workers. Blacklisted tokens are cached for 5 minutes for optimal performance.
			</div>
			
			<div class="flex flex-col gap-3 max-w-md">
				<div>
					<label class="block text-sm font-medium mb-1">Token to blacklist</label>
					<input 
						class="w-full" 
						type="text" 
						bind:value={blacklistToken} 
						placeholder="Enter token (with or without jwt_agent_ prefix)"
					/>
				</div>
				
				<div>
					<label class="block text-sm font-medium mb-1">Blacklist expires on (optional)</label>
					<input 
						class="w-full" 
						type="datetime-local" 
						bind:value={blacklistExpiry}
					/>
					<div class="text-xs text-secondary mt-1">
						If not specified, expiration will be extracted from the JWT token's 'exp' field. If extraction fails, defaults to 1 year from now.
					</div>
				</div>
				
				<button 
					class="px-4 py-2 bg-red-600 text-white rounded-lg hover:bg-red-700 transition disabled:opacity-50 disabled:cursor-not-allowed"
					onclick={addToBlacklist}
					disabled={!$superadmin}
				>
					Add to Blacklist
				</button>
				
				{#if !$superadmin}
					<div class="text-xs text-amber-600">
						Only superadmins can manage the token blacklist.
					</div>
				{/if}
			</div>
		{/if}
	</Section>
</div>
