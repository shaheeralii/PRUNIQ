import type { ReactNode } from "react";
import styles from "./StatusBadge.module.css";

export type BadgeTone = "neutral" | "ok" | "warn" | "error" | "accent";

interface StatusBadgeProps {
  tone?: BadgeTone;
  children: ReactNode;
}

export function StatusBadge({ tone = "neutral", children }: StatusBadgeProps) {
  const toneClass = tone === "neutral" ? "" : styles[tone];
  return <span className={`${styles.badge} ${toneClass}`}>{children}</span>;
}
