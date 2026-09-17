//comment this line and last to dev
export function wmillTsDev() { return `
let reqs: Record<string, any> = {}
let ws: WebSocket | null = null
let wsReady: Promise<void>
let wsReadyResolve: () => void

function initWebSocket() {
    wsReady = new Promise((resolve) => {
        wsReadyResolve = resolve
    })

    ws = new WebSocket((window.location.protocol === 'https:' ? 'wss:' : 'ws:') + '//' + window.location.host)

    ws.onopen = () => {
        console.log('[wmill] WebSocket connected')
        wsReadyResolve()
    }

    ws.onmessage = (event) => {
        const data = JSON.parse(event.data)
        if (data.type === 'streamJobUpdate') {
            // Handle streaming update
            const job = reqs[data.reqId]
            if (job && job.onUpdate) {
                job.onUpdate({
                    new_result_stream: data.new_result_stream,
                    stream_offset: data.stream_offset
                })
            }
        } else if (data.type === 'streamJobRes') {
            // Handle stream completion
            const job = reqs[data.reqId]
            if (job) {
                if (data.error) {
                    job.reject(new Error(data.result?.stack ?? data.result?.message ?? 'Stream error'))
                } else {
                    job.resolve(data.result)
                }
                delete reqs[data.reqId]
            }
        } else if (data.type === 'backendRes' || data.type === 'backendAsyncRes') {
            console.log('Message from WebSocket backend', data)
            const job = reqs[data.reqId]
            if (job) {
                const result = data.result
                if (data.error) {
                    job.reject(new Error(result.stack ?? result.message))
                } else {
                    job.resolve(result)
                }
                delete reqs[data.reqId]
            } else {
                console.error('No job found for', data.reqId)
            }
        }
    }

    ws.onerror = (error) => {
        console.error('[wmill] WebSocket error:', error)
    }

    ws.onclose = () => {
        console.log('[wmill] WebSocket closed, reconnecting...')
        setTimeout(initWebSocket, 1000)
    }
}

initWebSocket()

/** A runnable call leaves this page over the WebSocket without touching the DOM,
 * so the session recorder of \`wmill app dev --recording\` (which frames the app)
 * has nothing else to tell it a step is still waiting on the backend. Announcing
 * the request and its answer to the shell mirrors what the deployed runner posts
 * across the same boundary. */
const framed = typeof window !== 'undefined' && window.parent !== window

function notifyRecorder(type: string, reqId: string) {
    if (framed) window.parent.postMessage({ type, reqId }, window.location.origin)
}

// A reload takes the previous context and its WebSocket with it, so whatever it
// had in flight can never answer. Announcing a fresh module is how the shell
// learns those calls are dead: a message posted from the unloading document
// would be dropped with the realm that sent it, and this runs before any app
// code can issue a call of its own.
if (framed) {
    window.parent.postMessage({ type: 'wmillDevReady' }, window.location.origin)
}

function tracked(type: string, reqId: string, resolve: (v: any) => void, reject: (e: any) => void) {
    notifyRecorder(type, reqId)
    let settled = false
    const done = () => {
        if (settled) return
        settled = true
        notifyRecorder(type + 'Res', reqId)
    }
    return {
        resolve: (v: any) => { done(); resolve(v) },
        reject: (e: any) => { done(); reject(e) }
    }
}

async function doRequest(type: string, o: object) {
    await wsReady
    return new Promise((resolve, reject) => {
        const reqId = Math.random().toString(36)
        reqs[reqId] = tracked(type, reqId, resolve, reject)
        ws?.send(JSON.stringify({ ...o, type, reqId }))
    })
}

export const backend = new Proxy(
    {},
    {
        get(_, runnable_id: string) {
            return (v: any) => {
                return doRequest('backend', { runnable_id, v })
            }
        }
    })

export const backendAsync = new Proxy(
    {},
    {
        get(_, runnable_id: string) {
            return (v: any) => {
                return doRequest('backendAsync', { runnable_id, v })
            }
        }
    })

export function waitJob(jobId: string) {
    return doRequest('waitJob', { jobId })
}

export function getJob(jobId: string) {
    return doRequest('getJob', { jobId })
}

/**
 * Stream job results using SSE. Calls onUpdate for each stream update,
 * and resolves with the final result when the job completes.
 * @param jobId - The job ID to stream
 * @param onUpdate - Callback for stream updates with new_result_stream data
 * @returns Promise that resolves with the final job result
 */
export function streamJob(
    jobId: string,
    onUpdate?: (data: { new_result_stream?: string; stream_offset?: number }) => void
): Promise<any> {
    return new Promise(async (resolve, reject) => {
        await wsReady
        const reqId = Math.random().toString(36)
        reqs[reqId] = { ...tracked('streamJob', reqId, resolve, reject), onUpdate }
        ws?.send(JSON.stringify({ jobId, type: 'streamJob', reqId }))
    })
}
`}
