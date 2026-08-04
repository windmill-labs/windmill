<script lang="ts">
	import { base } from '$lib/base'
	import { userStore } from '$lib/stores'
	import Alert from '$lib/components/common/alert/Alert.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import DbtIcon from '$lib/components/icons/DbtIcon.svelte'
	import { BookOpen } from 'lucide-svelte'

	const DBT_DOCS = 'https://www.windmill.dev/docs/getting_started/scripts_quickstart/dbt'
</script>

<Alert type="info" title="Already using dbt? Bring the project as it is" size="xs">
	<div class="flex flex-col gap-2">
		<p>
			An unmodified dbt project runs here as a script of its own, and its models land on this graph
			as <span class="font-mono">dbt://</span> assets that native pipeline steps read and write like
			any other, so you can move one model at a time instead of rewriting the project. All it needs
			is a warehouse configured under
			<a class="underline" href="{base}/workspace_settings?tab=dbt">Settings → dbt</a>, which the
			project names by name.
		</p>
		<div class="flex flex-row flex-wrap items-center gap-2">
			{#if !$userStore?.operator}
				<!-- The dbt mark goes in as a child rather than `startIcon`: that slot
				     sizes its icon with lucide's `size`, which DbtIcon has no prop for. -->
				<Button variant="default" unifiedSize="sm" href="{base}/scripts/add?lang=dbt">
					<DbtIcon width={14} height={14} />
					New dbt script
				</Button>
			{/if}
			<Button
				variant="subtle"
				unifiedSize="sm"
				href={DBT_DOCS}
				target="_blank"
				rel="noreferrer"
				startIcon={{ icon: BookOpen }}
			>
				dbt documentation
			</Button>
		</div>
	</div>
</Alert>
