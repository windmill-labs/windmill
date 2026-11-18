export const triggerablesByAi = $state<TriggerablesMap>({})

export type TriggerablesMap = Record<
	string,
	{
		description: string
		onTrigger: ((value?: string) => void) | undefined
	}
>
