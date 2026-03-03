import { OpenAPI } from "../../gen/index.ts";

export function setClient(token?: string, baseUrl?: string) {
  if (baseUrl === undefined) {
    baseUrl = process.env["BASE_INTERNAL_URL"] ??
      process.env["BASE_URL"] ??
      "http://localhost:8000";
  }
  if (token === undefined) {
    token = process.env["WM_TOKEN"] ?? "no_token";
  }
  OpenAPI.WITH_CREDENTIALS = true;
  OpenAPI.TOKEN = token;
  OpenAPI.BASE = baseUrl + "/api";
}
