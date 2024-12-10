<script lang="ts">
	import { Alert, Button } from '$lib/components/common'
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import DrawerContent from '$lib/components/common/drawer/DrawerContent.svelte'
	import Path from '$lib/components/Path.svelte'
	import Required from '$lib/components/Required.svelte'
	import ScriptPicker from '$lib/components/ScriptPicker.svelte'
	import { KafkaTriggerService } from '$lib/gen'
	import { usedTriggerKinds, userStore, workspaceStore } from '$lib/stores'
	import { canWrite, emptyString, sendUserToast } from '$lib/utils'
	import { createEventDispatcher } from 'svelte'
	import Section from '$lib/components/Section.svelte'
	import { Loader2, Save } from 'lucide-svelte'
	import Label from '$lib/components/Label.svelte'
	import Toggle from '../Toggle.svelte'
	import KafkaTriggersConfigSection from './KafkaTriggersConfigSection.svelte'

	let drawer: Drawer
	let is_flow: boolean = false
	let initialPath = ''
	let edit = true
	let itemKind: 'flow' | 'script' = 'script'
	let script_path = ''
	let initialScriptPath = ''
	let fixedScriptPath = ''
	let path: string = ''
	let pathError = ''
	let dirtyGroupId = false
	let enabled = false
	let dirtyPath = false
	let can_write = true
	let drawerLoading = true
	let defaultValues: Record<string, any> | undefined = undefined
	let args: Record<string, any> = {}

	const dispatch = createEventDispatcher()

	$: is_flow = itemKind === 'flow'

	export async function openEdit(ePath: string, isFlow: boolean) {
		drawerLoading = true
		try {
			drawer?.openDrawer()
			initialPath = ePath
			itemKind = isFlow ? 'flow' : 'script'
			edit = true
			dirtyPath = false
			dirtyGroupId = false
			await loadTrigger()
		} catch (err) {
			sendUserToast(`Could not load kafka trigger: ${err}`, true)
		} finally {
			drawerLoading = false
		}
	}

	export async function openNew(
		nis_flow: boolean,
		fixedScriptPath_?: string,
		nDefaultValues?: Record<string, any>
	) {
		drawerLoading = true
		try {
			drawer?.openDrawer()
			is_flow = nis_flow
			edit = false
			itemKind = nis_flow ? 'flow' : 'script'
			args.kafka_resource_path = nDefaultValues?.kafka_resource_path ?? ''
			args.group_id = nDefaultValues?.group_id ?? ''
			args.topics = nDefaultValues?.topics ?? ['']
			dirtyGroupId = false
			initialScriptPath = ''
			fixedScriptPath = fixedScriptPath_ ?? ''
			script_path = fixedScriptPath
			path = ''
			initialPath = ''
			dirtyPath = false
			defaultValues = nDefaultValues
		} finally {
			drawerLoading = false
		}
	}

	async function loadTrigger(): Promise<void> {
		const s = await KafkaTriggerService.getKafkaTrigger({
			workspace: $workspaceStore!,
			path: initialPath
		})
		script_path = s.script_path
		initialScriptPath = s.script_path

		is_flow = s.is_flow
		path = s.path
		args.kafka_resource_path = s.kafka_resource_path
		args.group_id = s.group_id
		args.topics = s.topics
		enabled = s.enabled

		can_write = canWrite(s.path, s.extra_perms, $userStore)
	}

	async function updateTrigger(): Promise<void> {
		if (edit) {
			await KafkaTriggerService.updateKafkaTrigger({
				workspace: $workspaceStore!,
				path: initialPath,
				requestBody: {
					path,
					script_path,
					is_flow,
					kafka_resource_path: args.kafka_resource_path,
					group_id: args.group_id,
					topics: args.topics
				}
			})
			sendUserToast(`Kafka trigger ${path} updated`)
		} else {
			await KafkaTriggerService.createKafkaTrigger({
				workspace: $workspaceStore!,
				requestBody: {
					path,
					script_path,
					is_flow,
					enabled: true,
					kafka_resource_path: args.kafka_resource_path,
					group_id: args.group_id,
					topics: args.topics
				}
			})
			sendUserToast(`Kafka trigger ${path} created`)
		}
		if (!$usedTriggerKinds.includes('kafka')) {
			$usedTriggerKinds = [...$usedTriggerKinds, 'kafka']
		}
		dispatch('update')
		drawer.closeDrawer()
	}

	$: topicsError = args.topics?.some((b) => /[^[a-zA-Z0-9-_.]/.test(b)) ? 'Invalid topics' : ''
	$: groupIdError = /[^a-zA-Z0-9-_.]/.test(args.group_id) ? 'Invalid group ID' : ''

	function useDefaultValues() {
		if (args.kafka_resource_path && args.kafka_resource_path != '') {
			return false
		}
		if (!defaultValues) {
			return false
		}
		return (
			defaultValues.brokers &&
			defaultValues.brokers.length > 0 &&
			defaultValues.brokers.some((broker: string) => broker.trim() !== '')
		)
	}
</script>

<Drawer size="800px" bind:this={drawer}>
	<DrawerContent
		title={edit
			? can_write
				? `Edit kafka trigger ${initialPath}`
				: `Kafka trigger ${initialPath}`
			: 'New kafka trigger'}
		on:close={drawer.closeDrawer}
	>
		<svelte:fragment slot="actions">
			{#if !drawerLoading && can_write}
				{#if edit}
					<div class="mr-8 center-center -mt-1">
						<Toggle
							disabled={!can_write}
							checked={enabled}
							options={{ right: 'enable', left: 'disable' }}
							on:change={async (e) => {
								await KafkaTriggerService.setKafkaTriggerEnabled({
									path: initialPath,
									workspace: $workspaceStore ?? '',
									requestBody: { enabled: e.detail }
								})
								sendUserToast(`${e.detail ? 'enabled' : 'disabled'} kafka trigger ${initialPath}`)
							}}
						/>
					</div>
				{/if}
				<Button
					startIcon={{ icon: Save }}
					disabled={pathError != '' ||
						emptyString(script_path) ||
						emptyString(args.kafka_resource_path) ||
						args.topics.length < 1 ||
						args.topics.some((t) => emptyString(t)) ||
						topicsError != '' ||
						emptyString(args.group_id) ||
						groupIdError != '' ||
						!can_write}
					on:click={updateTrigger}
				>
					Save
				</Button>
			{/if}
		</svelte:fragment>
		{#if drawerLoading}
			<Loader2 class="animate-spin" />
		{:else}
			<Alert title="Info" type="info">
				{#if edit}
					Changes can take up to 30 seconds to take effect.
				{:else}
					Kafka consumers can take up to 30 seconds to start.
				{/if}
			</Alert>
			<div class="flex flex-col gap-12 mt-6">
				<div class="flex flex-col gap-4">
					<Label label="Path">
						<Path
							bind:dirty={dirtyPath}
							bind:error={pathError}
							bind:path
							{initialPath}
							checkInitialPathExistence={!edit}
							namePlaceholder="kafka_trigger"
							kind="kafka_trigger"
							disabled={!can_write}
						/>
					</Label>
				</div>

				<KafkaTriggersConfigSection
					bind:args
					defaultValues={useDefaultValues() ? defaultValues : undefined}
				/>

				<Section label="Runnable">
					<p class="text-xs mb-1 text-tertiary">
						Pick a script or flow to be triggered<Required required={true} />
					</p>
					<div class="flex flex-row mb-2">
						<ScriptPicker
							disabled={fixedScriptPath != '' || !can_write}
							initialPath={fixedScriptPath || initialScriptPath}
							kinds={['script']}
							allowFlow={true}
							bind:itemKind
							bind:scriptPath={script_path}
							allowRefresh
						/>
					</div>
				</Section>
			</div>
		{/if}
	</DrawerContent>
</Drawer>
