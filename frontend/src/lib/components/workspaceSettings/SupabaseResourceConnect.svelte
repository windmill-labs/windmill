<script lang="ts">
	import Button from '../common/button/Button.svelte'
	import Modal2 from '../common/modal/Modal2.svelte'
	import SupabaseIcon from '../icons/SupabaseIcon.svelte'
	import SupabaseProjectStep from './SupabaseProjectStep.svelte'
	import { newWizardState } from './addDataTableModel'
	import { resolveSupabaseConnection, supabaseResourceValue } from './supabaseProvisioning'
	import { useSupabaseOauth } from './supabaseOauth.svelte'
	import { sendUserToast } from '$lib/toast'

	type Props = {
		/** The `postgresql` resource value for the project that was picked. */
		onPicked: (value: Record<string, any>) => void
	}

	let { onPicked }: Props = $props()

	let open = $state(false)
	let busy = $state(false)
	// Only the intent a resource form can act on. Creating a project is a billed action and
	// belongs in the data table wizard, which can show what it is provisioning and record the
	// result; a resource form has nowhere to put either.
	let intent = $state(newWizardState({ name: '', projectName: '', folder: '' }).supabase)
	let awaiting = $state(false)

	// Authorizing is not something to present a dialog about first: the button goes straight
	// to the popup, and the dialog opens on the way back, already holding the projects.
	// No `redirectIfBlocked`: navigating this tab away would take the half-filled resource form
	// with it, and there is nothing here to park and resume.
	const oauth = useSupabaseOauth({
		onFallbackBlocked: () => {
			awaiting = false
			sendUserToast('Allow pop-ups for this site to connect your Supabase account.', true)
		},
		onAbandoned: () => (awaiting = false),
		// Guarded: an authorization started somewhere else on the page reaches this listener too,
		// and it must not open a dialog nobody asked for.
		onAuthed: () => {
			if (!awaiting) return
			awaiting = false
			open = true
		}
	})

	function connect() {
		if (oauth.authed) {
			open = true
			return
		}
		awaiting = true
		oauth.connect()
	}

	// The resource is being edited by the user rather than created for them, so the project's
	// password goes straight into the form as a value. They can link it to a secret variable
	// with the same affordance every other password field has.
	async function apply() {
		const project = intent.project
		if (!project || !intent.password) return
		busy = true
		try {
			const connection = await resolveSupabaseConnection(
				oauth.token!,
				project,
				intent.connectionMode
			)
			onPicked({ ...supabaseResourceValue(project, '', connection), password: intent.password })
			open = false
			sendUserToast(
				connection.unavailable
					? `Filled in a direct connection for ${project.name}: ${connection.unavailable}`
					: `Filled in the connection for ${project.name}`,
				!!connection.unavailable
			)
		} catch (err) {
			sendUserToast(String(err), true)
		} finally {
			busy = false
		}
	}
</script>

<Button
	unifiedSize="md"
	variant="default"
	startIcon={{ icon: SupabaseIcon }}
	loading={awaiting}
	onClick={connect}
>
	Connect Supabase
</Button>

<Modal2
	bind:isOpen={open}
	target="#content"
	formStyling
	title="Connect Supabase"
	contentClasses="flex flex-col"
	fixedWidth="md"
	fixedHeight="lg"
>
	<div class="flex h-full flex-col gap-3">
		<div class="flex-1 flex flex-col gap-3 min-h-0">
			{#if oauth.token}
				<SupabaseProjectStep bind:intent token={oauth.token} existingOnly />
			{/if}
		</div>
		<div class="flex justify-end pt-3">
			<Button
				size="sm"
				variant="accent"
				disabled={!intent.project || !intent.password}
				loading={busy}
				onClick={apply}
			>
				Use this project
			</Button>
		</div>
	</div>
</Modal2>
