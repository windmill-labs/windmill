<script lang="ts">
	import { AgentWorkersService, type ListBlacklistedAgentTokensResponse } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import { Copy, Trash2, RefreshCw } from 'lucide-svelte'
	import { Alert, Button, Tab, Tabs } from './common'
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
	let selectedTab: 'create' | 'blacklist' = $state('create')
	let blacklistedTokens: ListBlacklistedAgentTokensResponse | undefined = $state(undefined)
	let isLoadingBlacklist: boolean = $state(false)

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

	async function loadBlacklistedTokens() {
		isLoadingBlacklist = true
		try {
			blacklistedTokens = await AgentWorkersService.listBlacklistedAgentTokens({
				includeExpired: true
			})
		} catch (error) {
			sendUserToast('Error loading blacklisted tokens: ' + error.toString(), true)
			blacklistedTokens = []
		} finally {
			isLoadingBlacklist = false
		}
	}

	async function addToBlacklist() {
		if (!blacklistToken.trim()) {
			sendUserToast('Please enter a token to blacklist', true)
			return
		}

		try {
			await AgentWorkersService.blacklistAgentToken({
				requestBody: {
					token: blacklistToken
				}
			})

			sendUserToast('Token successfully added to blacklist')
			blacklistToken = ''
			// Refresh the blacklist after adding a new token
			await loadBlacklistedTokens()
		} catch (error) {
			sendUserToast('Error blacklisting token: ' + error.body, true)
		}
	}

	async function removeFromBlacklist(tokenToRemove: string) {
		try {
			const response = await fetch('/api/agent_workers/remove_blacklist_token', {
				method: 'POST',
				headers: {
					'Content-Type': 'application/json'
				},
				body: JSON.stringify({ token: tokenToRemove })
			})

			if (!response.ok) {
				const errorText = await response.text()
				throw new Error(errorText || 'Failed to remove token from blacklist')
			}

			sendUserToast('Token successfully removed from blacklist')
			// Refresh the blacklist after removing a token
			await loadBlacklistedTokens()
		} catch (error) {
			sendUserToast('Error removing token from blacklist: ' + error.toString(), true)
		}
	}

	$effect(() => {
		if (selectedTags.length > 0 && $superadmin) {
			refreshToken(workerGroup, selectedTags)
		}
	})

	$effect(() => {
		if (selectedTab === 'blacklist' && $enterpriseLicense && $superadmin) {
			loadBlacklistedTokens()
		}
	})
</script>

<Tabs bind:selected={selectedTab}>
	<Tab value="create">Create</Tab>
	<Tab value="blacklist">Blacklist</Tab>
	{#snippet content()}
		<div class="flex flex-col gap-y-4 pt-2">
			{#if selectedTab === 'create'}
				<Alert type="info" title="HTTP agent workers "
					>Use HTTP agent workers only when the workers need to be deployed remotely OR with only
					HTTP connectivity OR in untrusted environments. HTTP agent workers have more latency and
					less capabilities than normal workers.</Alert
				>
				<div class="flex flex-col gap-y-4 mt-4">
					<Section
						label="Worker group"
						tooltip="This is only used to give a name prefix to the agent worker and to group workers in the workers page, no worker group config is passed to an agent worker."
					>
						<input class="max-w-md" type="text" bind:value={workerGroup} />
					</Section>
					<Section label="Tags to listen to" eeOnly>
						{#if !$enterpriseLicense}
							<div class="text-sm text-secondary mb-2 max-w-md">
								Agent workers are only available in the enterprise edition. For evaluation purposes,
								you can only use the tag `agent_test` tag and it is limited to 100 jobs.
							</div>
						{/if}
						<TagsToListenTo
							disabled={!$enterpriseLicense}
							bind:worker_tags={selectedTags}
							{customTags}
						/>
					</Section>

					<Section label="Generated JWT token" primary>
						{#if !$enterpriseLicense}
							<div class="text-sm text-secondary mb-2 max-w-md">
								Agent workers are only available in the enterprise edition. For evaluation purposes,
								you can only use the tag `agent_test` tag and it is limited to 100 jobs.
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
								class="w-full pr-10 pl-3 py-2 text-sm text-gray-600 bg-gray-50 border border-gray-300 rounded-lg cursor-pointer hover:bg-gray-100 transition truncatere"
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
								(or <code>flow-&lt;workspace&gt;</code> and
								<code>dependency-&lt;workspace&gt;</code>
								if using workspace-specific default tags), because agent workers
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
				</div>
			{:else if selectedTab === 'blacklist'}
				<div class="flex flex-col gap-y-4 mt-4">
					<Section label="Agent Token Blacklist" eeOnly>
						{#if !$enterpriseLicense}
							<div class="text-sm text-secondary mb-2 max-w-md">
								Token blacklist management is only available in the enterprise edition.
							</div>
						{:else}
							<div class="text-sm text-secondary mb-4 max-w-md">
								Add tokens to the blacklist to prevent them from being used by agent workers.
								Blacklisted tokens may take up to 5 minutes to be effective because of caching.
							</div>

							<div class="flex flex-col gap-3 w-full mb-6">
								<div>
									<label class="block text-sm font-medium mb-1" for="blacklistTokenInput"
										>Token</label
									>
									<input
										id="blacklistTokenInput"
										class="w-full"
										type="text"
										bind:value={blacklistToken}
										placeholder="jwt_agent_eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJ3b3JrZXJfZ3JvdXAiOiJhZ2VudCIsInN1ZmZpeCI6bnVsbCwidGFncyI6WyJiYXNoIl0sImV4cCI6MTg0NDk1NDYxMX0.JQWb-_ERGaomukbl_cEPPmmCAEepTR79d9oIrKREscE"
									/>
								</div>
								<div class="flex">
									<Button color="red" on:click={addToBlacklist} disabled={!$superadmin}
										>Blacklist</Button
									>
								</div>

								{#if !$superadmin}
									<div class="text-xs text-amber-600">
										Only superadmins can manage the token blacklist.
									</div>
								{/if}
							</div>

							<!-- Blacklisted Tokens List -->
							<div class="border-t pt-6">
								<div class="flex items-center justify-between mb-4">
									<h3 class="text-lg font-medium">Blacklisted Tokens</h3>
									<button
										class="p-2 text-gray-500 hover:text-blue-600 hover:bg-gray-100 rounded-lg transition"
										onclick={loadBlacklistedTokens}
										disabled={isLoadingBlacklist}
										title="Refresh blacklist"
									>
										<RefreshCw size={16} class={isLoadingBlacklist ? 'animate-spin' : ''} />
									</button>
								</div>

								{#if isLoadingBlacklist}
									<div class="text-center py-4 text-gray-500"> Loading blacklisted tokens... </div>
								{:else if blacklistedTokens?.length === 0}
									<div class="text-center py-4 text-gray-500">
										No tokens are currently blacklisted.
									</div>
								{:else}
									<div class="space-y-2">
										{#each blacklistedTokens ?? [] as blacklistedToken}
											<div
												class="flex items-center justify-between p-3 bg-gray-50 rounded-lg border"
											>
												<div class="flex-1 min-w-0">
													<div class="font-mono text-xs text-gray-700 pr-4 break-all">
														{blacklistedToken.token}
													</div>
													{#if blacklistedToken.expires_at}
														<div class="text-xs text-gray-500 mt-1">
															Expires: {new Date(blacklistedToken.expires_at).toLocaleString()}
														</div>
													{/if}
												</div>
												{#if $superadmin}
													<button
														class="ml-3 p-2 text-red-600 hover:text-red-800 hover:bg-red-50 rounded-lg transition"
														onclick={() => removeFromBlacklist(blacklistedToken.token)}
														title="Remove from blacklist"
													>
														<Trash2 size={16} />
													</button>
												{/if}
											</div>
										{/each}
									</div>
								{/if}
							</div>
						{/if}
					</Section>
				</div>
			{/if}
		</div>
	{/snippet}
</Tabs>
