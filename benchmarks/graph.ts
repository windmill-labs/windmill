import * as d3 from "https://cdn.jsdelivr.net/npm/d3@7/+esm";
import { JSDOM } from "https://jspm.dev/jsdom@22";

type DataPoint = {
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

  let svg = body
    .append("svg")
    .attr("xmlns", "http://www.w3.org/2000/svg")
    .attr("width", width + marginLeft + marginRight)
    .attr("height", height + marginTop + marginBottom);

  svg
    .append("rect")
    .attr("width", "100%")
    .attr("height", "100%")
    .attr("fill", "white");

  svg = svg
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
      }) * 1.5,
    ])
    .range([height, 0])
    .nice();
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

interface DataPointMulti extends DataPoint {
  kind: string;
}

export function drawGraphMulti(data: DataPointMulti[], title: string) {
  const context = {
    jsdom: new JSDOM(""),
  };
  const { window } = context.jsdom;
  const { document } = window;

  const body = d3.select(document).select("body");

  const width = 400;
  const height = 200;

  const marginTop = 20;
  const marginRight = 100;
  const marginBottom = 30;
  const marginLeft = 60;

  let svg = body
    .append("svg")
    .attr("xmlns", "http://www.w3.org/2000/svg")
    .attr("width", width + marginLeft + marginRight)
    .attr("height", height + marginTop + marginBottom);

  svg
    .append("rect")
    .attr("width", "100%")
    .attr("height", "100%")
    .attr("fill", "white");

  svg = svg
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
      }) * 1.5,
    ])
    .range([height, 0])
    .nice();
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

  const sumstat = d3.group(data, function (d: DataPointMulti) {
    return d.kind;
  });

  const keys = Array.from(sumstat.keys());

  const color = d3
    .scaleOrdinal()
    .domain(keys)
    .range([
      "#e41a1c",
      "#377eb8",
      "#4daf4a",
      "#984ea3",
      "#ff7f00",
      "#ffff33",
      "#a65628",
      "#f781bf",
      "#999999",
    ]);

  // Add the line
  svg
    .selectAll("path.line")
    .data(sumstat)
    .join("path")
    .attr("class", "line")
    .attr("fill", "none")
    .attr("stroke", function (d) {
      return color(d[0]);
    })
    .attr("stroke-width", 1.5)
    .attr("d", (d) => {
      return d3
        .line()
        .x((d) => x(d.date))
        .y((d) => y(d.value))(d[1]);
    });

  const size = 15;
  svg
    .selectAll(".dot")
    .data(keys)
    .enter()
    .append("rect")
    .attr("class", "dot")
    .attr("x", 400)
    .attr("y", function (d, i) {
      return 5 + i * (size + 5);
    })
    .attr("width", size)
    .attr("height", size)
    .style("fill", function (d) {
      return color(d);
    });
  svg
    .selectAll(".label")
    .data(keys)
    .enter()
    .append("text")
    .attr("class", "label")
    .attr("x", 400 + size * 1.2)
    .attr("y", function (d, i) {
      return 5 + i * (size + 5) + size / 2;
    })
    .style("fill", function (d) {
      return color(d);
    })
    .text(function (d) {
      return d;
    })
    .attr("text-anchor", "left")
    .style("alignment-baseline", "middle");

  return body.node().innerHTML;
}

if (import.meta.main) {
  const svg = drawGraph(
    [
      {
        value: 10,
        date: new Date(86400000),
      },
      {
        value: 12,
        date: new Date(86400000 * 2),
      },
    ],
    "test"
  );

  const svg2 = drawGraphMulti(
    [
      {
        value: 10,
        date: new Date(86400000),
        kind: "test",
      },
      {
        value: 12,
        date: new Date(86400000 * 2),
        kind: "test",
      },
      {
        value: 8,
        date: new Date(86400000),
        kind: "test2",
      },
      {
        value: 9,
        date: new Date(86400000 * 2),
        kind: "test2",
      },
    ],
    "test"
  );

  console.log(svg);
  console.log(svg2);
  Deno.exit(0);
}
