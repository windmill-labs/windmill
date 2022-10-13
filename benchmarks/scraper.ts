/// <reference no-default-lib="true" />
/// <reference lib="deno.worker" />

import { sleep } from "https://deno.land/x/sleep@v1.2.1/sleep.ts";
import parsePrometheusTextFormat from "npm:parse-prometheus-text-format";

const promise = new Promise<string>((resolve, _reject) => {
  self.onmessage = (evt) => {
    resolve(evt.data);
    self.onmessage = null;
  };
});
const host = await promise;

while (true) {
  const start = Date.now();
  const response = await fetch(host);
  const text = await response.text();
  // TODO: parsePrometheusTextFormat seems incomplete. Consider rewriting for deno with actual completeness.
  const prometheusValues: [
    {
      name: string;
      help: string;
    } & (
      | {
          type: "COUNTER" | "GAUGE";
          metrics: [{ value: string; labels: Record<string, string> }];
        }
      | {
          type: "HISTOGRAM";
          metrics: [{ buckets: Record<string, number> }];
        }
    )
  ] = parsePrometheusTextFormat(text);
  prometheusValues.forEach((x) => {
    self.postMessage(x);
  });

  const timeTaken = Date.now() - start;
  await sleep((100 - timeTaken) / 1000);
}
