export function shouldHideAppEditButton(
	searchParams: URLSearchParams,
	appValue: { hideEditButton?: boolean } | null | undefined
): boolean {
	return searchParams.get('hideEditBtn') === 'true' || appValue?.hideEditButton === true
}
