import { expect, test } from 'bun:test'
import { WindmillChatApi } from '../src/api'
import { followJob } from '../src/follow'
import { fetchMock, ndjson, sse } from './support'

test('a retried streaming step reports its offset as lost before the new sub-job is followed', async () => {
  let streams = 0
  const { fetch } = fetchMock((c) => {
    if (!c.url.pathname.endsWith('/jobs_u/getupdate_sse/job-1')) return undefined
    streams++
    if (streams === 1) {
      return sse([
        { type: 'update', new_result_stream: ndjson({ type: 'token_delta', content: 'a' }), stream_offset: 3, flow_stream_job_id: 'agent-1' },
        { type: 'update', new_result_stream: ndjson({ type: 'token_delta', content: 'b' }), stream_offset: 4, flow_stream_job_id: 'agent-2' }
      ])
    }
    return sse([{ type: 'update', stream_offset: 1, flow_stream_job_id: 'agent-2', completed: true, only_result: 'ok' }])
  })
  const api = new WindmillChatApi({ baseUrl: 'http://wm.test', workspace: 'ws', token: 'tok', fetch })
  const offsets: (number | undefined)[] = []
  for await (const _ of followJob(api, 'job-1', { onOffset: (o) => offsets.push(o) })) {
    // A resumer that stored offset 3 must not reuse it against agent-2.
  }
  expect(offsets).toEqual([3, undefined, 1])
})
