<script lang="ts">
	import { Section } from '$lib/components/common'
	import SettingsFooter from '$lib/components/workspaceSettings/SettingsFooter.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import HubAccessEditor from './HubAccessEditor.svelte'
	import { HUB_ACCESS_DESCRIPTION } from '$lib/workspaceRoles'
	import { WorkspaceService, type HubAccessList, type WorkspaceRoles } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import { untrack } from 'svelte'

	interface Props {
		workspace: string
		/** The workspace's roles as last read; undefined while loading. */
		data: WorkspaceRoles | undefined
		onSaved: () => void
	}

	let { workspace, data, onSaved }: Props = $props()

	function canonical(list: HubAccessList | null | undefined): string {
		return JSON.stringify(list ? Object.entries(list).sort() : null)
	}

	const stored = $derived(data && canonical(data.hub_access))

	/** `null` follows the instance's list. Reset only when the stored list changed: saving the roles
	 *  reloads `data` too, and must not drop an edit here. */
	let draft: HubAccessList | null = $derived.by(() => {
		void stored
		return untrack(() => data?.hub_access ?? null)
	})
	const dirty = $derived(stored !== undefined && canonical(draft) !== stored)

	async function save() {
		await WorkspaceService.updateWorkspaceHubAccess({
			workspace,
			requestBody: { hub_access: draft }
		})
		sendUserToast('Hub scripts saved')
		onSaved()
	}
</script>

<Section
	label="Hub scripts"
	collapsable={true}
	description="{HUB_ACCESS_DESCRIPTION} This workspace uses the instance's list, or replaces it with its own."
>
	{#snippet action()}
		<SettingsFooter
			inline
			class="w-auto"
			hasUnsavedChanges={dirty}
			onSave={save}
			onDiscard={() => (draft = data?.hub_access ?? null)}
			saveLabel="Save Hub scripts"
		/>
	{/snippet}

	{#if !data}
		<span class="text-xs text-secondary">Loading Hub scripts...</span>
	{:else}
		<div class="flex flex-col gap-3">
			<Toggle
				checked={draft === null}
				options={{ right: 'Use the instance list' }}
				on:change={(e) => (draft = e.detail ? null : { ...(data?.instance_hub_access ?? {}) })}
			/>
			<!-- A fresh editor per mode: a selection kept from editing would otherwise still change
			     the list, and so stop following the instance's. -->
			{#key draft === null}
				<HubAccessEditor
					value={draft ?? data.instance_hub_access}
					saved={data.hub_access ?? data.instance_hub_access}
					readOnly={draft === null}
					onchange={(v) => (draft = v)}
				/>
			{/key}
		</div>
	{/if}
</Section>
