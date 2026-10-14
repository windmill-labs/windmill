// Query params Windmill consumes internally and that should not be exposed to
// app logic via the `query` context. Only params we actually own are listed
// here — the `wm_` prefix is a naming convention, not a reserved namespace, so
// we don't strip it wholesale (that would break apps reading their own `wm_*`
// params). `wm_coep` is a transport flag for cross-origin isolation headers;
// `wm_embed`/`wm_embedder_origin` are the opaque app viewer transport params
// (see PublicAppFrame).
export const WINDMILL_RESERVED_QUERY_PARAMS = new Set(['wm_coep', 'wm_embed', 'wm_embedder_origin'])

export function urlParamsToObject(
	params: URLSearchParams,
	opts?: { stripReserved?: boolean }
): Record<string, string> {
	const result: Record<string, string> = {}
	params.forEach((value, key) => {
		if (opts?.stripReserved && WINDMILL_RESERVED_QUERY_PARAMS.has(key)) {
			return
		}
		result[key] = value
	})
	return result
}
