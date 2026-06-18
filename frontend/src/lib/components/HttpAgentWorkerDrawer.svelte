<script lang="ts">
	import { AgentWorkersService, type ListBlacklistedAgentTokensResponse } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import { ExternalLink, RefreshCw, Trash } from 'lucide-svelte'
	import { Alert, Button, Tab, Tabs } from './common'
	import Section from './Section.svelte'
	import TagsToListenTo from './TagsToListenTo.svelte'
	import { enterpriseLicense, superadmin } from '$lib/stores'
	import Label from './Label.svelte'
	import TextInput from './text_input/TextInput.svelte'
	import CopyableCodeBlock from './details/CopyableCodeBlock.svelte'
	import { shell, json } from 'svelte-highlight/languages'
	import TokenDisplay from './settings/TokenDisplay.svelte'
	import Description from './Description.svelte'
	import { defaultTags, nativeTags } from './worker_group'

	type Props = {
		customTags: string[] | undefined
	}
	let { customTags = $bindable() }: Props = $props()
	let selectedTags: string[] = $state(!$enterpriseLicense ? ['agent_test'] : [])
	let workerGroup: string = $state('agent')
	let token: string = $state('')
	let blacklistToken: string = $state('')
	let blacklistTokenError: string = $state('')
	let selectedTab: 'create' | 'blacklist' = $state('create')
	let blacklistedTokens: ListBlacklistedAgentTokensResponse | undefined = $state(undefined)
	let isLoadingBlacklist: boolean = $state(false)
	let isGeneratingToken: boolean = $state(false)

	async function generateToken() {
		isGeneratingToken = true
		try {
			const newToken = await AgentWorkersService.createAgentToken({
				requestBody: {
					worker_group: workerGroup,
					tags: selectedTags,
					exp: Math.floor(Date.now() / 1000) + 60 * 60 * 24 * 365 * 3 // 3 years
				}
			})

			token = newToken
			sendUserToast('JWT token generated successfully')
		} catch (error) {
			sendUserToast('Error creating agent token: ' + error.toString(), true)
		} finally {
			isGeneratingToken = false
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

	function validateBlacklistToken(token: string) {
		if (!blacklistToken.trim()) {
			blacklistTokenError = 'Token cannot be empty'
		} else if (token && !token.startsWith('jwt_agent_')) {
			blacklistTokenError = 'Token must start with jwt_agent_'
		} else {
			blacklistTokenError = ''
		}
	}

	async function addToBlacklist() {
		if (!blacklistToken.trim()) {
			sendUserToast('Please enter a token to blacklist', true)
			return
		}

		if (blacklistTokenError) {
			sendUserToast('Invalid token format', true)
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
			blacklistTokenError = ''
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
		if (selectedTab === 'blacklist' && $enterpriseLicense && $superadmin) {
			loadBlacklistedTokens()
		}
	})
</script>

<Tabs bind:selected={selectedTab}>
	<Tab value="create" label="Create" />
	<Tab value="blacklist" label="Blacklist" />
	{#snippet content()}
		<div class="flex flex-col gap-y-6 pt-2">
			{#if selectedTab === 'create'}
				<Description
					><a href="https://www.windmill.dev/docs/core_concepts/agent_workers" target="_blank"
						>Agent workers <ExternalLink size={12} class="inline-block" /></a
					> can be used to run jobs with remote workers with unreliable connectivity, workers behind
					firewalls (HTTP-only), untrusted environments (no database access), or large deployments (thousands
					of workers). They have more latency than normal workers. Follow the steps below to create an
					agent worker.</Description
				>

				<Section
					label="1. Generate an agent worker token"
					class="flex flex-col gap-y-6"
					description="Generate a JWT token to authenticate the agent worker."
				>
					<Label
						label="Worker group"
						tooltip="This is only used to give a name prefix to the agent worker and to group workers in the workers page, no worker group config is passed to an agent worker."
					>
						<input class="max-w-md" type="text" bind:value={workerGroup} />
					</Label>
					<Label
						label="Tags to listen to"
						eeOnly
						tooltip="Tags determine which jobs this worker can execute. They are encoded in the JWT token and cannot be changed by the worker. You can use dynamic tags like 'tag-$args[argName]' or 'tag-$workspace' to target different workers based on job arguments or workspace."
					>
						{#if !$enterpriseLicense}
							<div class="text-xs text-secondary mb-2 max-w-md">
								Agent workers are only available in the enterprise edition. For evaluation purposes,
								you can only use the `agent_test` tag and it is limited to 100 jobs.
							</div>
						{/if}
						<div class="flex flex-row gap-2 w-full">
							<TagsToListenTo
								class="grow min-w-0"
								disabled={!$enterpriseLicense}
								bind:worker_tags={selectedTags}
								{customTags}
							/>
							<Button
								variant="default"
								unifiedSize="md"
								onclick={() => {
									selectedTags = [...defaultTags, ...nativeTags, ...(customTags ?? [])]
								}}>Add all tags</Button
							>
						</div>
					</Label>

					{#if !token}
						<div class="mb-4">
							<Button
								variant="accent"
								unifiedSize="md"
								disabled={selectedTags.length === 0 || !$superadmin || isGeneratingToken}
								onclick={generateToken}
								loading={isGeneratingToken}
							>
								{isGeneratingToken ? 'Generating...' : 'Generate token'}
							</Button>
							{#if selectedTags.length === 0}
								<div class="text-xs text-secondary mt-2">
									Please select at least one tag to generate a token.
								</div>
							{:else if !$superadmin}
								<div class="text-xs text-secondary mt-2">
									Only superadmins can generate JWT tokens.
								</div>
							{/if}
						</div>
					{:else}
						<TokenDisplay
							{token}
							title="JWT Token Generated Successfully"
							onClose={() => {
								token = ''
							}}
						/>
					{/if}
				</Section>

				<Section label="2. Create an agent worker" class="flex flex-col gap-y-2">
					<p class="text-xs text-primary">
						Set these environment variables for your agent worker.
					</p>
					<CopyableCodeBlock
						code={`MODE=agent
AGENT_TOKEN=<token>
BASE_INTERNAL_URL=<base url>
`}
						language={shell}
					/>
					<p class="text-2xs text-secondary">
						BASE_INTERNAL_URL: Base URL without trailing slash (e.g.,
						<code>http://windmill.example.com</code>). Can be same as BASE_URL or private network
						URL. <code>INIT_SCRIPT</code> can be passed as env variable if needed.
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

					<div class="mt-2"></div>
					<Section small collapsable label="Automate JWT token generation">
						<div class="text-xs text-primary">
							<p class="mb-2">
								Generate tokens programmatically using this endpoint with superadmin bearer token:
							</p>
							<code class="block mt-1 mb-2">POST /api/agent_workers/create_agent_token</code>
							<p class="mb-2">Request body:</p>
							<CopyableCodeBlock
								code={`{
  "worker_group": "agent",
  "tags": ["tag1", "tag2"],
  "exp": 1717334400
}`}
								language={json}
							/>

							<p class="mt-2">
								<code>exp</code> is Unix timestamp. Response contains the JWT token.
							</p>
						</div>
					</Section>
				</Section>
			{:else if selectedTab === 'blacklist'}
				<div class="flex flex-col gap-y-4">
					<Section label="Agent Token Blacklist" eeOnly>
						{#if !$enterpriseLicense}
							<div class="text-xs text-secondary mb-2 max-w-md">
								Token blacklist management is only available in the enterprise edition.
							</div>
						{:else}
							<div class="text-xs text-secondary mb-4 max-w-md">
								Revoke tokens to prevent agent workers from authenticating. Blacklisted tokens may
								take up to 5 minutes to be effective because of caching.
							</div>

							<div class="flex flex-col gap-3 w-full mb-6">
								<Label
									label="Token"
									for="blacklistTokenInput"
									tooltip="Blacklisted tokens cannot be used by agent workers to authenticate. Useful for revoking compromised tokens or decommissioning workers."
								>
									<div class="flex gap-2">
										<TextInput
											size="md"
											inputProps={{
												id: 'blacklistTokenInput',
												placeholder:
													'jwt_agent_eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJ3b3JrZXJfZ3JvdXAiOiJhZ2VudCIsInN1ZmZpeCI6bnVsbCwidGFncyI6WyJiYXNoIl0sImV4cCI6MTg0NDk1NDYxMX0.JQWb-_ERGaomukbl_cEPPmmCAEepTR79d9oIrKREscE',
												type: 'text',
												disabled: !$superadmin,
												oninput: (e) =>
													validateBlacklistToken((e.target as HTMLInputElement)?.value ?? '')
											}}
											bind:value={blacklistToken}
											error={blacklistTokenError}
										/>
										<Button
											variant="accent"
											unifiedSize="md"
											on:click={addToBlacklist}
											disabled={!$superadmin || blacklistTokenError !== ''}>Blacklist</Button
										>
									</div>

									{#if blacklistTokenError !== ''}
										<div class="text-xs text-red-600">
											{blacklistTokenError}
										</div>
									{/if}

									{#if !$superadmin}
										<div class="text-xs text-amber-600">
											Only superadmins can manage the token blacklist.
										</div>
									{/if}
								</Label>
							</div>

							<!-- Blacklisted Tokens List -->
							<div class="pt-6">
								<div class="flex items-center justify-between mb-2">
									<h3 class="text-xs text-primary">Blacklisted tokens</h3>
									<Button
										variant="subtle"
										unifiedSize="sm"
										on:click={loadBlacklistedTokens}
										disabled={isLoadingBlacklist}
										title="Refresh blacklist"
										startIcon={{ icon: RefreshCw }}
										iconProps={{ class: isLoadingBlacklist ? 'animate-spin' : '' }}
									/>
								</div>

								{#if isLoadingBlacklist}
									<div class="text-center py-4 text-xs text-secondary">
										Loading blacklisted tokens...
									</div>
								{:else if blacklistedTokens?.length === 0}
									<div class="text-center py-4 text-xs text-secondary">
										No tokens are currently blacklisted.
									</div>
								{:else}
									<div class="space-y-2">
										{#each blacklistedTokens ?? [] as blacklistedToken (blacklistedToken.token)}
											<div
												class="flex items-center justify-between p-3 surface-tertiary rounded-lg border border-light"
											>
												<div class="flex-1 min-w-0">
													<div class="font-mono text-2xs text-emphasis pr-4 break-all">
														{blacklistedToken.token}
													</div>
													{#if blacklistedToken.expires_at}
														<div class="text-2xs text-secondary mt-1">
															Expires: {new Date(blacklistedToken.expires_at).toLocaleString()}
														</div>
													{/if}
												</div>
												{#if $superadmin}
													<Button
														variant="subtle"
														destructive
														unifiedSize="sm"
														on:click={() => removeFromBlacklist(blacklistedToken.token)}
														title="Remove from blacklist"
														startIcon={{ icon: Trash }}
													/>
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
