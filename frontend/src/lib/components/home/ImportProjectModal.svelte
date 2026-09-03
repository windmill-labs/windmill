<script lang="ts">
	import { untrack } from 'svelte'
	import { resource } from 'runed'
	import Modal from '$lib/components/common/modal/Modal.svelte'
	import PagedContent from '$lib/components/common/modal/PagedContent.svelte'
	import Skeleton from '$lib/components/common/skeleton/Skeleton.svelte'
	import ImportWizardSteps from '$lib/components/ImportWizardSteps.svelte'
	import ImportProjectStep from '$lib/components/ImportProjectStep.svelte'
	import ImportSetupStep from '$lib/components/ImportSetupStep.svelte'
	import ImportProjectCard, {
		type ImportProjectSummary
	} from '$lib/components/ImportProjectCard.svelte'
	import { fetchHubProject, hubBrowserUrl, type HubProjectPick } from '$lib/hubProject'
	import type { ImportExecution } from '$lib/importWizard/execution.svelte'
	import { useSetupStep } from '$lib/importWizard/setupStep.svelte'
	import type { ImportPlan } from '$lib/importWizard/plan'
	import { workspaceStore } from '$lib/stores'
	import { logFeatureUsage } from '$lib/utils/featureUsage'

	interface Props {
		/** The project the picker chose. Setting it opens the dialog. */
		pick: HubProjectPick | undefined
		onClose: () => void
		/** The import landed: the caller reloads its list, which replaces the empty state. */
		onImported?: () => void
	}

	let { pick, onClose, onImported }: Props = $props()

	let slug = $derived(pick?.slug)

	// The wizard route asks step 1 which workspace to import into and step 2 which one it
	// is. Opened from inside a workspace both answers are already given, so the dialog
	// starts at the import itself and the plan is fixed rather than URL-driven.
	let folder = $state<string | undefined>(undefined)
	let onSetupStep = $state(false)
	let execution = $state<ImportExecution | undefined>(undefined)

	let plan = $derived<ImportPlan>({
		slug: slug ?? '',
		destination: { kind: 'existing', workspaceId: $workspaceStore },
		folder
	})

	const setup = useSetupStep(
		() => execution,
		() => $workspaceStore
	)

	// The catalogue the picker reads carries no item counts, and both the header card and
	// the import step show them, so the detail is fetched for the one project chosen. The
	// card renders from the pick until it lands — counts are the only thing missing, and
	// zero counts render as no badges rather than as zeroes.
	const detail = resource(
		() => slug,
		async (s) => (s ? await fetchHubProject(s) : undefined)
	)
	let project = $derived<ImportProjectSummary | undefined>(
		detail.current ??
			(pick
				? {
						slug: pick.slug,
						name: pick.name,
						summary: pick.summary,
						author: pick.author,
						apps: pick.apps,
						logoUrl: pick.logoUrl,
						iconApps: pick.iconApps,
						counts: { apps: 0, flows: 0, scripts: 0, resources: 0 }
					}
				: undefined)
	)

	let hubHost = $state('hub.windmill.dev')
	void hubBrowserUrl()
		.then((u) => (hubHost = new URL(u).host))
		.catch(() => {})

	// Each open is its own import: a dialog reopened for another project must not inherit
	// the previous run, or its step would offer to resume a bundle from a different slug.
	$effect(() => {
		if (slug === undefined) {
			folder = undefined
			onSetupStep = false
			execution = undefined
		}
	})

	function finish() {
		// On the way out rather than on the pick: what is worth counting is an import that
		// landed, not a dialog that was opened and abandoned.
		if (slug) logFeatureUsage('home', 'template_import', { key: slug })
		onImported?.()
		onClose()
	}

	// The two pages this dialog has, named the way the dialog titles them. The route wizard
	// asks two more questions before these; here both were answered by being in a workspace.
	const IMPORT_PAGE = 'Import the project'
	const SETUP_PAGE = 'Fill credentials'
	let currentPage = $derived(onSetupStep ? SETUP_PAGE : IMPORT_PAGE)
	// Whether this import ends on the credentials step, known before it runs: every resource
	// the project ships arrives as an empty stub, so a project with any is one to fill in.
	// `setup.needed` is the real answer and only lands with the export, which also knows about
	// data tables — this is what lets the stepper name both steps from the first frame rather
	// than growing one mid-flow.
	let setupExists = $derived((project?.counts?.resources ?? 0) > 0 || setup.needed || onSetupStep)

	// The box height, decided when the dialog opens and left alone. It cannot follow
	// `setupExists`: that turns true when the detail fetch lands a moment after opening, and a
	// box that grows then is the dialog visibly loading in two steps. The catalogue already
	// says which integrations a project uses, which is what its credential stubs are, so the
	// answer is there in the first frame. A page that outgrows the box scrolls instead, with
	// its actions pinned.
	let tallBox = $state(false)
	$effect(() => {
		const opened = pick
		untrack(() => {
			if (opened) tallBox = (opened.apps?.length ?? 0) > 0
		})
	})
</script>

{#snippet importPage()}
	<div class="flex flex-1 flex-col gap-4 overflow-y-auto">
		{#if project}
			<!-- What is about to be imported, before the checklist says what will happen to
			     it: the project's own logo, name and prose. -->
			<ImportProjectCard {project} {hubHost} description={pick?.description} showCounts={false} />
		{/if}
		<ImportProjectStep
			chooseFolder={false}
			showNotes={false}
			fillHeight
			{plan}
			{project}
			setupPending={setup.needed}
			setupUndecided={setup.undecided}
			onFolderChange={(f) => (folder = f)}
			onFinish={() => (setup.needed ? (onSetupStep = true) : finish())}
			onBack={onClose}
			onExecution={(e) => (execution = e)}
			resume={execution}
		/>
	</div>
{/snippet}

{#snippet setupPlaceholder()}
	<!-- The shape of the credentials page — a line of prose, then the rows to fill — so the
	     slide has something to carry. The real step mounts on arrival and replaces it. -->
	<div class="flex flex-1 flex-col gap-4 pt-1">
		<Skeleton layout={[[2], 0.5, [1], 0.8, [3], 0.5, [3], 0.5, [3]]} />
	</div>
{/snippet}

{#snippet setupPage()}
	<div class="flex flex-1 flex-col overflow-y-auto">
		<ImportSetupStep
			fillHeight
			workspace={$workspaceStore ?? ''}
			slug={slug ?? ''}
			{folder}
			showHeading={false}
			onSkip={finish}
			onFinish={finish}
			onBack={execution ? () => (onSetupStep = false) : undefined}
		/>
	</div>
{/snippet}

<!-- No title: the stepper names the step and the card names the project, so a third label for
     what those two already say would only compete with them. -->
<Modal
	title=""
	paginated
	open={slug !== undefined}
	on:close={onClose}
	enterConfirms={false}
	class="sm:!max-w-[640px]"
	kind="X"
>
	{#if slug}
		<!-- The wizard's own stepper, not the dialog's page breadcrumb: this is the same flow
		     the /projects/import route runs, minus the two steps a workspace already answers.
		     Outside the pages, so it stays put while they slide. `lowestStep` closes the way
		     back once the run this dialog held is gone. -->
		<ImportWizardSteps
			step={onSetupStep ? 2 : 1}
			labels={['Import']}
			setupLabel={SETUP_PAGE}
			hasSetup={setupExists}
			lowestStep={execution ? 1 : 2}
			onNavigate={(s) => (onSetupStep = s === 2)}
		/>
		<!-- A definite height, which laid-over pages need: they are absolutely positioned, so
		     without one the box collapses. Fixed per shape rather than per page — a dialog that
		     resizes as a page slides in is the jump this pattern exists to avoid — but the setup
		     page is much the taller of the two, so a project that has no setup step is not given
		     its room. -->
		<PagedContent
			class={tallBox ? 'h-[min(72vh,560px)]' : 'h-[380px]'}
			current={currentPage}
			onNavigate={(key) => {
				if (key === IMPORT_PAGE && execution) onSetupStep = false
				else if (key === SETUP_PAGE && setup.needed) onSetupStep = true
			}}
			pages={[
				{ key: IMPORT_PAGE, content: importPage },
				{ key: SETUP_PAGE, content: setupPage, placeholder: setupPlaceholder }
			]}
		/>
	{/if}
</Modal>
