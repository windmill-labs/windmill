import { describe, expect, test } from 'bun:test'
import { WindmillChatApi } from '../src/api'

describe('WindmillChatApi.attachmentUrl', () => {
  test('points at the workspace download endpoint, with the storage only when there is one', () => {
    const api = new WindmillChatApi({ baseUrl: 'https://wm.test/api/', workspace: 'my ws' })
    expect(api.attachmentUrl({ input: 'files', s3: 'chat/a b&c.png', storage: 'secondary' })).toBe(
      'https://wm.test/api/w/my%20ws/job_helpers/download_s3_file?file_key=chat%2Fa+b%26c.png&storage=secondary'
    )
    expect(api.attachmentUrl({ input: 'files', s3: 'chat/a.png' })).toBe(
      'https://wm.test/api/w/my%20ws/job_helpers/download_s3_file?file_key=chat%2Fa.png'
    )
  })
})
