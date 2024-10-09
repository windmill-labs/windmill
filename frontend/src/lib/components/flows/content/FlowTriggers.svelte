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
</script>

<FlowCard {noEditor} title="Flow Triggers">
	<div class="pt-4">
		<Tabs bind:selected={$selectedTrigger}>
			<Tab value="webhooks" selectedClass="text-primary font-semibold">Webhooks</Tab>
			<Tab value="mail" selectedClass="text-primary text-sm font-semibold">Email</Tab>
			<Tab value="routes" selectedClass="text-primary text-sm font-semibold">Routes</Tab>
			<Tab value="schedules" selectedClass="text-primary text-sm font-semibold">Schedules</Tab>

			<svelte:fragment slot="content">
				{#if $selectedTrigger === 'webhooks'}
					<div class="p-4">
						<WebhooksPanel
							{newFlow}
							scopes={[`run:flow/${path}`]}
							{path}
							isFlow={true}
							args={{}}
							token=""
						/>
					</div>
				{/if}

				{#if $selectedTrigger === 'mail'}
					<div class="p-4">
						<EmailTriggerPanel
							{newFlow}
							token=""
							scopes={[`run:flow/${path}`]}
							{path}
							isFlow={true}
						/>
					</div>
				{/if}

				{#if $selectedTrigger === 'routes'}
					<div class="p-4">
						<RoutesPanel {newFlow} path={path ?? ''} isFlow={true} bind:triggers />
					</div>
				{/if}

				{#if $selectedTrigger === 'schedules'}
					<div class="p-4">
						<RunPageSchedules
							{newFlow}
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
