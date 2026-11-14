import { ChevronRight, FileText, LayoutGrid, Search, SlidersHorizontal, X } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import { search, type SearchEntry } from "../search";
import { useApp } from "../stores/app";
import { cx } from "./ui";

const KIND_ICON = { page: LayoutGrid, setting: SlidersHorizontal, feature: FileText };

function Results({ results, active, onPick, onHover, query }: { results: SearchEntry[]; active: number; onPick: (e: SearchEntry) => void; onHover: (i: number) => void; query: string }) {
  if (!query.trim()) return null;
  return (
    <div role="listbox" aria-label="Search results" className="max-h-[360px] overflow-y-auto p-1">
      {results.length === 0 && <div className="px-3 py-6 text-center text-xs text-fg-3">No results for “{query.trim()}”</div>}
      {results.map((r, i) => {
        const Icon = KIND_ICON[r.kind];
        return (
          <button
            key={`${r.title}|${r.path.join("/")}`}
            role="option"
            aria-selected={i === active}
            onMouseEnter={() => onHover(i)}
            onMouseDown={(e) => e.preventDefault()}
            onClick={() => onPick(r)}
            className={cx("flex w-full items-center gap-3 rounded-md px-2.5 py-2 text-left", i === active ? "bg-fg/[0.07]" : "hover:bg-fg/[0.04]")}
          >
            <Icon size={15} className={cx("shrink-0", i === active ? "text-accent" : "text-fg-3")} />
            <div className="min-w-0 flex-1">
              <div className="truncate text-[13px] text-fg">{r.title}</div>
              {r.path.length > 0 && (
                <div className="flex items-center gap-0.5 truncate text-[11px] text-fg-3">
                  {r.path.map((p, j) => (
                    <span key={j} className="inline-flex items-center gap-0.5">
                      {j > 0 && <ChevronRight size={10} />}
                      {p}
                    </span>
                  ))}
                </div>
              )}
            </div>
          </button>
        );
      })}
    </div>
  );
}

/** Search pages and settings. `collapsed` shows an icon that opens a centered search panel. */
export function SearchBox({ collapsed }: { collapsed: boolean }) {
  const { navigate } = useApp();
  const [query, setQuery] = useState("");
  const [open, setOpen] = useState(false);
  const [active, setActive] = useState(0);
  const input = useRef<HTMLInputElement>(null);
  const results = useMemo(() => search(query), [query]);

  useEffect(() => setActive(0), [query]);

  // Ctrl+K or Ctrl+F focuses search from anywhere.
  useEffect(() => {
    const h = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && (e.key === "k" || e.key === "f")) {
        e.preventDefault();
        setOpen(true);
        window.setTimeout(() => input.current?.focus(), 0);
      }
    };
    window.addEventListener("keydown", h);
    return () => window.removeEventListener("keydown", h);
  }, []);

  const pick = (e: SearchEntry) => {
    navigate(e.page, e.section || e.setting ? { section: e.section, setting: e.setting } : undefined);
    setQuery("");
    setOpen(false);
    input.current?.blur();
  };

  const onKey = (e: React.KeyboardEvent) => {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      setActive((a) => Math.min(a + 1, results.length - 1));
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setActive((a) => Math.max(a - 1, 0));
    } else if (e.key === "Enter" && results[active]) {
      pick(results[active]);
    } else if (e.key === "Escape") {
      setQuery("");
      setOpen(false);
      input.current?.blur();
    }
  };

  const field = (
    <div className="relative">

      <input
        ref={input}
        value={query}
        onChange={(e) => {
          setQuery(e.target.value);
          setOpen(true);
        }}
        onFocus={() => setOpen(true)}
        onBlur={() => !collapsed && window.setTimeout(() => setOpen(false), 120)}
        onKeyDown={onKey}
        placeholder="Search"
        aria-label="Search Aeolus"
        spellCheck={false}
        title="Search (Ctrl+K)"
        className="h-8 w-full rounded-md border border-line border-b-fg-3/50 bg-control pl-3 pr-9 text-[13px] text-fg placeholder:text-fg-3 transition-colors hover:bg-surface-hover focus:border-b-accent focus:bg-bg focus:outline-none focus-visible:outline-none"
      />
      {query ? (
        <button aria-label="Clear search" onMouseDown={(e) => e.preventDefault()} onClick={() => setQuery("")} className="absolute right-1.5 top-1/2 flex h-5 w-5 -translate-y-1/2 items-center justify-center rounded text-fg-3 hover:text-fg">
          <X size={13} />
        </button>
      ) : (
        <Search size={14} className="pointer-events-none absolute right-2.5 top-1/2 -translate-y-1/2 text-fg-2" />
      )}
    </div>
  );

  if (collapsed) {
    return (
      <>
        <button
          aria-label="Search Aeolus"
          title="Search (Ctrl+K)"
          onClick={() => {
            setOpen(true);
            window.setTimeout(() => input.current?.focus(), 0);
          }}
          className="mx-auto flex h-9 w-9 items-center justify-center rounded-md text-fg-2 hover:bg-fg/[0.06] hover:text-fg"
        >
          <Search size={17} />
        </button>
        {open && (
          <div className="fixed inset-0 z-50 flex justify-center bg-black/40 pt-24" onMouseDown={() => setOpen(false)}>
            <div className="h-fit w-[440px] overflow-hidden rounded-lg border border-line bg-surface p-2 shadow-[var(--shadow)]" onMouseDown={(e) => e.stopPropagation()}>
              {field}
              <Results results={results} active={active} onPick={pick} onHover={setActive} query={query} />
            </div>
          </div>
        )}
      </>
    );
  }

  return (
    <div className="relative">
      {field}
      {open && query.trim() && (
        <div className="absolute left-0 top-10 z-50 w-[340px] overflow-hidden rounded-lg border border-line bg-surface shadow-[var(--shadow)]">
          <Results results={results} active={active} onPick={pick} onHover={setActive} query={query} />
        </div>
      )}
    </div>
  );
}

/** After a search jump, scroll to the matching setting and briefly highlight it. */
export function useSearchHighlight() {
  const { target, page } = useApp();
  useEffect(() => {
    if (!target?.setting) return;
    let tries = 0;
    let timer = 0;
    const find = () => {
      const el = document.querySelector<HTMLElement>(`[data-setting="${CSS.escape(target.setting!)}"]`);
      if (el) {
        el.scrollIntoView?.({ block: "center", behavior: "smooth" });
        el.classList.remove("search-found");
        void el.offsetWidth;
        el.classList.add("search-found");
        window.setTimeout(() => el.classList.remove("search-found"), 1900);
      } else if (tries++ < 15) {
        timer = window.setTimeout(find, 80);
      }
    };
    timer = window.setTimeout(find, 60);
    return () => window.clearTimeout(timer);
  }, [target, page]);
}
