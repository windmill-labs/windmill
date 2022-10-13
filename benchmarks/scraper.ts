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
  // We don't care for the actual parsing, we only care for reading out the lines & exporting those.
  // We waste a bunch of performance by doing this too
  // Read below TODOs, they are also solved by this.
  const prometheusValues: [
    {
      name: string;
      help: string;
    } & (
      | {
          type: "COUNTER" | "GAUGE";
          metrics: [{ value: string; labels: { [key: string]: string } }];
        }
      | {
          type: "HISTOGRAM";
          metrics: [{ buckets: { [key: string]: number } }];
        }
    )
  ] = parsePrometheusTextFormat(text); /*
  const map: Map<string, string> = new Map();
  // TODO: optimize this
  // This "simply" unpacks measurements into the common name_{labels} form and puts them into a map (with their value as value)
  // for counters & gauges labels are considered, for histograms buckets are considered (le=)
  prometheusValues.forEach((value) => {
    if (value.type == "COUNTER" || value.type == "GAUGE") {
      value.metrics.forEach((m) => {
        const labels = new Map(Object.entries(m.labels));
        if (labels.size > 0) {
          const arr = Array.from(labels.entries());
          let s = "{";
          arr.forEach(([k, v]) => {
            s = s + k + "=" + v + ",";
          });
          map.set(value.name + s.substring(0, s.length - 1) + "}", m.value);
        } else {
          map.set(value.name, m.value);
        }
      });
    } else if (value.type == "HISTOGRAM") {
      // TODO: Histograms ususally also have labels. parsePrometheusTextFormat doesn't expose these.
      value.metrics.forEach((m) => {
        new Map(Object.entries(m.buckets)).forEach((v, b) => {
          map.set(value.name + "{le=" + b + "}", v);
        });
      });
    }
  });*/
  prometheusValues.forEach((x) => {
    self.postMessage(x);
  });

  const timeTaken = Date.now() - start;
  await sleep((100 - timeTaken) / 1000);
}
