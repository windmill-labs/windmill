// SVG → PDF rendering. Uses jspdf + svg2pdf.js via npm: specifiers (Deno).
// Used by render_report.ts to also emit a dashboard.pdf alongside dashboard.svg
// so the report is readable in PDF viewers / printable without losing the
// vector format.
//
// Implementation note: svg2pdf.js needs a DOM SVGElement, which Deno doesn't
// have natively. We use jsdom (already used by the chart code) to parse the
// SVG string into a real Element, then pass it to svg2pdf.

import { JSDOM } from "https://jspm.dev/jsdom";
import { jsPDF } from "npm:jspdf@2.5.1";
import "npm:svg2pdf.js@2.2.4";

// jsPDF augments its prototype when svg2pdf.js is imported.
type JsPDFWithSvg = jsPDF & {
  svg(element: Element, opts?: Record<string, unknown>): Promise<jsPDF>;
};

export async function renderSvgToPdf(
  svgPath: string,
  pdfPath: string,
): Promise<void> {
  const svgText = await Deno.readTextFile(svgPath);

  // Parse the SVG into a real DOM Element via jsdom.
  // deno-lint-ignore no-explicit-any
  const dom = new (JSDOM as any)(`<!DOCTYPE html><body>${svgText}</body>`);
  const svgEl = dom.window.document.querySelector("svg");
  if (!svgEl) throw new Error(`no <svg> in ${svgPath}`);

  // Read intrinsic dimensions from the SVG; default to A3 landscape if missing.
  const w = parseFloat(svgEl.getAttribute("width") ?? "1200");
  const h = parseFloat(svgEl.getAttribute("height") ?? "850");

  // 1 SVG unit = 1 pt in jsPDF. Page sized exactly to SVG so nothing crops.
  const pdf = new jsPDF({
    orientation: w >= h ? "landscape" : "portrait",
    unit: "pt",
    format: [w, h],
  }) as JsPDFWithSvg;

  await pdf.svg(svgEl, { x: 0, y: 0, width: w, height: h });
  const bytes = pdf.output("arraybuffer");
  await Deno.writeFile(pdfPath, new Uint8Array(bytes));
  console.log(`[pdf] ${pdfPath}`);
}
