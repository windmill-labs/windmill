/**
 * Reads the value of a cookie by name.
 * @param {string} name - The name of the cookie to retrieve.
 * @returns {string | undefined} The cookie value, or undefined if not found.
 */
export function getCookie(name: string): string | undefined {
	const match = document.cookie.match(
		new RegExp('(?:^|; )' + name.replace(/([.$?*|{}()\[\]\\\/\+^])/g, '\\$1') + '=([^;]*)')
	)
	return match ? decodeURIComponent(match[1]) : undefined
}



// this only check the last 2 segments to work for popup on shared top-domain
export function sameTopDomainOrigin(origin: string | null, desktopOrigin: string): boolean {
	if (origin == null) {
		return false
	}
	const getLastTwoSegments = (url: string) => {
		const parts = url.split('.');
		return parts.length >= 2 ? parts.slice(-2).join('.') : url;
	};
	if (origin.includes('.') && desktopOrigin.includes('.')) {
		return getLastTwoSegments(origin) === getLastTwoSegments(desktopOrigin);
	} else {
		return origin === desktopOrigin;
	}
}