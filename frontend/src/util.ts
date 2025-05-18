/// <reference types="vite/client" />
export type Endpoints = {
  status: string;
  login: string;
  websocket: string;
};

export function getUrls(): Endpoints {
  if (import.meta.env.MODE === "development") {
    const backendHost = import.meta.env.VITE_BACKEND_HOST;
    if (!backendHost) {
      throw new Error("bad build: missing required parameter: backend host");
    } else {
      return {
        status: `http://${backendHost}/status`,
        login: `http://${backendHost}/login`,
        websocket: `ws://${backendHost}/sock`,
      };
    }
  } else {
    return {
      status: "/status",
      login: "/login",
      websocket: "/sock",
    };
  }
}

export function getRootElement(): HTMLElement {
  const element: null | HTMLElement = document.getElementById("root");
  if (!element) {
    throw new Error("bad build: missing required HTML root element");
  }
  return element;
}
