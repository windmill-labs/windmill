<script lang="ts">
	import { base } from '$lib/base'
	import Alert from '$lib/components/common/alert/Alert.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import DbtIcon from '$lib/components/icons/DbtIcon.svelte'
	import { BookOpen } from 'lucide-svelte'

	const DBT_DOCS = 'https://www.windmill.dev/docs/getting_started/scripts_quickstart/dbt'
</script>

<Alert type="info" title="Prefer using dbt? Windmill also runs dbt" size="xs">
	<div class="flex flex-col gap-2">
		<p>
			A pipeline is Windmill's own abstraction; an existing dbt project stays a dbt project, and
			runs here unchanged as its own kind of script, with dbt still owning its models, refs and
			tests. All it needs is a warehouse configured under
			<a class="underline" href="{base}/workspace_settings?tab=dbt">Settings → dbt</a>. Its models
			show up in the asset graph as <span class="font-mono">dbt://</span> nodes, visible next to whatever
			pipelines you do build, so you can adopt pipelines later, or never.
		</p>
		<div class="flex flex-row flex-wrap items-center gap-2">
			<!-- The dbt mark goes in as a child rather than `startIcon`: that slot
			     sizes its icon with lucide's `size`, which DbtIcon has no prop for. -->
			<Button variant="default" unifiedSize="sm" href="{base}/scripts/add?lang=dbt">
				<DbtIcon width={14} height={14} />
				New dbt script
			</Button>
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
