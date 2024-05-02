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
	function dismissToast() {
		toast.dismiss(toastId)
	}

	if (error) {
		toast.error(Toast, {
			componentProps: {
				message,
				actions,
				errorMessage,
				onClose: dismissToast
			}
		})
		return
	}

	const toastId = toast.info(Toast, {
		componentProps: {
			message,
			actions,
			errorMessage,
			onClose: dismissToast
		}
	})
}
