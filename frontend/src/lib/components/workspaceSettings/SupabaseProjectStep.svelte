<script lang="ts">
	import Alert from '../common/alert/Alert.svelte'
	import ToggleButtonGroup from '../common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '../common/toggleButton-v2/ToggleButton.svelte'
	import TextInput from '../text_input/TextInput.svelte'
	import Password from '../Password.svelte'
	import Select from '../select/Select.svelte'
	import { Database, Loader2 } from 'lucide-svelte'
	import { tick } from 'svelte'
	import { sendUserToast } from '$lib/toast'
	import SupabaseConnectionMode from './SupabaseConnectionMode.svelte'
	import type { WizardState } from './addDataTableModel'
	import {
		getSupabaseOrgPlan,
		listSupabaseOrgs,
		listSupabaseProjects,
		orgSlug,
		projectOrg,
		projectRef,
		SUPABASE_REGIONS,
		type SupabaseOrg,
		type SupabaseProject
	} from './supabaseProvisioning'

	type Props = {
		/** The wizard's Supabase slice. Collected only -- nothing here creates anything. */
		intent: WizardState['supabase']
		token: string
		/** Hides the create tab where provisioning a billed project is not on offer. */
		existingOnly?: boolean
		/** Fired whenever the choice changes, so the host can drop what it derived from it. */
		onIntentChange?: () => void
	}

	let { intent = $bindable(), token, existingOnly = false, onIntentChange }: Props = $props()

	let orgs: SupabaseOrg[] | undefined = $state(undefined)
	let projects: SupabaseProject[] | undefined = $state(undefined)
	let plans: Record<string, string> = $state({})

	// Nothing but a spinner until *both* lists are in. Which mode to open on depends on the
	// projects, so clearing this when only the orgs have landed is what makes the toggle flip
	// under the user a moment later.
	let loading = $state(false)
	let loaded = $state(false)

	$effect(() => {
		if (token && !loaded) load(token)
	})

	async function load(t: string) {
		loaded = true
		loading = true
		try {
			orgs = await listSupabaseOrgs(t)
			if (orgs?.length && !intent.org) intent.org = orgSlug(orgs[0])
			projects = await listSupabaseProjects(t)
			// Someone who already has a Supabase database almost always means to connect it
			// rather than make a second one. Decided before anything renders, so the toggle
			// never visibly flips under the user.
			if (projects?.length) intent.mode = 'existing'
			else if (existingOnly) intent.mode = 'existing'
			// The plan decides who gets billed, and the list endpoint does not carry it.
			for (const o of orgs ?? []) {
				getSupabaseOrgPlan(t, orgSlug(o)).then((p) => {
					if (p) plans[orgSlug(o)] = p
				})
			}
		} catch (err) {
			sendUserToast(String(err), true)
			orgs = orgs ?? []
		} finally {
			loading = false
		}
	}

	/** Supabase statuses are SCREAMING_SNAKE; only surface one that is not the happy path. */
	function projectStatus(p: SupabaseProject): string | undefined {
		if (!p.status || p.status === 'ACTIVE_HEALTHY') return undefined
		return p.status === 'INACTIVE' ? 'paused' : p.status.toLowerCase().replace(/_/g, ' ')
	}

	// Each tab owns what it produced. A project picked on one tab, and whatever the host
	// derived from it, must not survive into the other.
	function setMode(v: 'create' | 'existing') {
		if (v === intent.mode) return
		intent.mode = v
		onIntentChange?.()
	}

	/** Takes the picked project as a parameter: read directly off the prop inside a `$derived`,
	 * the checker narrows its optional type to `never`. */
	function isSelected(picked: SupabaseProject | undefined, p: SupabaseProject): boolean {
		return !!picked && projectRef(picked) === projectRef(p)
	}

	// The password field lives inside the card it belongs to, so it is scoped to one project:
	// carrying a value over to another card would show it already filled in. Selecting the last
	// card in a long list also grows it past the fold, hence the scroll once it has resized.
	async function selectProject(p: SupabaseProject, card: HTMLElement | null) {
		if (!isSelected(intent.project, p)) intent.password = ''
		intent.project = p
		onIntentChange?.()
		await tick()
		card?.scrollIntoView({ block: 'nearest' })
	}

	/** Built from parameters rather than read off the surrounding `$state(undefined)`, which a
	 * `$derived` in the same scope narrows to `never`. */
	function orgOptions(
		all: SupabaseOrg[] | undefined,
		projs: SupabaseProject[] | undefined,
		plansBySlug: Record<string, string>
	) {
		return (all ?? []).map((o) => {
			const slug = orgSlug(o)
			const count = (projs ?? []).filter((p) => projectOrg(p) === slug).length
			return {
				label: o.name,
				value: slug,
				subtitle: [plansBySlug[slug], `${count} project${count === 1 ? '' : 's'}`]
					.filter(Boolean)
					.join(' · ')
			}
		})
	}

	let orgItems = $derived(orgOptions(orgs, projects, plans))
</script>

{#if loading}
	<div class="flex items-center gap-2 text-xs text-secondary py-2">
		<Loader2 size={16} class="animate-spin" />
		Loading your Supabase projects...
	</div>
{:else}
	{#if !existingOnly}
		<ToggleButtonGroup bind:selected={() => intent.mode, (v) => setMode(v)}>
			{#snippet children({ item })}
				<ToggleButton value="existing" label="Use an existing project" {item} small />
				<ToggleButton value="create" label="Create a new project" {item} small />
			{/snippet}
		</ToggleButtonGroup>
	{/if}

	{#if intent.mode === 'create'}
		<div class="grid grid-cols-2 gap-2">
			<div>
				<span class="text-xs font-semibold text-emphasis">Organization</span>
				<Select
					items={orgItems}
					bind:value={() => intent.org, (v) => ((intent.org = v), onIntentChange?.())}
					placeholder={orgs === undefined ? 'Loading...' : 'Select'}
				/>
				<p class="text-2xs text-secondary mt-1">
					The project is created here and billed to this organization.
				</p>
			</div>
			<div>
				<span class="text-xs font-semibold text-emphasis">Region</span>
				<Select
					items={SUPABASE_REGIONS.map((r) => ({ label: r.label, value: r.code }))}
					bind:value={intent.region}
					placeholder="Region"
				/>
			</div>
		</div>
		<div>
			<span class="text-xs font-semibold text-emphasis">Project name</span>
			<TextInput bind:value={intent.projectName} inputProps={{ placeholder: 'windmill-data' }} />
		</div>
		<Alert type="info" size="xs" bgClass="border-0" title="">
			Windmill generates and stores the database password. A new project takes a minute or two to
			come up.
		</Alert>
		<SupabaseConnectionMode bind:mode={intent.connectionMode} onChange={onIntentChange} />
	{:else}
		<div class="flex flex-col gap-2 overflow-y-auto flex-1 min-h-24 pr-1">
			{#each projects ?? [] as p (projectRef(p))}
				{@const selected = isSelected(intent.project, p)}
				<!-- shrink-0 or the flex column squeezes the cards to fit instead of letting the
				list scroll, and the selected one loses its password field to the clip. -->
				<div
					class="shrink-0 border rounded-md overflow-hidden transition-colors {selected
						? 'border-border-selected/50 bg-surface-accent-selected'
						: 'border-border-light'}"
				>
					<button
						class="w-full text-left p-3 flex gap-3 items-start {selected
							? ''
							: 'hover:bg-surface-hover'}"
						onclick={(e) => selectProject(p, e.currentTarget.parentElement)}
					>
						<span class="mt-0.5 shrink-0"><Database size={18} class="text-secondary" /></span>
						<span class="flex flex-col gap-0.5 min-w-0">
							<span class="text-xs font-medium {selected ? 'text-accent' : 'text-emphasis'}"
								>{p.name}</span
							>
							<span class="text-xs text-secondary font-normal">
								{p.region}{projectOrg(p) ? ` · ${projectOrg(p)}` : ''}{projectStatus(p)
									? ` · ${projectStatus(p)}`
									: ''}
							</span>
						</span>
					</button>
					{#if selected}
						<div class="px-3 pb-3 flex flex-col gap-2">
							<div>
								<span class="text-xs font-semibold text-emphasis">Database password</span>
								<Password
									bind:password={
										() => intent.password, (v) => ((intent.password = v ?? ''), onIntentChange?.())
									}
									placeholder="••••••••"
								/>
								<p class="text-2xs text-secondary mt-1">
									Supabase only shows this when the project is created, and never exposes it through
									its API. If you no longer have it, <a
										href="https://supabase.com/dashboard/project/{projectRef(p)}/database/settings"
										target="_blank"
										rel="noreferrer"
										class="text-blue-500 hover:underline">set a new one</a
									> — every existing connection to this project stops working when you do.
								</p>
							</div>
							<SupabaseConnectionMode bind:mode={intent.connectionMode} onChange={onIntentChange} />
						</div>
					{/if}
				</div>
			{/each}
			{#if (projects ?? []).length === 0}
				<Alert type="info" size="xs" bgClass="border-0" title="">
					This Supabase account has no projects yet.
				</Alert>
			{/if}
		</div>
	{/if}
{/if}
