import { useEffect, useState } from "react";
import { Input, Select, cx } from "./ui";

export const FPS_PRESETS = [30, 60, 120, 144, 165, 240];
export const FPS_MIN = 15;
export const FPS_MAX = 9999;

/** FPS cap picker: a preset list plus a custom number. `null` = the inherit option. */
export function FpsCapInput({ value, onChange, inheritLabel }: { value: number | null; onChange: (v: number | null) => void; inheritLabel: string }) {
  const isCustom = value !== null && !FPS_PRESETS.includes(value);
  const [custom, setCustom] = useState(isCustom);
  const [text, setText] = useState(value !== null ? String(value) : "");
  useEffect(() => {
    setText(value !== null ? String(value) : "");
    if (value !== null && !FPS_PRESETS.includes(value)) setCustom(true);
  }, [value]);
  const n = Number(text);
  const bad = !/^\d+$/.test(text) || n < FPS_MIN || n > FPS_MAX;
  const selectValue = custom ? "custom" : value === null ? "" : String(value);

  return (
    <div className="flex items-center gap-2">
      <Select
        className="w-44"
        aria-label="FPS cap"
        value={selectValue}
        onChange={(e) => {
          const v = e.target.value;
          if (v === "custom") {
            setCustom(true);
            setText(value !== null ? String(value) : "360");
            if (value === null) onChange(360);
            return;
          }
          setCustom(false);
          onChange(v === "" ? null : Number(v));
        }}
      >
        <option value="">{inheritLabel}</option>
        {FPS_PRESETS.map((f) => (
          <option key={f} value={f}>{f} FPS</option>
        ))}
        <option value="custom">Custom…</option>
      </Select>
      {custom && (
        <div className="flex items-center gap-1.5">
          <Input
            aria-label="Custom FPS cap"
            className={cx("w-20 text-right", bad && "border-danger/60")}
            value={text}
            onChange={(e) => setText(e.target.value)}
            onBlur={() => (bad ? setText(value !== null ? String(value) : "") : n !== value && onChange(n))}
          />
          <span className="text-xs text-fg-3">FPS</span>
        </div>
      )}
    </div>
  );
}
