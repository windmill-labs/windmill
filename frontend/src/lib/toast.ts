import Toast from '$lib/components/Toast.svelte'
import { toast } from 'svelte-sonner'
export type ToastAction = {
	label: string
	callback: () => void
}

export function sendUserToast(
	message: string,
	error: boolean = false,
	actions: ToastAction[] = [],
	errorMessage: string | undefined = undefined,
	duration: number = 5000
): void {
	function onClose() {
		toast.dismiss(toastId)
	}

	const props = {
		componentProps: {
			message,
			actions,
			errorMessage,
			duration,
			onClose
		}
	}

	const toastId = error ? toast.error(Toast, props) : toast.info(Toast, props)
}
