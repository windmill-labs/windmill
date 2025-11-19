<script lang="ts">
	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import PageHeader from '$lib/components/PageHeader.svelte'
	import { Tab } from '$lib/components/common'
	import Tabs from '$lib/components/common/tabs/Tabs.svelte'
	import { BookOpen, Users } from 'lucide-svelte'

	let tab: 'core' | 'team' = $state('core')
</script>

<CenteredPage>
	<PageHeader
		title="Guides & Documentation"
		tooltip="Learn how to use Windmill with our comprehensive guides and documentation"
		documentationLink="https://www.windmill.dev/docs/intro"
	/>
	<div class="flex justify-between pt-4">
		<Tabs class="w-full" bind:selected={tab}>
			<Tab value="core" label="Core Concepts" icon={BookOpen} />
			<Tab value="team" label="Team Collaboration" icon={Users} />
		</Tabs>
	</div>

	{#if tab === 'core'}
		<!-- Core Concepts content will go here -->
	{:else if tab === 'team'}
		<!-- Team Collaboration content will go here -->
	{/if}
</CenteredPage>
