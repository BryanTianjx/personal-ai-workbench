/*
 * Small query-lifecycle hook for IPC reads. Keeps loading / error / data state
 * per call site so one failed panel can degrade independently (08 §1 partial state).
 * Re-running is always a safe read; mutations never go through this hook.
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
  const activeRef = useRef(true);

  useEffect(() => {
    activeRef.current = true;
    setLoading(true);
    setError(null);
    fetcherRef
      .current()
      .then((result) => {
        if (activeRef.current) setData(result);
      })
      .catch((cause) => {
        if (activeRef.current) setError(describe(cause));
      })
      .finally(() => {
        if (activeRef.current) setLoading(false);
      });
    return () => {
      activeRef.current = false;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [...deps, tick]);

  const reload = useCallback(() => setTick((t) => t + 1), []);
  return { data, loading, error, reload };
}
