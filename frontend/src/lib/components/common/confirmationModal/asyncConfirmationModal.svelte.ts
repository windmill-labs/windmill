import { createRawSnippet, type ComponentProps } from 'svelte'
import type ConfirmationModal from './ConfirmationModal.svelte'

/**
 * This allows asking for confirmation while maintaining a linear imperative flow,
 * and avoiding unnecessary states and callback hopping
 *
 * @example
 *  let confirmationModal = createAsyncConfirmationModal()
 *  // ...
 *  let confirmed = await confirmationModal.ask({
 *    title: "The following objects do not exist",
 *    confirmationText: `Yes`,
 *    children: `Do you want to create the objects ?`
 *  })
 *  if (!confirmed) return
 *  // ...
 *  <ConfirmationModal {...confirmationModal.props} />
 */
export function createAsyncConfirmationModal(): {
	props: ComponentProps<ConfirmationModal>
	ask: (props: Params) => Promise<boolean>
} {
	// Create a new instance of ConfirmationModal with the provided props
	const o: ReturnType<typeof createAsyncConfirmationModal> = $state({
		props: {
			confirmationText: '',
			title: ''
		},
		ask: (params) =>
			new Promise<boolean>((resolve) => {
				o.props = {
					...params,
					children: createRawSnippet(() => ({ render: () => params.children })),
					open: true,
					loading: false,
					onCanceled: () => (resolve(false), (o.props.open = false)),
					onConfirmed: async () => {
						o.props.loading = true
						await params.onConfirmed?.()
						o.props.loading = false
						resolve(true)
						o.props.open = false
					}
				}
			})
	})
	return o
}

type Params = Omit<ComponentProps<ConfirmationModal>, 'children'> & { children: string }
