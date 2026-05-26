// pipeline
// on res://f/admins/intelligent_s3
// freshness 30m
// Resource showcase: uses the alternate `res://` form of the resource
// reference (the more common form is `$res:`, which `x_py_publish.py`
// exercises). Body-inferred resource read via wmill.getResource so the
// resource appears on both the annotation list AND the body.

import * as wmill from "windmill-client"

export async function main() {
  const cfg = await wmill.getResource("f/admins/intelligent_s3")
  return {
    bucket: cfg?.bucket ?? null,
    endpoint: cfg?.endPoint ?? null,
    region: cfg?.region ?? null,
  }
}
