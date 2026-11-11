import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { EVENTS, inTauri } from "../services/api";

/** Subscribe to backend "changed" notifications for the given areas. */
export function useChanged(areas: string[], fn: () => void) {
  const ref = useRef(fn);
  ref.current = fn;
  const key = areas.join(",");
  useEffect(() => {
    if (!inTauri()) return;
    let un: (() => void) | undefined;
    let alive = true;
    listen<string>(EVENTS.changed, (e) => {
      if (e.payload === "all" || key.split(",").includes(e.payload)) ref.current();
    }).then((u) => {
      if (alive) un = u;
      else u();
    });
    return () => {
      alive = false;
      un?.();
    };
  }, [key]);
}

export function useResource<T>(fetcher: () => Promise<T>, areas: string[] = [], deps: unknown[] = []) {
  const [data, setData] = useState<T | null>(null);
  const [error, setError] = useState<unknown>(null);
  const [loading, setLoading] = useState(true);
  const seq = useRef(0);
  const reload = useCallback(async () => {
    const id = ++seq.current;
    setLoading(true);
    try {
      const d = await fetcher();
      if (id === seq.current) {
        setData(d);
        setError(null);
      }
    } catch (e) {
      if (id === seq.current) setError(e);
    } finally {
      if (id === seq.current) setLoading(false);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, deps);
  useEffect(() => {
    void reload();
  }, [reload]);
  useChanged(areas, () => void reload());
  return { data, error, loading, reload, setData };
}
