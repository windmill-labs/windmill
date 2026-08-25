<script lang="ts">
	import Alert from '../common/alert/Alert.svelte'
	import type { TestDataTableConnectionResponse } from '$lib/gen'

	type Props = {
		/** What the report is about: a data table, a Supabase project, a database name. */
		name: string
		report?: TestDataTableConnectionResponse | undefined
		error?: string | undefined
		bgClass?: string
		class?: string
	}

	let { name, report, error, bgClass, class: className }: Props = $props()

	let fullyPrivileged = $derived(!!report?.can_create_table && !!report?.can_create_schema)
</script>

{#if error}
	<Alert type="error" title="Could not connect to {name}" size="xs" {bgClass} class={className}>
		{error}
	</Alert>
{:else if report}
	<Alert
		type={fullyPrivileged ? 'success' : 'warning'}
		title={fullyPrivileged
			? `${name} is reachable and its user can create tables and schemas`
			: `${name} is reachable but its user is missing privileges`}
		size="xs"
		{bgClass}
		class={className}
	>
		<div class="flex flex-col gap-2">
			<div>
				Connects as <span class="font-mono">{report.user}</span>{#if report.schema}, resolving
					unqualified statements to schema <span class="font-mono">{report.schema}</span>{/if}.
			</div>
			{#if report.suggested_search_path}
				<div>
					Its search_path resolves to no schema, so unqualified statements fail with
					<span class="font-mono">no schema has been selected to create in</span> whatever
					privileges the role holds. Point it at one, e.g.
					<span class="font-mono select-all">{report.suggested_search_path}</span>.
				</div>
			{/if}
			<ul class="list-disc list-inside">
				<li>
					Create tables{report.schema ? ` in ${report.schema}` : ''}:
					<span class="font-semibold">{report.can_create_table ? 'yes' : 'no'}</span>
				</li>
				<li>
					Create schemas:
					<span class="font-semibold">{report.can_create_schema ? 'yes' : 'no'}</span>
				</li>
				<li>
					Migration bookkeeping table exists:
					<span class="font-semibold">{report.migrations_table_exists ? 'yes' : 'no'}</span>
				</li>
			</ul>
			{#if report.suggested_grants.length > 0}
				<div>
					Windmill connects as the role that lacks these privileges, so it cannot grant them itself.
					Run as a schema owner or superuser on that database:
				</div>
				<pre class="whitespace-pre-wrap select-all text-xs"
					>{report.suggested_grants.map((g) => `${g};`).join('\n')}</pre
				>
				{#if report.schema && !report.can_create_table && !report.migrations_table_exists}
					<div>
						Alternatively, create the <span class="font-mono">_wm_migrations</span> bookkeeping table
						yourself and grant only SELECT, INSERT, UPDATE, DELETE on it.
					</div>
				{/if}
			{/if}
		</div>
	</Alert>
{/if}
