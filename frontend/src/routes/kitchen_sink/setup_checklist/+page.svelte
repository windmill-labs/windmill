<script lang="ts">
	import SetupChecklist, { type SetupStep } from '$lib/components/wizards/SetupChecklist.svelte'
	import { instanceSetupSteps } from '$lib/components/workspaceSettings/instanceDbSteps'
	import { supabaseSetupSteps } from '$lib/components/workspaceSettings/supabaseProvisioning'
	import { Button } from '$lib/components/common'
	import Toggle from '$lib/components/Toggle.svelte'
	import DarkModeToggle from '$lib/components/sidebar/DarkModeToggle.svelte'
	import type { CustomInstanceDb, LoggedWizardStatus } from '$lib/gen'

	// Playground for the wizard setup checklist: the REAL SetupChecklist driven by fake
	// progress, so the run-through animation and every failure position can be seen without
	// a backend, a superadmin, or a Supabase account.

	let stepMs = $state(700)
	let failAt = $state(0) // 0 = never fail, otherwise the 1-based step that fails

	// --- Supabase: stage 0 idle, 1 created, 2 starting, 3 checking, 4 ready ---
	let supaStage = $state(0)
	let supaRunning = $state(false)

	async function runSupabase() {
		if (supaRunning) return
		supaRunning = true
		supaStage = 0
		for (let s = 1; s <= 4; s++) {
			supaStage = s
			await sleep(stepMs)
			if (failAt === s) break
		}
		supaRunning = false
	}

	let supaFailed = $derived(failAt > 0 && failAt === supaStage && !supaRunning && supaStage < 4)

	// --- Instance: the backend reports every check at once, so the fake mirrors that ---
	const INSTANCE_LOG_KEYS = [
		'super_admin',
		'database_credentials',
		'valid_dbname',
		'created_database',
		'db_connect',
		'grant_permissions',
		'replication_user'
	] as const

	let instanceRunning = $state(false)
	let instanceStatus: CustomInstanceDb | undefined = $state(undefined)

	async function runInstance() {
		if (instanceRunning) return
		instanceRunning = true
		instanceStatus = undefined
		// One call, one answer: the spinner sits on the first unreported step for the whole
		// duration, exactly as it does against the real endpoint.
		await sleep(stepMs * 3)
		instanceStatus = buildStatus()
		instanceRunning = false
	}

	function buildStatus(): CustomInstanceDb {
		const logs: Record<string, LoggedWizardStatus> = {}
		for (let i = 0; i < INSTANCE_LOG_KEYS.length; i++) {
			if (failAt > 0 && i + 1 >= failAt) break
			logs[INSTANCE_LOG_KEYS[i]] = 'OK'
		}
		return {
			success: failAt === 0,
			error: failAt === 0 ? undefined : `Simulated failure at step ${failAt}`,
			logs
		} as unknown as CustomInstanceDb
	}

	function sleep(ms: number) {
		return new Promise((r) => setTimeout(r, ms))
	}

	// --- A hand-rolled list, to see every status side by side ---
	const allStates: SetupStep[] = [
		{ title: 'Pending', status: 'pending', description: 'Not reached yet.' },
		{ title: 'Running', status: 'running' },
		{ title: 'Done', status: 'done' },
		{
			title: 'Failed',
			status: 'failed',
			description: 'Failures expand on their own so the reason is never hidden behind a click.'
		},
		{ title: 'Skipped', status: 'skipped', description: 'Nothing to do for this one.' }
	]
</script>

<div class="p-6 flex flex-col gap-6 max-w-3xl mx-auto">
	<div class="flex items-center justify-between">
		<h1 class="text-2xl font-semibold">Setup checklist</h1>
		<DarkModeToggle />
	</div>

	<div class="flex flex-wrap items-end gap-4 p-4 rounded-md bg-surface-secondary">
		<label class="flex flex-col gap-1 text-xs">
			<span class="font-semibold text-emphasis">Step duration (ms)</span>
			<input type="number" bind:value={stepMs} min="100" step="100" class="w-32" />
		</label>
		<label class="flex flex-col gap-1 text-xs">
			<span class="font-semibold text-emphasis">Fail at step (0 = never)</span>
			<input type="number" bind:value={failAt} min="0" max="7" class="w-32" />
		</label>
		<Toggle bind:checked={supaRunning} disabled options={{ right: 'Supabase running' }} />
		<Toggle bind:checked={instanceRunning} disabled options={{ right: 'Instance running' }} />
	</div>

	<section class="flex flex-col gap-2">
		<div class="flex items-center justify-between">
			<h2 class="text-sm font-semibold text-emphasis">Supabase provisioning</h2>
			<Button size="xs" variant="accent" onClick={runSupabase} disabled={supaRunning}>Run</Button>
		</div>
		<SetupChecklist steps={supabaseSetupSteps(supaStage, supaFailed)} />
	</section>

	<section class="flex flex-col gap-2">
		<div class="flex items-center justify-between">
			<h2 class="text-sm font-semibold text-emphasis">Instance database setup</h2>
			<Button size="xs" variant="accent" onClick={runInstance} disabled={instanceRunning}>
				Run
			</Button>
		</div>
		<SetupChecklist steps={instanceSetupSteps('dt_playground', instanceStatus, instanceRunning)} />
	</section>

	<section class="flex flex-col gap-2">
		<h2 class="text-sm font-semibold text-emphasis">Every status</h2>
		<SetupChecklist steps={allStates} />
	</section>
</div>
