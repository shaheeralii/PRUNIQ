import styles from "./AlertBanner.module.css";

interface AlertBannerProps {
  tone: "error" | "warn" | "info";
  title: string;
  detail?: string | null;
  items?: string[];
  onDismiss?: () => void;
}

export function AlertBanner({ tone, title, detail, items, onDismiss }: AlertBannerProps) {
  const toneClass = tone === "info" ? "" : styles[tone];
  return (
    <div
      className={`${styles.banner} ${toneClass}`}
      role={tone === "error" ? "alert" : "status"}
    >
      <div className={styles.header}>
        <span className={styles.title}>{title}</span>
        {onDismiss && (
          <button type="button" className={styles.dismiss} onClick={onDismiss}>
            Dismiss
          </button>
        )}
      </div>
      {detail && <p className={styles.detail}>{detail}</p>}
      {items && items.length > 0 && (
        <ul className={styles.list}>
          {items.map((item, idx) => (
            <li key={`${idx}-${item}`}>{item}</li>
          ))}
        </ul>
      )}
    </div>
  );
}
