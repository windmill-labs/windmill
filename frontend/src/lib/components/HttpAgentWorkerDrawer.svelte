<script lang="ts">
	import { AgentWorkersService } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import { Copy } from 'lucide-svelte'
	import { Alert } from './common'
	import Section from './Section.svelte'
	import TagsToListenTo from './TagsToListenTo.svelte'
	import { enterpriseLicense, superadmin } from '$lib/stores'
	import CollapseLink from './CollapseLink.svelte'

	export let customTags: string[] | undefined
	let selectedTags: string[] = []

	let workerGroup: string = 'agent'

	let token: string = ''

	$: selectedTags &&
		selectedTags.length > 0 &&
		$enterpriseLicense &&
		$superadmin &&
		workerGroup &&
		refreshToken()

	async function refreshToken() {
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
	<Section label="Tags to listen to">
		<TagsToListenTo bind:worker_tags={selectedTags} {customTags} />
	</Section>
	<Section label="Generated JWT token" primary eeOnly>
		<div class="relative max-w-md">
			<input
				on:click|preventDefault|stopPropagation|capture={() => {
					navigator.clipboard.writeText(token)
					sendUserToast('Copied to clipboard')
				}}
				placeholder="Select tags to generate a jwt token"
				type="text"
				disabled
				value={token}
				class="pr-8 text-sm text-secondary"
			/>
			<button
				class="absolute right-2 top-1/2 -translate-y-1/2 text-primary"
				on:click|preventDefault|stopPropagation={() => {
					navigator.clipboard.writeText(token)
					sendUserToast('Copied to clipboard')
				}}
			>
				<Copy size={16} />
			</button>
		</div>

		<div class="text-xs text-secondary mt-1 mb-12">
			Pass MODE=agent and AGENT_TOKEN={'"<token above>"'} to a worker to have it act as an HTTP agent
			worker. INIT_SCRIPT, if needed, must be passed as an env variable.
		</div>

		<CollapseLink text="Automate JWT token generation" small>
			<div class="text-xs text-secondary">
				Use the following api endpoint with a superadmin bearer token:
				<code class="text-primary"> POST /api/agent_workers/create_agent_token </code>
				with body:
				<pre>
			<code class="text-primary">
{`{
	"worker_group": "agent",
	"tags": ["tag1", "tag2"],
	"exp": 1717334400 // 3 years from now
}`}
					</code>
				</pre>
				JSON response will be the JWT token.
			</div>
		</CollapseLink>
	</Section>
</div>
