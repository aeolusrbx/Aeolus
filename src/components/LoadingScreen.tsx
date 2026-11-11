import { Logo } from "./Logo";

export function LoadingScreen({ inline }: { inline?: boolean }) {
  return (
    <div className={inline ? "flex h-64 items-center justify-center" : "flex h-full flex-col items-center justify-center gap-5 bg-bg"} data-tauri-drag-region>
      <Logo size={inline ? 36 : 84} animated className={inline ? "opacity-60" : ""} />
      {!inline && (
        <div className="h-1 w-40 overflow-hidden rounded-full bg-line">
          <div className="progress-indeterminate h-full w-1/3 rounded-full bg-accent" />
        </div>
      )}
    </div>
  );
}
