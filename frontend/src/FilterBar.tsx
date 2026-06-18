import type { FilterOption, LibraryFilters } from "./libraryFilters";

type SelectOption = Pick<FilterOption, "value" | "label">;
type UpdateFilter = (key: keyof LibraryFilters, value: string) => void;

const archiveOptions: SelectOption[] = [
  { value: "pending", label: "pending" },
  { value: "succeeded", label: "succeeded" },
  { value: "failed", label: "failed" },
];

const watchOptions: SelectOption[] = [
  { value: "unwatched", label: "unwatched" },
  { value: "watched", label: "watched" },
];

export function FilterBar({
  filters,
  platforms,
  tags,
  onFiltersChange,
}: {
  filters: LibraryFilters;
  platforms: FilterOption[];
  tags: FilterOption[];
  onFiltersChange: (filters: LibraryFilters) => void;
}) {
  const update = (key: keyof LibraryFilters, value: string) =>
    onFiltersChange({ ...filters, [key]: value || undefined } as LibraryFilters);

  return (
    <section className="filter-bar" aria-label="Library filters">
      <TextFilter
        label="Search"
        placeholder=""
        type="search"
        value={filters.q ?? ""}
        onChange={updateText("q", update)}
      />
      <SelectFilter
        label="Platform"
        value={filters.platform ?? ""}
        options={platforms}
        onChange={updateText("platform", update)}
      />
      <SelectFilter
        label="Tag"
        value={filters.tag ?? ""}
        options={tags}
        onChange={updateText("tag", update)}
      />
      <TextFilter
        label="Created from"
        type="text"
        placeholder="2026-06-01T00:00:00Z"
        value={filters.createdFrom ?? ""}
        onChange={updateText("createdFrom", update)}
      />
      <TextFilter
        label="Created to"
        type="text"
        placeholder="2026-06-15T23:59:59Z"
        value={filters.createdTo ?? ""}
        onChange={updateText("createdTo", update)}
      />
      <SelectFilter
        label="Archive"
        value={filters.archiveStatus ?? ""}
        options={archiveOptions}
        onChange={updateText("archiveStatus", update)}
      />
      <SelectFilter
        label="Watch status"
        value={filters.watchStatus ?? ""}
        options={watchOptions}
        onChange={updateText("watchStatus", update)}
      />
    </section>
  );
}

function TextFilter({
  label,
  type,
  placeholder,
  value,
  onChange,
}: {
  label: string;
  type: "search" | "text";
  placeholder: string;
  value: string;
  onChange: (value: string) => void;
}) {
  return (
    <label>
      {label}
      <input
        placeholder={placeholder}
        type={type}
        value={value}
        onChange={(event) => onChange(event.target.value)}
      />
    </label>
  );
}

function SelectFilter({
  label,
  value,
  options,
  onChange,
}: {
  label: string;
  value: string;
  options: SelectOption[];
  onChange: (value: string) => void;
}) {
  return (
    <label>
      {label}
      <select value={value} onChange={(event) => onChange(event.target.value)}>
        <option value="">All</option>
        {options.map((option) => (
          <option key={option.value} value={option.value}>
            {option.label}
          </option>
        ))}
      </select>
    </label>
  );
}

function updateText(key: keyof LibraryFilters, update: UpdateFilter) {
  return (value: string) => update(key, value);
}
