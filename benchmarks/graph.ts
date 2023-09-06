import * as d3 from "https://cdn.jsdelivr.net/npm/d3@7/+esm";
import { JSDOM } from "https://jspm.dev/jsdom";

type DataPoint = {
  ts: number;
  value: number;
  date: Date;
};
export function drawGraph(data: DataPoint[], title: string) {
  const context = {
    jsdom: new JSDOM(""),
  };
  const { window } = context.jsdom;
  const { document } = window;

  const body = d3.select(document).select("body");

  const width = 400;
  const height = 200;

  const marginTop = 20;
  const marginRight = 30;
  const marginBottom = 30;
  const marginLeft = 60;

  const svg = body
    .append("svg")
    .attr("width", width + marginLeft + marginRight)
    .attr("height", height + marginTop + marginBottom)
    .append("g")
    .attr("transform", "translate(" + marginLeft + "," + marginTop + ")");

  const x = d3
    .scaleTime()
    .domain(
      d3.extent(data, function (d: DataPoint) {
        return d.date;
      })
    )
    .nice()
    .range([0, width]);

  const xAxis = d3.axisBottom(x).ticks(5);

  svg
    .append("g")
    .attr("transform", "translate(0," + height + ")")
    .call(xAxis);

  // Add Y axis
  const y = d3
    .scaleLinear()
    .domain([
      0,
      d3.max(data, function (d: DataPoint) {
        return +d.value;
      }) + 100,
    ])
    .range([height, 0]);
  svg.append("g").call(d3.axisLeft(y));

  svg
    .append("text")
    .attr("text-anchor", "middle")
    .attr("style", "font-size: 12px")
    .attr("transform", "rotate(-90)")
    .attr("y", -marginLeft + 20)
    .attr("x", -height / 2)
    .text("[jobs/s]");

  svg
    .append("text")
    .attr("text-anchor", "middle")
    .attr("style", "font-size: 16px")
    .attr("y", 0)
    .attr("x", width / 2)
    .text(title);

  // Add the line
  svg
    .append("path")
    .datum(data)
    .attr("fill", "none")
    .attr("stroke", "steelblue")
    .attr("stroke-width", 1.5)
    .attr(
      "d",
      d3.line(
        function (d: DataPoint) {
          return x(d.date);
        },
        function (d: DataPoint) {
          return y(d.value);
        }
      )
    );

  return body.node().innerHTML;
}

if (import.meta.main) {
  Deno.exit(0);
}
