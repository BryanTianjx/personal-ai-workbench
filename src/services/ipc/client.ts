/*
 * Typed IPC client — GUI Design Specification R1 (07-ipc-integration-contract.md §1, §4).
 * Every backend call goes through callCommand so error normalization is centralized.
 */
import { invoke } from "@tauri-apps/api/core";
import { toWorkbenchError } from "./errors";

/**
 * Dev-only mock transport: enabled solely when running under `vite dev` with an
 * explicit `?mock=1` query flag, for browser-based visual acceptance without the
 * Tauri backend (10-acceptance-test-plan.md). The DEV constant is statically
 * replaced at build time, so the dynamic import is unreachable in production
 * bundles and the mock module is never emitted.
 */
const USE_MOCK =
  import.meta.env.DEV &&
  new URLSearchParams(window.location.search).has("mock");

export async function callCommand<TResponse>(
  command: string,
  args?: Record<string, unknown>,
): Promise<TResponse> {
  if (USE_MOCK) {
    const { mockInvoke } = await import("./dev-mock");
    return mockInvoke<TResponse>(command, args ?? {});
  }
  try {
    return await invoke<TResponse>(command, args);
  } catch (cause) {
    throw toWorkbenchError(cause);
  }
}
