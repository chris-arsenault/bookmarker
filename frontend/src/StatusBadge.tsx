import type { ArchiveStatus } from "./types";

export function StatusBadge({ status }: { status: ArchiveStatus }) {
  return <span className={`status-badge ${status}`}>{statusLabel(status)}</span>;
}

function statusLabel(status: ArchiveStatus) {
  return status === "not_applicable" ? "snippet" : status;
}
