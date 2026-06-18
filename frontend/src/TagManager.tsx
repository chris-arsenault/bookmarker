import type { MergeTagsRequest, RenameTagRequest, TagCorpusEntry } from "./types";

export function TagManager({
  tags,
  onRenameTag,
  onMergeTags,
}: {
  tags: TagCorpusEntry[];
  onRenameTag: (tagId: string, request: RenameTagRequest) => Promise<TagCorpusEntry[]>;
  onMergeTags: (sourceTagId: string, request: MergeTagsRequest) => Promise<TagCorpusEntry[]>;
}) {
  if (tags.length === 0) {
    return (
      <section className="tag-manager" aria-label="Tag management">
        <h2>Tag management</h2>
        <p>No tags yet</p>
      </section>
    );
  }
  return (
    <section className="tag-manager" aria-label="Tag management">
      <h2>Tag management</h2>
      <div className="tag-manager-list">
        {tags.map((tag) => (
          <TagManagerRow
            key={tag.id}
            onMergeTags={onMergeTags}
            onRenameTag={onRenameTag}
            tag={tag}
            tags={tags}
          />
        ))}
      </div>
    </section>
  );
}

function TagManagerRow({
  tag,
  tags,
  onRenameTag,
  onMergeTags,
}: {
  tag: TagCorpusEntry;
  tags: TagCorpusEntry[];
  onRenameTag: (tagId: string, request: RenameTagRequest) => Promise<TagCorpusEntry[]>;
  onMergeTags: (sourceTagId: string, request: MergeTagsRequest) => Promise<TagCorpusEntry[]>;
}) {
  const targets = tags.filter((target) => target.id !== tag.id);
  return (
    <div className="tag-manager-row">
      <div>
        <strong>{tag.display_name}</strong>
        <span>{tag.usage_count} saved</span>
      </div>
      <RenameTagForm onRenameTag={onRenameTag} tag={tag} />
      <MergeTagForm onMergeTags={onMergeTags} tag={tag} targets={targets} />
    </div>
  );
}

function RenameTagForm({
  tag,
  onRenameTag,
}: {
  tag: TagCorpusEntry;
  onRenameTag: (tagId: string, request: RenameTagRequest) => Promise<TagCorpusEntry[]>;
}) {
  const submit = (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const displayName = formValue(new FormData(event.currentTarget), "display_name");
    onRenameTag(tag.id, { display_name: displayName }).catch(() => {});
  };
  return (
    <form
      className="tag-action-form"
      data-tag-action="rename"
      data-tag-id={tag.id}
      onSubmit={submit}
    >
      <input defaultValue={tag.display_name} name="display_name" />
      <button className="secondary-action" type="submit">
        Rename
      </button>
    </form>
  );
}

function MergeTagForm({
  tag,
  targets,
  onMergeTags,
}: {
  tag: TagCorpusEntry;
  targets: TagCorpusEntry[];
  onMergeTags: (sourceTagId: string, request: MergeTagsRequest) => Promise<TagCorpusEntry[]>;
}) {
  const submit = (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const targetTagId = formValue(new FormData(event.currentTarget), "target_tag_id");
    if (targetTagId) {
      onMergeTags(tag.id, { target_tag_id: targetTagId }).catch(() => {});
    }
  };
  return (
    <form
      className="tag-action-form"
      data-tag-action="merge"
      data-tag-id={tag.id}
      onSubmit={submit}
    >
      <select disabled={targets.length === 0} name="target_tag_id">
        {targets.map((target) => (
          <option key={target.id} value={target.id}>
            {target.display_name}
          </option>
        ))}
      </select>
      <button className="secondary-action" disabled={targets.length === 0} type="submit">
        Merge
      </button>
    </form>
  );
}

function formValue(formData: FormData, name: string) {
  const value = formData.get(name);
  return typeof value === "string" ? value.trim() : "";
}
