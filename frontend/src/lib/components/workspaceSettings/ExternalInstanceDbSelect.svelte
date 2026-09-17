<script lang="ts">
	import { SettingService, type CustomInstanceDbTag } from '$lib/gen'
	import { resource } from 'runed'
	import Select from '../select/Select.svelte'
	import { safeSelectItems } from '../select/utils.svelte'
	import Button from '../common/button/Button.svelte'
	import { sendUserToast } from '$lib/toast'
	import { isExternalInstanceDbEnabled } from './utils.svelte'
	import { Plus } from 'lucide-svelte'

	type Props = {
		value: string | undefined
		tag: CustomInstanceDbTag
		class?: string
	}
	let { value = $bindable(), tag, class: className }: Props = $props()

	let refreshKey = $state(0)
	const databases = resource(
		() => refreshKey,
		async () => {
			try {
				return await SettingService.listExternalInstancePgDatabases()
			} catch {
				return {}
			}
		}
	)

	// Every database Windmill created is offered, whatever it was created for: the tag only
	// sorts the ones made for this kind of storage first.
	let items = $derived(
		safeSelectItems(
			Object.entries(databases.current ?? {})
				.sort(([, a], [, b]) => Number(b.tag === tag) - Number(a.tag === tag))
				.map(([name]) => name)
		)
	)
	let exists = $derived(!!value && !!databases.current?.[value])
	let creating = $state(false)

	async function create() {
		if (!value) return
		creating = true
		try {
			await SettingService.createExternalInstancePgDatabase({
				name: value,
				requestBody: { tag }
			})
			sendUserToast(`Created database ${value} on the external cluster`)
		} catch (e) {
			sendUserToast(e?.body ?? e?.message ?? String(e), true)
		} finally {
			creating = false
			refreshKey++
		}
	}
</script>

<div class="flex items-center gap-1 {className}">
	<Select
		class="flex-1"
		bind:value
		onCreateItem={(i) => (value = i)}
		placeholder="Search or create..."
		showPlaceholderOnOpen
		{items}
		id="external-instance-db-select"
		disabled={!$isExternalInstanceDbEnabled}
	/>
	{#if value && !databases.loading && !exists}
		<Button
			unifiedSize="sm"
			variant="default"
			startIcon={{ icon: Plus }}
			loading={creating}
			disabled={!$isExternalInstanceDbEnabled}
			title="Create this database on the external cluster"
			onclick={create}
		>
			Create
		</Button>
	{/if}
</div>
