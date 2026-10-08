import { SettingService, type DatatableRoleCluster, type InstanceDatatableRole } from '$lib/gen'
import { sendUserToast } from '$lib/toast'
import type { ConfirmationModalHandle } from '../common/confirmationModal/asyncConfirmationModal.svelte'

export function pgInstanceName(cluster: DatatableRoleCluster | undefined): string {
	return cluster === 'external_instance'
		? 'the external Postgres instance'
		: 'the Windmill Postgres instance'
}

function escapeHtml(text: string): string {
	return text.replace(
		/[&<>"']/g,
		(c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[c]!
	)
}

/** Reports a failed role creation. One refused because Postgres already has a role of that name
 * (a 409) opens the offer to take that role over instead. `confirmationModal` must be closed: the
 * offer takes it over. */
export async function handleRoleCreationError(
	e: any,
	opts: {
		name: string
		cluster: DatatableRoleCluster | undefined
		confirmationModal: ConfirmationModalHandle
		onTakenOver: (role: InstanceDatatableRole) => void
	}
) {
	if (e?.status !== 409) {
		sendUserToast(e?.body ?? e?.message ?? String(e), true)
		return
	}
	await offerTakeover(opts)
}

/** A role no data table names any more is still a login on its Postgres instance. A superadmin is
 * offered to drop it; anyone else is told to ask one. Resolves to whether it was dropped. */
export async function offerUnusedRoleDrop({
	role,
	cluster,
	superadmin,
	confirmationModal
}: {
	role: InstanceDatatableRole
	cluster: DatatableRoleCluster | undefined
	superadmin: boolean
	confirmationModal: ConfirmationModalHandle
}): Promise<boolean> {
	const name = `<span class="font-mono">${escapeHtml(role.name)}</span>`
	const instance = pgInstanceName(cluster)
	if (!superadmin) {
		await confirmationModal.ask({
			title: `Role ${role.name} is no longer used`,
			children: `<p>No data table uses the role ${name} any more, but its login still exists in ${instance}. Ask a superadmin to drop it there.</p>`,
			confirmationText: 'OK',
			type: 'info',
			hideCancel: true
		})
		return false
	}
	let dropped = false
	await confirmationModal.ask({
		title: `Drop the role ${role.name}?`,
		children: `<div class="flex flex-col gap-2">
			<p>No data table uses the role ${name} any more, but its login still exists in ${instance}. Windmill can drop it.</p>
			<p>Dropping it <b>deletes the login ${name}</b> from ${instance}: anything still connecting as it, such as a script with <span class="font-mono">-- role ${escapeHtml(role.name)}</span>, fails from then on. Everything it owns in the databases Windmill manages there is handed back to admin, and its grants are revoked. This cannot be undone.</p>
			<p>Keep it if you plan to give it to a data table again.</p>
		</div>`,
		confirmationText: 'Drop role',
		type: 'danger',
		onConfirmed: async () => {
			try {
				await SettingService.deleteInstanceDatatableRole({ id: role.id })
				dropped = true
				sendUserToast(`Dropped the role ${role.name}`)
			} catch (e) {
				sendUserToast(e?.body ?? e?.message ?? String(e), true)
			}
		}
	})
	return dropped
}

async function offerTakeover({
	name,
	cluster,
	confirmationModal,
	onTakenOver
}: {
	name: string
	cluster: DatatableRoleCluster | undefined
	confirmationModal: ConfirmationModalHandle
	onTakenOver: (role: InstanceDatatableRole) => void
}) {
	const role = `<span class="font-mono">${escapeHtml(name)}</span>`
	let taken: InstanceDatatableRole | undefined
	await confirmationModal.ask({
		title: `Let Windmill take over the role ${name}?`,
		children: `<div class="flex flex-col gap-2">
			<p>A Postgres role named ${role} already exists in ${pgInstanceName(cluster)}, and Windmill did not create it. A data table role logs in with a password only Windmill knows, so Windmill has to manage it to use it.</p>
			<p>Taking it over <b>replaces the password of ${role}</b> with a random one and lets it log in. Anything that logs in as ${role} today with its current password, such as another application, a person or another Windmill instance, will be locked out. The old password cannot be restored.</p>
			<p>From then on Windmill manages ${role} like a role it created: deleting it in Windmill drops it from Postgres. A role that is a superuser, can create roles or databases, is a member of other roles or has members of its own is refused.</p>
		</div>`,
		confirmationText: 'Replace password and take over',
		type: 'danger',
		onConfirmed: async () => {
			try {
				taken = await SettingService.createInstanceDatatableRole({
					requestBody: { name, cluster, take_over: true }
				})
			} catch (e) {
				sendUserToast(e?.body ?? e?.message ?? String(e), true)
			}
		}
	})
	if (!taken) return
	sendUserToast(`Windmill now manages the role ${name}`)
	onTakenOver(taken)
}
