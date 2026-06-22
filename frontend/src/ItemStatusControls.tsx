import { useState, type FocusEvent } from "react";
import {
  AlertTriangle,
  Archive,
  CalendarDays,
  CheckCircle2,
  Clock3,
  Eye,
  EyeOff,
  Inbox,
  type LucideIcon,
} from "lucide-react";
import { formatDate } from "./dateDisplay";
import type { ArchiveStatus, InboxStatus, LibraryItemSummary, WatchStatus } from "./types";

type StatusOption<T extends string> = {
  label: string;
  value: T;
  icon: LucideIcon;
};

const watchOptions: StatusOption<WatchStatus>[] = [
  { label: "Unwatched", value: "unwatched", icon: EyeOff },
  { label: "Watched", value: "watched", icon: Eye },
];

const inboxOptions: StatusOption<InboxStatus>[] = [
  { label: "Unsorted", value: "unsorted", icon: Inbox },
  { label: "Organized", value: "organized", icon: CheckCircle2 },
];

export function ItemStatusControls({
  summary,
  disabled,
  onWatchStatus,
  onInboxStatus,
}: {
  summary: LibraryItemSummary;
  disabled: boolean;
  onWatchStatus: (status: WatchStatus) => void;
  onInboxStatus: (status: InboxStatus) => void;
}) {
  return (
    <div className="detail-icon-strip">
      <span aria-label={`Added ${formatDate(summary.created_at)}`} className="detail-icon-pill">
        <CalendarDays aria-hidden="true" />
        <time>{formatDate(summary.created_at)}</time>
      </span>
      <ArchiveStatusIcon status={summary.archive_status} />
      <StatusIconControl
        current={summary.watch_status}
        disabled={disabled}
        label="Watch status"
        options={watchOptions}
        onSelect={onWatchStatus}
      />
      <StatusIconControl
        current={summary.inbox_status}
        disabled={disabled}
        label="Inbox status"
        options={inboxOptions}
        onSelect={onInboxStatus}
      />
    </div>
  );
}

function ArchiveStatusIcon({ status }: { status: ArchiveStatus }) {
  return (
    <span aria-label={`Archive ${status}`} className={`detail-icon-pill archive ${status}`}>
      {archiveStatusIcon(status)}
    </span>
  );
}

function StatusIconControl<T extends string>({
  label,
  current,
  options,
  disabled,
  onSelect,
}: {
  label: string;
  current: T;
  options: StatusOption<T>[];
  disabled: boolean;
  onSelect: (value: T) => void;
}) {
  const [open, setOpen] = useState(false);
  const selected = activeStatusOption(options, current);
  const Icon = selected.icon;
  return (
    <div className="status-popover-host" onBlur={(event) => closeStatusPopover(event, setOpen)}>
      <button
        aria-expanded={open}
        aria-label={`${label}: ${selected.label}`}
        className="detail-icon-button"
        disabled={disabled}
        onClick={() => setOpen(!open)}
        title={selected.label}
        type="button"
      >
        <Icon aria-hidden="true" />
      </button>
      {open ? (
        <StatusPopover
          current={current}
          options={options}
          onSelect={(value) => {
            setOpen(false);
            onSelect(value);
          }}
        />
      ) : null}
    </div>
  );
}

function StatusPopover<T extends string>({
  current,
  options,
  onSelect,
}: {
  current: T;
  options: StatusOption<T>[];
  onSelect: (value: T) => void;
}) {
  return (
    <div className="status-popover">
      {options.map((option) => (
        <StatusOptionButton
          current={current}
          key={option.value}
          option={option}
          onSelect={onSelect}
        />
      ))}
    </div>
  );
}

function StatusOptionButton<T extends string>({
  option,
  current,
  onSelect,
}: {
  option: StatusOption<T>;
  current: T;
  onSelect: (value: T) => void;
}) {
  const Icon = option.icon;
  return (
    <button
      aria-pressed={option.value === current}
      onClick={() => onSelect(option.value)}
      type="button"
    >
      <Icon aria-hidden="true" />
      <span>{option.label}</span>
    </button>
  );
}

function activeStatusOption<T extends string>(options: StatusOption<T>[], current: T) {
  return options.find((option) => option.value === current) ?? options[0];
}

function archiveStatusIcon(status: ArchiveStatus) {
  if (status === "succeeded") {
    return <CheckCircle2 aria-hidden="true" />;
  }
  if (status === "failed") {
    return <AlertTriangle aria-hidden="true" />;
  }
  return status === "pending" ? <Clock3 aria-hidden="true" /> : <Archive aria-hidden="true" />;
}

function closeStatusPopover(event: FocusEvent<HTMLDivElement>, setOpen: (open: boolean) => void) {
  const nextTarget = event.relatedTarget;
  if (!(nextTarget instanceof Node) || !event.currentTarget.contains(nextTarget)) {
    setOpen(false);
  }
}
