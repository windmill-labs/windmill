<script lang="ts">
	import { workspaceStore } from '$lib/stores'
	import { Button, Drawer, DrawerContent, Tab, TabContent, Tabs } from '$lib/components/common'
	import CreateToken from '$lib/components/settings/CreateToken.svelte'
	import CopyableCodeBlock from '$lib/components/details/CopyableCodeBlock.svelte'
	import { Bot, ExternalLink, Terminal } from 'lucide-svelte'
	import { shell } from 'svelte-highlight/languages'

	type ConnectTab = 'cli' | 'mcp'

	let drawer: Drawer | undefined = $state()
	let selectedTab: ConnectTab = $state('cli')
	let openVersion = $state(0)

	const origin = $derived(typeof window === 'undefined' ? '' : window.location.origin)
	const workspaceId = $derived($workspaceStore ?? '<workspace>')
	const cliCommands = $derived(`npm install -g windmill-cli
wmill workspace add ${workspaceId} ${workspaceId} ${origin}
wmill init
wmill sync pull`)

	function noop() {}

	export function openDrawer(tab: ConnectTab = 'cli') {
		selectedTab = tab
		openVersion += 1
		drawer?.openDrawer()
	}

	function closeDrawer() {
		drawer?.closeDrawer()
	}
</script>

<Drawer bind:this={drawer} size="720px">
	<DrawerContent title="Connect this workspace" on:close={closeDrawer}>
		<div class="flex flex-col gap-5 pb-4">
			<div class="flex flex-col gap-2">
				<div class="w-full">
					<Tabs values={['cli', 'mcp']} bind:selected={selectedTab} wrapperClass="scrollbar-hidden">
						<Tab value="cli" label="CLI" icon={Terminal} />
						<Tab value="mcp" label="MCP" icon={Bot} />
						{#snippet content()}
							<div class="pt-4">
								<TabContent value="cli">
									<div class="flex flex-col gap-4">
										<div class="flex items-start justify-between gap-3 flex-wrap">
											<div class="flex flex-col gap-1">
												<h3 class="text-sm font-semibold text-emphasis">Local setup</h3>
												<p class="text-xs text-secondary max-w-xl">
													Run this in your local repo to bind the current workspace, create
													<code class="rounded bg-surface-secondary px-1 py-0.5 font-mono text-2xs text-emphasis"
														>wmill.yaml</code
													>, and pull the latest files.
												</p>
											</div>

											<Button
												variant="subtle"
												unifiedSize="sm"
												href="https://www.windmill.dev/docs/advanced/cli"
												target="_blank"
												startIcon={{ icon: ExternalLink }}
											>
												CLI docs
											</Button>
										</div>

										<CopyableCodeBlock
											code={cliCommands}
											language={shell}
											wrap
											copyOnClick={false}
										/>

										<p class="text-2xs text-secondary">
											<code class="rounded bg-surface-secondary px-1 py-0.5 font-mono text-2xs text-emphasis"
												>wmill workspace add</code
											>
											will handle authentication,
											<code class="rounded bg-surface-secondary px-1 py-0.5 font-mono text-2xs text-emphasis"
												>wmill init</code
											>
											bootstraps the local config, and
											<code class="rounded bg-surface-secondary px-1 py-0.5 font-mono text-2xs text-emphasis"
												>wmill sync pull</code
											>
											fetches the workspace content.
										</p>
									</div>
								</TabContent>

								<TabContent value="mcp">
									<div class="flex flex-col gap-4">
										<div class="flex items-start justify-between gap-3 flex-wrap">
											<div class="flex flex-col gap-1">
												<h3 class="text-sm font-semibold text-emphasis">MCP URL</h3>
												<p class="text-xs text-secondary max-w-xl">
													Generate an MCP server URL for the current workspace and choose which
													scripts, flows, and endpoints the client can access.
												</p>
											</div>

											<Button
												variant="subtle"
												unifiedSize="sm"
												href="https://www.windmill.dev/docs/core_concepts/mcp"
												target="_blank"
												startIcon={{ icon: ExternalLink }}
											>
												MCP docs
											</Button>
										</div>

										{#key openVersion}
											<CreateToken
												mcpOnly
												lockWorkspace
												title="Generate MCP URL"
												defaultNewTokenWorkspace={$workspaceStore}
												onTokenCreated={noop}
											/>
										{/key}
									</div>
								</TabContent>
							</div>
						{/snippet}
					</Tabs>
				</div>
			</div>
		</div>
	</DrawerContent>
</Drawer>
