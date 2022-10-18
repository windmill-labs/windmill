/// <reference no-default-lib="true" />
/// <reference lib="deno.worker" />

import { sleep } from "https://deno.land/x/sleep@v1.2.1/sleep.ts";
import parsePrometheusTextFormat from "npm:parse-prometheus-text-format";

const promise = new Promise<{
  host: string;
  histogramBuckets: string[];
  exportHistograms: string[];
  exportSimple: string[];
}>((resolve, _reject) => {
  self.onmessage = (evt) => {
    resolve(evt.data);
    self.onmessage = null;
  };
});
const { host, histogramBuckets, exportHistograms, exportSimple } =
  await promise;

const columns = exportHistograms
  .flatMap((x) => (histogramBuckets as string[]).map((b) => x + "_" + b))
  .concat(exportSimple.map((x) => x + "_value"));

const values: Float32Array[] = [];

let cont = true;
self.onmessage = (_evt) => {
  cont = false;
};

while (cont) {
  const start = Date.now();
  const response = await fetch(host);
  const text = await response.text();
  // TODO: parsePrometheusTextFormat seems incomplete. Consider rewriting for deno with actual completeness.
  // Specifically histogram labels seem to not be reported.
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
  const new_values: Float32Array = new Float32Array(columns.length);

  prometheusValues.forEach((x) => {
    if (
      x.type == "HISTOGRAM" &&
      exportHistograms.findIndex((e) => e == x.name) !== -1
    ) {
      x.metrics.forEach((m) => {
        histogramBuckets.forEach((e) => {
          new_values[columns.indexOf(x.name + "_" + e)] = m.buckets[e];
        });
      });
    }
    if (
      (x.type == "GAUGE" || x.type == "COUNTER") &&
      exportSimple.findIndex((e) => e == x.name) !== -1
    ) {
      // TODO: is there something smarter we can do then take the mean of all labels?
      let v = 0;
      let n = 0;
      x.metrics.forEach((m) => {
        n++;
        v += Number(m.value);
      });
      new_values[columns.indexOf(x.name + "_value")] = v / n;
    }
  });

  values.push(new_values);

  const timeTaken = Date.now() - start;
  await sleep((100 - timeTaken) / 1000);
}

const transfer_values = values.map((x) => x.buffer);
self.postMessage({ columns, transfer_values }, transfer_values);
