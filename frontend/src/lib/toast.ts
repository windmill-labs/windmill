import Toast from '$lib/components/Toast.svelte'
import { toast } from '@zerodevx/svelte-toast'

export type ToastAction = {
	label: string
	callback: () => void
}

export function sendUserToast(
	message: string,
	error: boolean = false,
	actions: ToastAction[] = [],
	errorMessage: string | undefined = undefined
): void {
	toast.push({
		component: {
			src: Toast,
			props: {
				message,
				error,
				actions,
				errorMessage
			},
			sendIdTo: 'toastId'
		},
		dismissable: false,
		initial: 0,
		theme: {
			'--toastPadding': '0',
			'--toastMsgPadding': '0'
		}
	})
}
