<script lang="ts">
	/**
	 * The flow chat's model button: the same ChatModelSettings the session chat renders,
	 * over whatever the flow exposes.
	 *
	 * The agent takes one `provider` object, but an author can expose it field by field —
	 * fixing the resource in the step and letting the chat pick only the model, say. Each
	 * control here appears exactly when the flow wired the field behind it, so a chat never
	 * offers a knob whose value it could not write back.
	 */
	import ChatModelSettings from '$lib/components/copilot/ChatModelSettings.svelte'
	import {
		carriedReasoning,
		type ChatModelSettingsConfig
	} from '$lib/components/copilot/chatModelSettings'
	import AppConnect from '$lib/components/AppConnectDrawer.svelte'
	import { AI_PROVIDERS, fetchAvailableModels } from '$lib/components/copilot/lib'
	import {
		explicitOffToken,
		getReasoningCapability
	} from '$lib/components/copilot/reasoningRegistry'
	import { ResourceService, type AIProvider } from '$lib/gen'
	import type { Item } from '$lib/utils'
	import { Plug, Plus } from 'lucide-svelte'
	import { resource } from 'runed'
	import {
		composerDrivenFields,
		type AgentModelWiring,
		type ProviderField
	} from './agentChatInputs'

	interface Props {
		wiring: AgentModelWiring
		/** Every flow input value the composer holds for this conversation. */
		values: Record<string, any>
		setValue: (name: string, value: any) => void
		workspace?: string
	}

	let { wiring, values, setValue, workspace }: Props = $props()

	function fieldValue(field: ProviderField): any {
		if (wiring.whole) return values[wiring.whole]?.[field]
		const name = wiring.fields[field]
		return name ? values[name] : wiring.fixed[field]
	}

	const driven = $derived(composerDrivenFields(wiring))

	function editable(field: ProviderField): boolean {
		return driven.has(field)
	}

	/** Written together, because choosing a resource also invalidates the model. */
	function setFields(patch: Partial<Record<ProviderField, any>>) {
		if (wiring.whole) {
			setValue(wiring.whole, { ...(values[wiring.whole] ?? {}), ...patch })
			return
		}
		for (const [field, value] of Object.entries(patch)) {
			const name = wiring.fields[field as ProviderField]
			if (name) setValue(name, value)
		}
	}

	const resourceEditable = $derived(editable('resource'))
	const modelEditable = $derived(editable('model'))
	const effortEditable = $derived(editable('reasoning_effort'))
	// Wired, but left to the Configure-inputs modal: the button has no model to place it on.
	const effortInModal = $derived(wiring.fields.reasoning_effort !== undefined && !effortEditable)
	// Nothing to write: the flow fixes the lot, so the button names it and opens nothing.
	const readOnly = $derived(!resourceEditable && !modelEditable && !effortEditable)

	const AI_RESOURCE_TYPES = Object.keys(AI_PROVIDERS)

	// `$res:` is the stored form; the picker works in bare paths.
	const resourcePath = $derived(
		typeof fieldValue('resource') === 'string'
			? fieldValue('resource').replace(/^\$res:/, '') || undefined
			: undefined
	)
	const model = $derived(fieldValue('model'))
	const effort = $derived(fieldValue('reasoning_effort'))

	let appConnect: AppConnect | undefined = $state(undefined)
	// Bumped after the connect drawer creates one, to re-list.
	let resourcesVersion = $state(0)
	// Set when a resource is created here: it can only be selected once the re-listing
	// that follows tells us which provider it speaks.
	let pendingResourcePath = $state<string | undefined>(undefined)

	// A flow that fixes `kind` but exposes `resource` accepts resources of that kind only:
	// `setFields` drops a `kind` it cannot write, so any other provider's resource would be
	// listed, selected, and then run against the kind the flow still fixes.
	const allowedResourceTypes = $derived.by(() => {
		const fixedKind = editable('kind') ? undefined : (fieldValue('kind') as string | undefined)
		return fixedKind && AI_RESOURCE_TYPES.includes(fixedKind) ? [fixedKind] : AI_RESOURCE_TYPES
	})

	const resources = resource(
		() =>
			resourceEditable ? { workspace, version: resourcesVersion, allowedResourceTypes } : undefined,
		async (args) => {
			const ws = args?.workspace
			if (!ws) return []
			const rows = await ResourceService.listResource({
				workspace: ws,
				resourceType: (args?.allowedResourceTypes ?? AI_RESOURCE_TYPES).join(',')
			})
			return rows.map((r) => ({
				path: r.path,
				// The row's own type is the provider; an unrecognised one is a custom endpoint.
				provider: (AI_RESOURCE_TYPES.includes(r.resource_type ?? '')
					? r.resource_type
					: 'customai') as AIProvider
			}))
		}
	)

	const provider = $derived(
		resources.current?.find((r) => r.path === resourcePath)?.provider ??
			(fieldValue('kind') as AIProvider | undefined)
	)

	// Models the resource actually serves, asked of the provider. Its own catalogue is the
	// fallback, so a listing that fails or is unsupported still offers real ids rather than
	// an empty menu.
	const models = resource(
		() => ({ workspace, resourcePath, provider, modelEditable }),
		async ({ workspace, resourcePath, provider, modelEditable }, _prev, { onCleanup }) => {
			if (!modelEditable || !provider) return []
			const fallback = AI_PROVIDERS[provider]?.defaultModels ?? []
			if (!workspace || !resourcePath) return fallback
			const controller = new AbortController()
			onCleanup(() => controller.abort())
			try {
				const listed = await fetchAvailableModels(
					resourcePath,
					workspace,
					provider,
					controller.signal
				)
				return listed.length > 0 ? listed : fallback
			} catch {
				return fallback
			}
		}
	)

	$effect(() => {
		if (!pendingResourcePath) return
		const created = resources.current?.find((r) => r.path === pendingResourcePath)
		if (created) {
			pendingResourcePath = undefined
			selectResource(created.path, created.provider)
		}
	})

	/**
	 * The effort to write alongside a new model: `''` — the agent's "no effort" — wherever
	 * that model has no such level, so the composer never leaves behind a level the provider
	 * would reject. Writes nothing where the registry cannot speak for the model, since
	 * clearing a value on a guess would destroy the author's own default.
	 */
	function effortPatch(nextModel: string | undefined): Partial<Record<ProviderField, any>> {
		if (!provider || !nextModel) return {}
		const capability = getReasoningCapability(provider, nextModel)
		if (!capability.known) return {}
		const carried = carriedReasoning(
			typeof effort === 'string' ? effort : undefined,
			explicitOffToken(provider, nextModel) ?? '',
			capability
		)
		return { reasoning_effort: carried ?? '' }
	}

	function selectResource(path: string, picked: AIProvider) {
		setFields({
			kind: picked,
			resource: `$res:${path}`,
			// The models of one provider mean nothing to another, and the new list only
			// arrives async, so there is nothing to carry the current one against — nor the
			// effort, which only means something against a model. Cleared only where the
			// registry can speak for the new provider, for the same reason as `effortPatch`.
			// `''`, not `undefined`: storage drops an undefined key, and the schema's default
			// model would then come back under the new provider on reload.
			model: '',
			...(getReasoningCapability(picked, '').known ? { reasoning_effort: '' } : {})
		})
	}

	function providerItem(close: () => void): Item {
		const rows: Item[] = resources.loading
			? [{ displayName: 'Loading resources...', disabled: true }]
			: (resources.current ?? []).length === 0
				? [{ displayName: 'No AI resource in this workspace', disabled: true }]
				: (resources.current ?? []).map((r) => ({
						displayName: r.path,
						selected: r.path === resourcePath,
						action: () => selectResource(r.path, r.provider)
					}))
		return {
			displayName: 'Provider',
			icon: Plug,
			extra: providerSummary,
			submenuItems: [
				...rows,
				{
					// The same reach the form's ResourcePicker gives: create one without
					// leaving for workspace settings first.
					displayName: 'Add a resource',
					icon: Plus,
					separatorTop: true,
					action: () => {
						close()
						appConnect?.open()
					}
				}
			]
		}
	}

	const config = $derived<ChatModelSettingsConfig>({
		label: typeof model === 'string' && model ? model : 'Select a model',
		title: 'Model & reasoning settings',
		readOnly,
		readOnlyReason: 'Set in the flow',
		topItems: resourceEditable ? (close) => [providerItem(close)] : undefined,
		sections: modelEditable
			? [
					{
						label: 'Model',
						options: (models.current ?? []).map((m) => ({
							key: m,
							label: m,
							selected: m === model,
							onSelect: () => setFields({ model: m, ...effortPatch(m) })
						})),
						loading: models.loading,
						emptyMessage: provider ? 'No model listed' : 'Pick a provider first',
						// The step's own provider picker takes any model id, and the modal does not
						// ask for this input: without a typed entry, an endpoint that lists nothing
						// leaves the run with no model.
						custom: provider
							? {
									placeholder: 'Custom model id',
									onCommit: (m) => setFields({ model: m, ...effortPatch(m) })
								}
							: undefined
					}
				]
			: undefined,
		// Present whatever we can say about it, since the run uses an effort either way: a ladder,
		// a typed token, why there is none, or what the flow fixed. Absent only when the modal
		// is the effort's editor.
		reasoning: effortInModal
			? undefined
			: {
					provider,
					model: typeof model === 'string' && model ? model : undefined,
					value: typeof effort === 'string' ? effort : undefined,
					// An agent writes the provider-native token straight into its step, so there is no
					// sentinel to translate later. Where a model disables by omission instead, the empty
					// string is that off: the run reads an empty `reasoning_effort` as absent
					// (types.rs `get_reasoning_effort`).
					offToken:
						provider && typeof model === 'string' && model
							? (explicitOffToken(provider, model) ?? '')
							: '',
					// An agent step omits `reasoning_effort` when it is unset, so the provider picks —
					// naming a level would claim something the run does not do.
					sendsDefaultWhenUnset: false,
					writable: effortEditable,
					typedWhenUnknown: true,
					onSelect: (token) => setFields({ reasoning_effort: token })
				}
	})
</script>

{#snippet providerSummary()}
	{#if resourcePath}
		<span class="shrink-0 text-tertiary truncate max-w-[80px]">{resourcePath}</span>
	{/if}
{/snippet}

{#if resourceEditable}
	<AppConnect
		bind:this={appConnect}
		{workspace}
		on:refresh={(e) => {
			resourcesVersion++
			if (e.detail) {
				pendingResourcePath = e.detail
			}
		}}
	/>
{/if}

<ChatModelSettings {config} />
