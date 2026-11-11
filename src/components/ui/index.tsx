import { forwardRef, type ButtonHTMLAttributes, type InputHTMLAttributes, type ReactNode, type SelectHTMLAttributes, type TextareaHTMLAttributes } from "react";
import { Loader2 } from "lucide-react";

const cx = (...c: (string | false | null | undefined)[]) => c.filter(Boolean).join(" ");
export { cx };

type Variant = "primary" | "secondary" | "ghost" | "danger";
const VARIANTS: Record<Variant, string> = {
  primary: "bg-accent text-on-accent hover:brightness-110 active:brightness-95",
  secondary: "bg-surface border border-line text-fg hover:bg-surface-hover",
  ghost: "text-fg-2 hover:text-fg hover:bg-surface-hover",
  danger: "bg-danger/10 text-danger border border-danger/30 hover:bg-danger/20",
};

export interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: Variant;
  size?: "sm" | "md" | "lg";
  loading?: boolean;
  icon?: ReactNode;
}

export const Button = forwardRef<HTMLButtonElement, ButtonProps>(function Button(
  { variant = "secondary", size = "md", loading, icon, className, children, disabled, ...rest },
  ref,
) {
  const sizes = { sm: "h-7 px-2.5 text-xs gap-1.5", md: "h-9 px-3.5 text-[13px] gap-2", lg: "h-11 px-5 text-sm gap-2" };
  return (
    <button
      ref={ref}
      disabled={disabled || loading}
      className={cx(
        "inline-flex shrink-0 items-center justify-center rounded-lg font-medium transition-[background,filter,color,opacity] duration-150 disabled:cursor-not-allowed disabled:opacity-50",
        sizes[size],
        VARIANTS[variant],
        className,
      )}
      {...rest}
    >
      {loading ? <Loader2 size={size === "sm" ? 13 : 15} className="animate-spin" /> : icon}
      {children}
    </button>
  );
});

export function IconButton({ label, className, children, ...rest }: ButtonHTMLAttributes<HTMLButtonElement> & { label: string }) {
  return (
    <button
      aria-label={label}
      title={label}
      className={cx("inline-flex h-8 w-8 items-center justify-center rounded-lg text-fg-2 transition-colors hover:bg-surface-hover hover:text-fg disabled:opacity-40", className)}
      {...rest}
    >
      {children}
    </button>
  );
}

export function Toggle({ checked, onChange, disabled, label }: { checked: boolean; onChange: (v: boolean) => void; disabled?: boolean; label?: string }) {
  return (
    <button
      role="switch"
      aria-checked={checked}
      aria-label={label}
      disabled={disabled}
      onClick={() => onChange(!checked)}
      className={cx(
        "relative h-5 w-9 shrink-0 rounded-full transition-colors duration-150 disabled:opacity-50",
        checked ? "bg-accent" : "bg-line",
      )}
    >
      <span className={cx("absolute left-0 top-0.5 h-4 w-4 rounded-full bg-white shadow transition-transform duration-150", checked ? "translate-x-[18px]" : "translate-x-0.5")} />
    </button>
  );
}

const width = (c?: string) => (c && /(^|\s)w-/.test(c) ? "" : "w-full");
const fieldBase =
  "rounded-lg border border-line bg-bg-2 px-3 text-[13px] text-fg placeholder:text-fg-3 transition-colors focus:border-accent/60 focus:outline-none disabled:opacity-50";

export const Input = forwardRef<HTMLInputElement, InputHTMLAttributes<HTMLInputElement>>(function Input({ className, ...rest }, ref) {
  return <input ref={ref} className={cx(fieldBase, width(className), "h-9", className)} {...rest} />;
});

export function Textarea({ className, ...rest }: TextareaHTMLAttributes<HTMLTextAreaElement>) {
  return <textarea className={cx(fieldBase, width(className), "py-2 font-mono text-xs", className)} {...rest} />;
}

export function Select({ className, children, ...rest }: SelectHTMLAttributes<HTMLSelectElement>) {
  return (
    <select className={cx(fieldBase, width(className), "h-9 cursor-pointer appearance-auto pr-2", className)} {...rest}>
      {children}
    </select>
  );
}

export function Card({ className, children, ...rest }: { className?: string; children: ReactNode } & React.HTMLAttributes<HTMLDivElement>) {
  return (
    <div className={cx("rounded-xl border border-line bg-surface", className)} {...rest}>
      {children}
    </div>
  );
}

type Tone = "neutral" | "accent" | "success" | "warning" | "danger";
const TONES: Record<Tone, string> = {
  neutral: "bg-fg-3/15 text-fg-2",
  accent: "bg-accent/15 text-accent",
  success: "bg-success/15 text-success",
  warning: "bg-warning/15 text-warning",
  danger: "bg-danger/15 text-danger",
};
export function Badge({ tone = "neutral", children, className }: { tone?: Tone; children: ReactNode; className?: string }) {
  return <span className={cx("inline-flex items-center gap-1 whitespace-nowrap rounded-md px-1.5 py-0.5 text-[11px] font-medium", TONES[tone], className)}>{children}</span>;
}

export function ProgressBar({ value, indeterminate }: { value?: number | null; indeterminate?: boolean }) {
  const pct = value == null ? 0 : Math.max(0, Math.min(100, value * 100));
  return (
    <div className="relative h-1.5 w-full overflow-hidden rounded-full bg-line" role="progressbar" aria-valuenow={indeterminate ? undefined : Math.round(pct)} aria-valuemin={0} aria-valuemax={100}>
      {indeterminate ? (
        <div className="progress-indeterminate absolute inset-y-0 w-1/3 rounded-full bg-accent" />
      ) : (
        <div className="h-full rounded-full bg-accent transition-[width] duration-300" style={{ width: `${pct}%` }} />
      )}
    </div>
  );
}

export function Spinner({ size = 16, className }: { size?: number; className?: string }) {
  return <Loader2 size={size} className={cx("animate-spin text-accent", className)} />;
}

export function Field({ label, description, children, className }: { label: ReactNode; description?: ReactNode; children: ReactNode; className?: string }) {
  return (
    <div className={cx("flex items-center justify-between gap-6 py-3", className)}>
      <div className="min-w-0">
        <div className="text-[13px] font-medium text-fg">{label}</div>
        {description && <div className="mt-0.5 text-xs text-fg-2">{description}</div>}
      </div>
      <div className="flex shrink-0 items-center gap-2">{children}</div>
    </div>
  );
}

export function SectionTitle({ children, action }: { children: ReactNode; action?: ReactNode }) {
  return (
    <div className="mb-3 flex items-center justify-between">
      <h2 className="text-[11px] font-semibold uppercase tracking-wider text-fg-3">{children}</h2>
      {action}
    </div>
  );
}

export function EmptyState({ icon, title, description, action }: { icon?: ReactNode; title: string; description?: string; action?: ReactNode }) {
  return (
    <div className="flex flex-col items-center justify-center px-6 py-10 text-center">
      {icon && <div className="mb-3 text-fg-3">{icon}</div>}
      <div className="text-[13px] font-medium text-fg">{title}</div>
      {description && <div className="mt-1 max-w-sm text-xs text-fg-2">{description}</div>}
      {action && <div className="mt-4">{action}</div>}
    </div>
  );
}

export function PageHeader({ title, description, actions }: { title: string; description?: string; actions?: ReactNode }) {
  return (
    <div className="mb-6 flex items-end justify-between gap-4">
      <div>
        <h1 className="text-xl font-semibold tracking-tight text-fg">{title}</h1>
        {description && <p className="mt-1 text-[13px] text-fg-2">{description}</p>}
      </div>
      {actions && <div className="flex items-center gap-2">{actions}</div>}
    </div>
  );
}

export function Tabs<T extends string>({ value, onChange, items }: { value: T; onChange: (v: T) => void; items: { id: T; label: string }[] }) {
  return (
    <div className="inline-flex rounded-lg border border-line bg-bg-2 p-0.5">
      {items.map((i) => (
        <button
          key={i.id}
          onClick={() => onChange(i.id)}
          className={cx(
            "h-7 rounded-md px-3 text-xs font-medium transition-colors",
            value === i.id ? "bg-surface text-fg shadow-sm ring-1 ring-line" : "text-fg-2 hover:text-fg",
          )}
        >
          {i.label}
        </button>
      ))}
    </div>
  );
}
