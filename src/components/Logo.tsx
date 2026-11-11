import logo from "../assets/aeolus-logo.png";
import { cx } from "./ui";

/** The official Aeolus wind mark (from the supplied artwork, unmodified). */
export function Logo({ size = 28, className, animated }: { size?: number; className?: string; animated?: boolean }) {
  return (
    <img
      src={logo}
      width={size}
      height={size}
      alt="Aeolus"
      draggable={false}
      className={cx("select-none object-contain", animated && "logo-drift", className)}
    />
  );
}
