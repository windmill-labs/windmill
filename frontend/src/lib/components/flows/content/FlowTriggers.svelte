<script lang="ts">
	import Tab from '$lib/components/common/tabs/Tab.svelte'
	import { Tabs } from '$lib/components/common'
	import WebhooksPanel from '$lib/components/details/WebhooksPanel.svelte'
	import EmailTriggerPanel from '$lib/components/details/EmailTriggerPanel.svelte'
	import RoutesPanel from '$lib/components/triggers/RoutesPanel.svelte'
	import RunPageSchedules from '$lib/components/RunPageSchedules.svelte'
	import { HttpTriggerService, ScheduleService, type HttpTrigger, type Schedule } from '$lib/gen'
	import { canWrite } from '$lib/utils'
	import { userStore, workspaceStore } from '$lib/stores'
	import FlowCard from '../common/FlowCard.svelte'
	import { onMount, getContext } from 'svelte'
	import type { FlowEditorContext } from '../types'
	let schedules: Schedule[] | undefined = undefined

	export let noEditor: boolean
	export let newFlow = false
	let triggers: (HttpTrigger & { canWrite: boolean })[] | undefined = undefined
	let path = ''

	const { pathStore, selectedTrigger } = getContext<FlowEditorContext>('FlowEditorContext')

	$: path = $pathStore

	async function loadSchedules() {
		if (!path) return
		try {
			schedules = (
				await ScheduleService.listSchedules({
					workspace: $workspaceStore ?? '',
					path: path,
					isFlow: true
				})
			).filter((s) => s.path != path)
		} catch (e) {
			console.error('impossible to load schedules')
		}
	}

	async function loadTriggers() {
		try {
			triggers = (
				await HttpTriggerService.listHttpTriggers({
					workspace: $workspaceStore ?? '',
					path,
					isFlow: true
				})
			).map((x) => {
				return { canWrite: canWrite(x.path, x.extra_perms!, $userStore), ...x }
			})
		} catch (e) {
			console.error('impossible to load http routes')
		}
	}

	onMount(() => {
		if (!newFlow) {
			loadSchedules()
			loadTriggers()
		}
	})

	let selectedTab = $selectedTrigger ?? 'webhooks'
</script>

<FlowCard {noEditor} title="Flow Triggers">
	<div class="pt-4">
		<Tabs bind:selected={selectedTab}>
			<Tab value="webhooks" disabled={newFlow} selectedClass="text-primary font-semibold"
				>Webhooks</Tab
			>
			<Tab value="mail" disabled={newFlow} selectedClass="text-primary text-sm font-semibold"
				>Mail</Tab
			>
			<Tab value="routes" disabled={newFlow} selectedClass="text-primary text-sm font-semibold"
				>Routes</Tab
			>
			<Tab value="schedules" disabled={newFlow} selectedClass="text-primary text-sm font-semibold"
				>Schedules</Tab
			>

			<svelte:fragment slot="content">
				{#if selectedTab === 'webhooks'}
					<div class="p-4">
						<WebhooksPanel scopes={[`run:flow/${path}`]} {path} isFlow={true} args={{}} token="" />
					</div>
				{/if}

				{#if selectedTab === 'mail'}
					<div class="p-4">
						<EmailTriggerPanel token="" scopes={[`run:flow/${path}`]} {path} isFlow={true} />
					</div>
				{/if}

				{#if !newFlow && selectedTab === 'routes'}
					<div class="p-4">
						<RoutesPanel path={path ?? ''} isFlow={true} bind:triggers />
					</div>
				{/if}

				{#if !newFlow && selectedTab === 'schedules'}
					<div class="p-4">
						<RunPageSchedules
							isFlow={true}
							path={path ?? ''}
							can_write={canWrite(path, {}, $userStore)}
							bind:schedules
						/>
					</div>
				{/if}
			</svelte:fragment>
		</Tabs>
	</div>
</FlowCard>
