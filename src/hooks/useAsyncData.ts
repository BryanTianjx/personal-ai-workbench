/*
 * Small query-lifecycle hook for IPC reads. Keeps loading / error / data state
 * per call site so one failed panel can degrade independently (08 §1 partial state).
 * Re-running is always a safe read; mutations never go through this hook.
 *
 * Staleness guard: every effect run takes a fresh sequence number, and a
 * response may only update state while it is still the latest request
 * (PAW-AUDIT-004 fix). A shared boolean flag is not safe here — a new effect
 * re-arms the flag before a superseded request settles, letting stale
 * workspace data overwrite the current workspace.
 */
import { useCallback, useEffect, useRef, useState } from "react";
import { describe } from "../services/ipc/errors";

export interface AsyncData<T> {
  data: T | null;
  loading: boolean;
  error: string | null;
  reload: () => void;
}

export function useAsyncData<T>(fetcher: () => Promise<T>, deps: unknown[]): AsyncData<T> {
  const [data, setData] = useState<T | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [tick, setTick] = useState(0);
  const fetcherRef = useRef(fetcher);
  fetcherRef.current = fetcher;
  const seqRef = useRef(0);

  useEffect(() => {
    const seq = ++seqRef.current;
    setLoading(true);
    setError(null);
    fetcherRef
      .current()
      .then((result) => {
        if (seq === seqRef.current) setData(result);
      })
      .catch((cause) => {
        if (seq === seqRef.current) setError(describe(cause));
      })
      .finally(() => {
        if (seq === seqRef.current) setLoading(false);
      });
    // No cleanup guard: a superseded request is rejected by the sequence
    // check above even after its effect has been torn down.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [...deps, tick]);

  const reload = useCallback(() => setTick((t) => t + 1), []);
  return { data, loading, error, reload };
}
