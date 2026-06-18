import type { ItemTag, TagCorpusEntry } from "./types";

type TagChoice = {
  display_name: string;
  normalized_name: string;
};

export function TagEditor({
  availableTags,
  selectedTags,
  disabled,
}: {
  availableTags: TagCorpusEntry[];
  selectedTags: ItemTag[];
  disabled: boolean;
}) {
  const selectedNames = new Set(selectedTags.map((tag) => tag.normalized_name));
  const choices = tagChoices(availableTags, selectedTags);
  return (
    <fieldset className="tag-editor">
      <legend>Tags</legend>
      <div className="tag-options">
        {choices.map((tag) => (
          <label className="tag-toggle" key={tag.normalized_name}>
            <input
              defaultChecked={selectedNames.has(tag.normalized_name)}
              disabled={disabled}
              name="tag"
              type="checkbox"
              value={tag.display_name}
            />
            <span>{tag.display_name}</span>
          </label>
        ))}
      </div>
      <label className="new-tag-field">
        New tag
        <input disabled={disabled} name="new_tag" type="text" />
      </label>
    </fieldset>
  );
}

export function selectedTagsFromForm(formData: FormData) {
  const checkedTags = formData.getAll("tag").map(formValue);
  const newTags = formValue(formData.get("new_tag")).split(",");
  return dedupeTags([...checkedTags, ...newTags]);
}

function tagChoices(availableTags: TagCorpusEntry[], selectedTags: ItemTag[]) {
  const choices = new Map<string, TagChoice>();
  selectedTags.forEach((tag) => choices.set(tag.normalized_name, tag));
  availableTags.forEach((tag) => choices.set(tag.normalized_name, tag));
  return [...choices.values()].sort((left, right) =>
    left.normalized_name.localeCompare(right.normalized_name)
  );
}

function dedupeTags(values: string[]) {
  const selected = new Map<string, string>();
  values.map((value) => value.trim()).forEach((value) => addTagValue(selected, value));
  return [...selected.values()];
}

function addTagValue(selected: Map<string, string>, value: string) {
  if (value.length > 0) {
    selected.set(value.toLowerCase(), value);
  }
}

function formValue(value: FormDataEntryValue | null) {
  return typeof value === "string" ? value : "";
}
