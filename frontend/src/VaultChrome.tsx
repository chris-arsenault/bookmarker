import type { ReactElement } from "react";
import { config } from "./config";
import type { LibraryFilters } from "./libraryFilters";

type ScopePatch = Pick<LibraryFilters, "inboxStatus" | "watchStatus" | "archiveStatus">;

type Scope = {
  id: string;
  label: string;
  patch: ScopePatch;
  icon: () => ReactElement;
};

const SCOPE_KEYS = ["inboxStatus", "watchStatus", "archiveStatus"] as const;

const SCOPES: Scope[] = [
  { id: "all", label: "All items", patch: {}, icon: LayersIcon },
  { id: "inbox", label: "Inbox", patch: { inboxStatus: "unsorted" }, icon: InboxIcon },
  { id: "unwatched", label: "Unwatched", patch: { watchStatus: "unwatched" }, icon: EyeIcon },
  { id: "archived", label: "Archived", patch: { archiveStatus: "succeeded" }, icon: ArchiveIcon },
  {
    id: "attention",
    label: "Needs attention",
    patch: { archiveStatus: "failed" },
    icon: AlertIcon,
  },
];

export function VaultRail({
  filters,
  itemCount,
  tagCount,
  onFiltersChange,
}: {
  filters: LibraryFilters;
  itemCount: number;
  tagCount: number;
  onFiltersChange: (filters: LibraryFilters) => void;
}) {
  return (
    <aside className="rail" aria-label="Vault navigation">
      <div className="rail-brand">
        <span className="rail-mark" aria-hidden="true">
          <BookmarkIcon />
        </span>
        <div>
          <p className="eyebrow">Ahara</p>
          <p className="rail-word">{config.productName}</p>
        </div>
      </div>
      <nav className="rail-scopes" aria-label="Saved views">
        <p className="eyebrow rail-heading">Views</p>
        {SCOPES.map((scope) => (
          <ScopeButton
            filters={filters}
            key={scope.id}
            onFiltersChange={onFiltersChange}
            scope={scope}
          />
        ))}
      </nav>
      <div className="rail-foot">
        <div className="rail-stat">
          <strong>{itemCount}</strong>
          <span>in view</span>
        </div>
        <div className="rail-stat">
          <strong>{tagCount}</strong>
          <span>tags</span>
        </div>
      </div>
    </aside>
  );
}

function ScopeButton({
  scope,
  filters,
  onFiltersChange,
}: {
  scope: Scope;
  filters: LibraryFilters;
  onFiltersChange: (filters: LibraryFilters) => void;
}) {
  const Icon = scope.icon;
  const active = scopeMatches(filters, scope.patch);
  const select = () =>
    onFiltersChange({
      ...filters,
      inboxStatus: undefined,
      watchStatus: undefined,
      archiveStatus: undefined,
      ...scope.patch,
    });
  return (
    <button className={active ? "scope active" : "scope"} onClick={select} type="button">
      <Icon />
      <span>{scope.label}</span>
    </button>
  );
}

export function AppBar({
  filters,
  savedCount,
  activeFilters,
}: {
  filters: LibraryFilters;
  savedCount: number;
  activeFilters: number;
}) {
  const scope = SCOPES.find((candidate) => scopeMatches(filters, candidate.patch));
  return (
    <header className="appbar">
      <div className="appbar-id">
        <p className="eyebrow">{config.productName} Vault</p>
        <h2 className="appbar-title">{scope?.label ?? "Filtered view"}</h2>
      </div>
      <p className="appbar-summary">
        <b>{savedCount}</b> saved · {activeFilters} active{" "}
        {activeFilters === 1 ? "filter" : "filters"}
      </p>
    </header>
  );
}

function scopeMatches(filters: LibraryFilters, patch: ScopePatch) {
  return SCOPE_KEYS.every((key) => normalizeStatus(filters[key]) === normalizeStatus(patch[key]));
}

function normalizeStatus(value: string | undefined) {
  return value && value !== "all" ? value : undefined;
}

/* ---- inline icons (1.6 stroke, currentColor) ------------------------ */

function svgProps() {
  return {
    "aria-hidden": true,
    focusable: false,
    viewBox: "0 0 24 24",
    fill: "none",
    stroke: "currentColor",
    strokeWidth: 1.6,
    strokeLinecap: "round" as const,
    strokeLinejoin: "round" as const,
  };
}

function BookmarkIcon() {
  return (
    <svg {...svgProps()} fill="currentColor" stroke="none">
      <path d="M6 3h12a1 1 0 0 1 1 1v17l-7-4-7 4V4a1 1 0 0 1 1-1Z" />
    </svg>
  );
}

function LayersIcon() {
  return (
    <svg {...svgProps()}>
      <path d="M12 3 3 8l9 5 9-5-9-5Z" />
      <path d="M3 14l9 5 9-5" />
    </svg>
  );
}

function InboxIcon() {
  return (
    <svg {...svgProps()}>
      <path d="M3 12h5l2 3h4l2-3h5" />
      <path d="M5 5h14l2 7v6a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1v-6l2-7Z" />
    </svg>
  );
}

function EyeIcon() {
  return (
    <svg {...svgProps()}>
      <path d="M2 12s4-7 10-7 10 7 10 7-4 7-10 7S2 12 2 12Z" />
      <circle cx="12" cy="12" r="3" />
    </svg>
  );
}

function ArchiveIcon() {
  return (
    <svg {...svgProps()}>
      <rect x="3" y="4" width="18" height="4" rx="1" />
      <path d="M5 8v11a1 1 0 0 0 1 1h12a1 1 0 0 0 1-1V8" />
      <path d="M10 12h4" />
    </svg>
  );
}

function AlertIcon() {
  return (
    <svg {...svgProps()}>
      <path d="M12 4 2.5 20h19L12 4Z" />
      <path d="M12 10v4" />
      <path d="M12 17h.01" />
    </svg>
  );
}
