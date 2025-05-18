/// <reference types="vite/client" />
import type * as libsync from "./store/sync";

export type Endpoints = {
  status: URL;
  login: URL;
  websocket: URL;
};

export function getUrls(): Endpoints {
  let backendHost: string;
  const buildParam: unknown = import.meta.env.VITE_BACKEND_HOST;
  if (typeof buildParam !== "string" || !buildParam) {
    throw new Error("BUG: bad build: missing required parameter: backend host");
  } else {
    backendHost = buildParam;
  }

  let schemeHttp: "http" | "https";
  let schemeWebSocket: "ws" | "wss";
  const origin: URL = new URL(window.origin);
  if (origin.protocol === "http:") {
    schemeHttp = "http";
    schemeWebSocket = "ws";
  } else {
    schemeHttp = "https";
    schemeWebSocket = "wss";
  }

  return {
    status: new URL(`${schemeHttp}://${backendHost}/status`),
    login: new URL(`${schemeHttp}://${backendHost}/login`),
    websocket: new URL(`${schemeWebSocket}://${backendHost}/sock`),
  };
}

export function getRootElement(): HTMLElement {
  const element: null | HTMLElement = document.getElementById("root");
  if (!element) {
    throw new Error("BUG: bad build: missing required HTML root element");
  }
  return element;
}

/**
 * @example "session=R3N2yEX5LxRRxl8gkQJZB+zpgOoTCwvoEzvDqqwhgCQ%3D%7B%22session_id%22%3A%223e24b217-1f31-4be7-88be-a1309353228b%22%7D"
 */
export function getSessionId(cookie: string): null | libsync.Uuid {
  const cookieDelimiterIdx: number = cookie.indexOf("=");
  if (cookieDelimiterIdx < 0) {
    return null;
  }
  const cookieValueEnc: string = cookie.substring(cookieDelimiterIdx + 1);
  const cookieValueDec: string = decodeURIComponent(cookieValueEnc);

  const cookieInnerDelimiterIdx: number = cookieValueDec.indexOf("=");
  if (cookieInnerDelimiterIdx < 0) {
    return null;
  }
  const cookieValueJson: string = cookieValueDec.substring(cookieInnerDelimiterIdx + 1);

  const deserialized: unknown = JSON.parse(cookieValueJson);
  if (typeof deserialized !== "object" || !deserialized || !("session_id" in deserialized)) {
    return null;
  }
  if (typeof deserialized.session_id !== "string") {
    return null;
  }

  const id: libsync.Uuid = deserialized.session_id;
  return id;
}
