//comment this line and last to dev
export function wmillTsDev(port: number) { return `
let reqs: Record<string, any> = {}
let ws: WebSocket | null = null
let wsReady: Promise<void>
let wsReadyResolve: () => void

function initWebSocket() {
    wsReady = new Promise((resolve) => {
        wsReadyResolve = resolve
    })

    ws = new WebSocket('ws://localhost:${port}')

    ws.onopen = () => {
        console.log('[wmill] WebSocket connected')
        wsReadyResolve()
    }

    ws.onmessage = (event) => {
        const data = JSON.parse(event.data)
        if (data.type === 'backendRes' || data.type === 'backendAsyncRes') {
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

async function doRequest(type: string, o: object) {
    await wsReady
    return new Promise((resolve, reject) => {
        const reqId = Math.random().toString(36)
        reqs[reqId] = { resolve, reject }
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
`}