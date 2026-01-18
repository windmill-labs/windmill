// OpenTelemetry Auto-Instrumentation Loader for Bun/Node.js
// This file is loaded via -r flag when WINDMILL_OTEL_AUTO_INSTRUMENTATION=true
// It wraps the global fetch to automatically create OTel spans

const OTEL_ENDPOINT = process.env.OTEL_EXPORTER_OTLP_TRACES_ENDPOINT;
const JOB_ID = process.env.WINDMILL_JOB_ID;
const WORKSPACE_ID = process.env.WINDMILL_WORKSPACE_ID;
const SERVICE_NAME = process.env.OTEL_SERVICE_NAME || 'windmill-script';

if (!OTEL_ENDPOINT || !JOB_ID || !WORKSPACE_ID) {
  // Skip instrumentation if env vars not set
  console.log('[OTel] Missing environment variables, skipping instrumentation');
} else {
  console.log(`[OTel] Auto-instrumentation enabled, sending traces to ${OTEL_ENDPOINT}`);

  // Simple trace context generator
  function generateTraceId() {
    const bytes = new Uint8Array(16);
    crypto.getRandomValues(bytes);
    return Array.from(bytes).map(b => b.toString(16).padStart(2, '0')).join('');
  }

  function generateSpanId() {
    const bytes = new Uint8Array(8);
    crypto.getRandomValues(bytes);
    return Array.from(bytes).map(b => b.toString(16).padStart(2, '0')).join('');
  }

  // Store for pending spans
  const pendingSpans = [];
  let flushTimeout = null;

  // Encode spans to OTLP protobuf format (simplified JSON export for now)
  async function flushSpans() {
    if (pendingSpans.length === 0) return;

    const spansToSend = pendingSpans.splice(0, pendingSpans.length);

    // Build OTLP JSON format (will be converted to protobuf by collector or use JSON endpoint)
    const jsonEndpoint = OTEL_ENDPOINT.replace('/v1/traces', '/v1/traces');

    const exportRequest = {
      resourceSpans: [{
        resource: {
          attributes: [
            { key: 'service.name', value: { stringValue: SERVICE_NAME } },
            { key: 'windmill.job_id', value: { stringValue: JOB_ID } },
            { key: 'windmill.workspace_id', value: { stringValue: WORKSPACE_ID } }
          ]
        },
        scopeSpans: [{
          scope: {
            name: 'windmill-otel-bun-loader',
            version: '1.0.0'
          },
          spans: spansToSend
        }]
      }]
    };

    try {
      // Use the original fetch to avoid recursion
      await originalFetch(jsonEndpoint, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json'
        },
        body: JSON.stringify(exportRequest)
      });
    } catch (e) {
      // Silently ignore export errors to not affect the script
      console.error('[OTel] Failed to export spans:', e.message);
    }
  }

  function scheduleFlush() {
    if (flushTimeout) return;
    flushTimeout = setTimeout(() => {
      flushTimeout = null;
      flushSpans();
    }, 100);
  }

  // Wrap the global fetch
  const originalFetch = globalThis.fetch;

  globalThis.fetch = async function instrumentedFetch(input, init) {
    const url = typeof input === 'string' ? input : input.url;
    const method = init?.method || (typeof input === 'object' ? input.method : 'GET') || 'GET';

    // Don't instrument calls to the OTel endpoint to avoid recursion
    if (url.includes(OTEL_ENDPOINT) || url.includes('/v1/traces')) {
      return originalFetch(input, init);
    }

    const traceId = generateTraceId();
    const spanId = generateSpanId();
    const startTimeNano = BigInt(Date.now()) * 1000000n;

    let statusCode = 0; // UNSET
    let statusMessage = '';
    let responseStatus = 0;
    let error = null;

    try {
      const response = await originalFetch(input, init);
      responseStatus = response.status;

      if (response.ok) {
        statusCode = 1; // OK
      } else {
        statusCode = 2; // ERROR
        statusMessage = `HTTP ${response.status}`;
      }

      return response;
    } catch (e) {
      statusCode = 2; // ERROR
      statusMessage = e.message;
      error = e;
      throw e;
    } finally {
      const endTimeNano = BigInt(Date.now()) * 1000000n;

      // Parse URL for attributes
      let parsedUrl;
      try {
        parsedUrl = new URL(url);
      } catch {
        parsedUrl = { hostname: '', pathname: url, protocol: '' };
      }

      const span = {
        traceId: hexToBase64(traceId),
        spanId: hexToBase64(spanId),
        name: `HTTP ${method}`,
        kind: 3, // CLIENT
        startTimeUnixNano: startTimeNano.toString(),
        endTimeUnixNano: endTimeNano.toString(),
        attributes: [
          { key: 'http.method', value: { stringValue: method } },
          { key: 'http.url', value: { stringValue: url } },
          { key: 'http.host', value: { stringValue: parsedUrl.hostname } },
          { key: 'http.target', value: { stringValue: parsedUrl.pathname } },
          { key: 'http.scheme', value: { stringValue: parsedUrl.protocol?.replace(':', '') || 'https' } }
        ],
        status: {
          code: statusCode,
          message: statusMessage
        }
      };

      if (responseStatus) {
        span.attributes.push({ key: 'http.status_code', value: { intValue: responseStatus.toString() } });
      }

      if (error) {
        span.events = [{
          timeUnixNano: endTimeNano.toString(),
          name: 'exception',
          attributes: [
            { key: 'exception.message', value: { stringValue: error.message } },
            { key: 'exception.type', value: { stringValue: error.name } }
          ]
        }];
      }

      pendingSpans.push(span);
      scheduleFlush();
    }
  };

  // Helper to convert hex to base64 for OTLP JSON format
  function hexToBase64(hex) {
    const bytes = new Uint8Array(hex.match(/.{1,2}/g).map(byte => parseInt(byte, 16)));
    return btoa(String.fromCharCode(...bytes));
  }

  // Ensure spans are flushed before process exits
  process.on('beforeExit', async () => {
    if (flushTimeout) {
      clearTimeout(flushTimeout);
      flushTimeout = null;
    }
    await flushSpans();
  });

  process.on('exit', () => {
    // Synchronous - can't do async here, but beforeExit should have handled it
  });
}
