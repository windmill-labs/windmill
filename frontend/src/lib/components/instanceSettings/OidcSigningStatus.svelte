<script lang="ts">
	import { OidcService } from '$lib/gen'
	import { Alert, Badge, CopyButton } from '$lib/components/common'
	import DataTable from '$lib/components/table/DataTable.svelte'
	import Head from '$lib/components/table/Head.svelte'
	import Cell from '$lib/components/table/Cell.svelte'
	import { displayDate } from '$lib/utils'

	type Status = Awaited<ReturnType<typeof OidcService.getOidcSigningStatus>>

	let status = $state<Status | undefined>(undefined)
	let loadError = $state<string | undefined>(undefined)

	const modeLabels: Record<Status['mode'], string> = {
		database: 'Key stored in the database',
		file: 'Key file (OIDC_SIGNING_KEY_FILE)',
		external: 'External signer (OIDC_SIGNER_URL)'
	}

	$effect(() => {
		OidcService.getOidcSigningStatus()
			.then((s) => {
				status = s
				loadError = undefined
			})
			.catch((e) => {
				loadError = e?.body ?? e?.message ?? String(e)
			})
	})
</script>

{#if loadError}
	<Alert type="error" title="Could not load the OIDC signing status">{loadError}</Alert>
{:else if status}
	<div class="flex flex-col gap-4">
		<div class="text-xs text-primary">
			<span class="text-secondary">Tokens are signed by:</span>
			{modeLabels[status.mode]}
		</div>
		{#if status.signer}
			<div class="flex flex-wrap items-center gap-2 text-xs text-primary">
				<span class="text-secondary">Last key listing:</span>
				{status.signer.last_refresh_success
					? displayDate(status.signer.last_refresh_success)
					: 'never'}
				{#if status.signer.breaker !== 'closed'}
					<Badge color="red">Signer unreachable</Badge>
				{:else if !status.signer.last_refresh_error && status.signer.last_refresh_success}
					<Badge color="green">Signer reachable</Badge>
				{/if}
			</div>
			{#if status.signer.last_refresh_error}
				<Alert type="warning" title="The last key listing was refused">
					{status.signer.last_refresh_error}
				</Alert>
			{/if}
			{#if status.signer.keys_from_store}
				<Alert type="warning" title="The signer has not answered since this server started">
					The keys below are those stored by the last accepted listing. Tokens cannot be signed
					until the signer answers.
				</Alert>
			{/if}
			{#if status.signer.tls_files_error}
				<Alert type="warning" title="The mTLS files on disk are not in use">
					{status.signer.tls_files_error}. The previous certificate and key are kept.
				</Alert>
			{/if}
		{/if}
		<DataTable size="xs">
			<Head>
				<tr>
					<Cell head first>Key ID (kid)</Cell>
					<Cell head>Algorithm</Cell>
					<Cell head>State</Cell>
					<Cell head last>Source</Cell>
				</tr>
			</Head>
			<tbody>
				{#each status.keys as key, i (`${key.source}:${key.kid ?? i}`)}
					<tr>
						<Cell first>
							<span class="flex items-center gap-1 font-mono text-xs">
								{key.kid}
								{#if key.kid}<CopyButton value={key.kid} />{/if}
							</span>
						</Cell>
						<Cell>{key.alg}</Cell>
						<Cell>
							<Badge color={key.state === 'active' ? 'green' : 'gray'}>{key.state}</Badge>
						</Cell>
						<Cell last>{key.source}</Cell>
					</tr>
				{/each}
			</tbody>
		</DataTable>
		{#if status.keys.length === 0}
			<div class="text-xs text-secondary">The JWKS serves no key.</div>
		{/if}
	</div>
{/if}
