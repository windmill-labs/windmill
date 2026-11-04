<script lang="ts">
	import { Badge } from './common'

	interface Props {
		external_id?: string | null
		scim_managed?: boolean
	}

	let { external_id, scim_managed = false }: Props = $props()
</script>

<span class="inline-flex items-center gap-1.5 min-w-0 text-xs">
	<span class="truncate" title={external_id ?? undefined}>{external_id ?? '-'}</span>
	{#if scim_managed}
		<Badge color="blue" verySmall>SCIM</Badge>
	{/if}
</span>
