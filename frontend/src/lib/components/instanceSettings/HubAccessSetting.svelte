<script lang="ts">
	import type { Writable } from 'svelte/store'
	import { Alert, Button } from '../common'
	import SettingCard from './SettingCard.svelte'
	import HubAccessEditor from '../settings/HubAccessEditor.svelte'
	import { HUB_ACCESS_DESCRIPTION } from '$lib/workspaceRoles'
	import type { HubAccess, HubAccessList } from '$lib/gen'
	import { RotateCcw } from 'lucide-svelte'

	interface Props {
		values: Writable<Record<string, any>>
		/** The settings as saved, without the page's edits. */
		initialValues?: Record<string, any>
		loading?: boolean
		/** Without a valid license the stored list keeps applying but can only be cleared. */
		unlicensed?: boolean
		/** A Community Edition server never applies the instance's list. */
		ceBuild?: boolean
	}

	let {
		values,
		initialValues,
		loading = false,
		unlicensed = false,
		ceBuild = false
	}: Props = $props()

	const KEY = 'hub_access'

	// A list written through the API or YAML is stored as spelled.
	const normalize = (stored: Record<string, HubAccess> | undefined | null): HubAccessList =>
		Object.fromEntries(Object.entries(stored ?? {}).map(([k, v]) => [k.toLowerCase(), v]))
	const list = $derived(normalize($values[KEY]))

	// Unset rather than `{}`, so that removing the last integration is a reset of the setting.
	function set(value: HubAccessList) {
		$values[KEY] = Object.keys(value).length > 0 ? value : undefined
	}
</script>

<SettingCard
	label="Hub scripts"
	description="{HUB_ACCESS_DESCRIPTION} Applies to every workspace that does not replace this list with its own."
	settingKey={KEY}
>
	<div class="flex flex-col gap-3 pt-2">
		{#if unlicensed}
			<Alert
				type="info"
				size="xs"
				title="Changing the instance's Hub scripts requires a valid Windmill Enterprise Edition license"
			>
				{#if ceBuild}
					This Community Edition server does not apply the list below. A workspace admin can still
					set one in that workspace's settings.
				{:else}
					The list below keeps applying as it is. A workspace admin can still replace it in that
					workspace's settings.
				{/if}
				{#if Object.keys(list).length > 0}
					<div class="mt-2">
						<Button
							variant="default"
							unifiedSize="sm"
							startIcon={{ icon: RotateCcw }}
							disabled={loading}
							onclick={() => set({})}
						>
							Clear the list
						</Button>
					</div>
				{/if}
			</Alert>
		{/if}
		<HubAccessEditor
			value={list}
			saved={initialValues ? normalize(initialValues[KEY]) : list}
			readOnly={unlicensed || loading}
			onchange={set}
		/>
	</div>
</SettingCard>
