import { useId, useState, type FocusEvent, type KeyboardEvent } from "react";
import type { TagCorpusEntry } from "./types";

type TagOption = {
  displayName: string;
  normalizedName: string;
  usageCount: number;
};

export function TagEditor({
  availableTags,
  selectedTags,
  disabled,
  onChange,
}: {
  availableTags: TagCorpusEntry[];
  selectedTags: string[];
  disabled: boolean;
  onChange: (tags: string[]) => void;
}) {
  const listId = useId();
  const [query, setQuery] = useState("");
  const [open, setOpen] = useState(false);
  const options = tagOptions(availableTags, selectedTags, query);
  const createTag = creatableTag(query, availableTags, selectedTags);

  const select = (tag: string) => {
    onChange(dedupeTags([...selectedTags, tag]));
    setQuery("");
    setOpen(true);
  };

  return (
    <div className="tag-selector" onBlur={(event) => closeOnBlur(event, setOpen)}>
      <div className={open ? "tag-selector-input open" : "tag-selector-input"}>
        {selectedTags.map((tag) => (
          <SelectedTagChip
            disabled={disabled}
            key={normalizeTag(tag)}
            tag={tag}
            onRemove={() => onChange(removeTag(selectedTags, tag))}
          />
        ))}
        <input
          aria-label="Tags"
          disabled={disabled}
          onChange={(event) => {
            setQuery(event.target.value);
            setOpen(true);
          }}
          onClick={() => setOpen(true)}
          onFocus={() => setOpen(true)}
          onKeyDown={(event) =>
            handleTagKeyDown(event, query, selectedTags, select, onChange, setOpen)
          }
          placeholder={selectedTags.length === 0 ? "Tags" : ""}
          type="text"
          value={query}
        />
      </div>
      {open ? (
        <TagDropdown
          createTag={createTag}
          disabled={disabled}
          id={listId}
          options={options}
          onSelect={select}
        />
      ) : null}
    </div>
  );
}

function SelectedTagChip({
  tag,
  disabled,
  onRemove,
}: {
  tag: string;
  disabled: boolean;
  onRemove: () => void;
}) {
  return (
    <span className="tag-selector-chip">
      {tag}
      <button aria-label={`Remove ${tag}`} disabled={disabled} onClick={onRemove} type="button">
        &times;
      </button>
    </span>
  );
}

function TagDropdown({
  id,
  options,
  createTag,
  disabled,
  onSelect,
}: {
  id: string;
  options: TagOption[];
  createTag: string | null;
  disabled: boolean;
  onSelect: (tag: string) => void;
}) {
  if (options.length === 0 && !createTag) {
    return (
      <div className="tag-selector-dropdown" id={id}>
        <p>No tags available</p>
      </div>
    );
  }
  return (
    <div className="tag-selector-dropdown" id={id}>
      {createTag ? (
        <TagOptionButton
          disabled={disabled}
          label={`Create ${createTag}`}
          onSelect={() => onSelect(createTag)}
        />
      ) : null}
      {options.map((option) => (
        <TagOptionButton
          disabled={disabled}
          key={option.normalizedName}
          label={option.displayName}
          onSelect={() => onSelect(option.displayName)}
          usageCount={option.usageCount}
        />
      ))}
    </div>
  );
}

function TagOptionButton({
  label,
  usageCount,
  disabled,
  onSelect,
}: {
  label: string;
  usageCount?: number;
  disabled: boolean;
  onSelect: () => void;
}) {
  return (
    <button disabled={disabled} onClick={onSelect} type="button">
      <span>{label}</span>
      {usageCount ? <small>{usageCount}</small> : null}
    </button>
  );
}

function handleTagKeyDown(
  event: KeyboardEvent<HTMLInputElement>,
  query: string,
  selectedTags: string[],
  select: (tag: string) => void,
  onChange: (tags: string[]) => void,
  setOpen: (open: boolean) => void
) {
  if (event.key === "Enter" && query.trim()) {
    event.preventDefault();
    select(query);
  }
  if (event.key === "Backspace" && !query && selectedTags.length > 0) {
    event.preventDefault();
    onChange(selectedTags.slice(0, -1));
  }
  if (event.key === "Escape") {
    setOpen(false);
  }
}

function tagOptions(
  availableTags: TagCorpusEntry[],
  selectedTags: string[],
  query: string
): TagOption[] {
  const selected = new Set(selectedTags.map(normalizeTag));
  const search = query.trim().toLowerCase();
  return availableTags
    .filter((tag) => !selected.has(tag.normalized_name))
    .filter((tag) => tag.display_name.toLowerCase().includes(search))
    .sort(sortTagsByUsage)
    .map((tag) => ({
      displayName: tag.display_name,
      normalizedName: tag.normalized_name,
      usageCount: tag.usage_count,
    }));
}

function creatableTag(
  query: string,
  availableTags: TagCorpusEntry[],
  selectedTags: string[]
): string | null {
  const value = query.trim();
  const normalized = normalizeTag(value);
  const known = availableTags.some((tag) => tag.normalized_name === normalized);
  const selected = selectedTags.some((tag) => normalizeTag(tag) === normalized);
  return value && !known && !selected ? value : null;
}

function sortTagsByUsage(left: TagCorpusEntry, right: TagCorpusEntry) {
  return (
    right.usage_count - left.usage_count || left.display_name.localeCompare(right.display_name)
  );
}

function closeOnBlur(event: FocusEvent<HTMLDivElement>, setOpen: (open: boolean) => void) {
  const nextTarget = event.relatedTarget;
  if (!(nextTarget instanceof Node) || !event.currentTarget.contains(nextTarget)) {
    setOpen(false);
  }
}

function removeTag(tags: string[], tag: string) {
  const normalized = normalizeTag(tag);
  return tags.filter((item) => normalizeTag(item) !== normalized);
}

function dedupeTags(values: string[]) {
  const selected = new Map<string, string>();
  values.map((value) => value.trim()).forEach((value) => addTagValue(selected, value));
  return [...selected.values()];
}

function addTagValue(selected: Map<string, string>, value: string) {
  if (value.length > 0) {
    selected.set(normalizeTag(value), value);
  }
}

function normalizeTag(value: string) {
  return value.trim().toLowerCase();
}
