<script lang="ts">
	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import PageHeader from '$lib/components/PageHeader.svelte'
	import { Tab, Button } from '$lib/components/common'
	import Tabs from '$lib/components/common/tabs/Tabs.svelte'
	import { BookOpen, Users, Workflow } from 'lucide-svelte'
	import { base } from '$lib/base'

	let tab: 'quickstart' | 'team' = $state('quickstart')

	function startFlowTutorial() {
		window.location.href = `${base}/flows/add?tutorial=workspace-onboarding-continue&nodraft=true`
	}
</script>

<CenteredPage>
	<PageHeader
		title="Guides"
		tooltip="Learn how to use Windmill with our comprehensive guides"
		documentationLink="https://www.windmill.dev/docs/intro"
	/>
	<div class="flex justify-between pt-4">
		<Tabs class="w-full" bind:selected={tab}>
			<Tab value="quickstart" label="Quickstart" icon={BookOpen} />
			<Tab value="team" label="Team Collaboration" icon={Users} />
		</Tabs>
	</div>

	{#if tab === 'quickstart'}
		<div class="pt-8">
			<div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
				<button
					onclick={startFlowTutorial}
					class="block border rounded-lg p-6 bg-surface-secondary hover:bg-surface-hover transition-colors text-left"
				>
					<div class="flex items-center gap-3 mb-4">
						<div class="p-3 rounded-lg bg-surface-tertiary">
							<Workflow size={28} class="text-emphasis" />
						</div>
						<h2 class="text-lg font-semibold text-emphasis">Create a simple flow</h2>
					</div>
					<p class="text-sm text-secondary mb-4">
						Learn how to build workflows in Windmill with our interactive step-by-step tutorial.
					</p>
				</button>
			</div>
		</div>
	{:else if tab === 'team'}
		<!-- Team Collaboration content will go here -->
	{/if}
</CenteredPage>
