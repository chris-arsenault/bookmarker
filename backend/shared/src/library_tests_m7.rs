use super::*;

#[tokio::test]
async fn in_memory_list_filters_items_by_inbox_status() {
    let matching_id = Uuid::parse_str("00000000-0000-0000-0000-000000000701").unwrap();
    let hidden_id = Uuid::parse_str("00000000-0000-0000-0000-000000000702").unwrap();
    let service = InMemoryLibraryService::with_user_items(
        "user-sub",
        [
            LibraryItemDetail {
                summary: LibraryItemSummary {
                    inbox_status: InboxStatus::Unsorted,
                    ..item(matching_id).summary
                },
                notes: String::new(),
            },
            LibraryItemDetail {
                summary: LibraryItemSummary {
                    inbox_status: InboxStatus::Organized,
                    ..item(hidden_id).summary
                },
                notes: String::new(),
            },
        ],
    );

    let items = service
        .list_items(
            &user(),
            &ListItemsQuery {
                inbox_status: Some(InboxStatus::Unsorted),
                ..ListItemsQuery::default()
            },
        )
        .await
        .unwrap();

    assert_eq!(
        items.into_iter().map(|item| item.id).collect::<Vec<_>>(),
        vec![matching_id]
    );
}

#[tokio::test]
async fn in_memory_update_item_replaces_tags_and_updates_corpus() {
    let item_one = Uuid::parse_str("00000000-0000-0000-0000-000000000801").unwrap();
    let item_two = Uuid::parse_str("00000000-0000-0000-0000-000000000802").unwrap();
    let service = InMemoryLibraryService::with_user_items(
        "user-sub",
        [
            item_with_tags(item_one, ["Learning"]),
            item_with_tags(item_two, ["Learning"]),
        ],
    );
    service.set_user_tags(
        "user-sub",
        [TagCorpusEntry {
            id: Uuid::parse_str("00000000-0000-0000-0000-000000000803").unwrap(),
            display_name: "Learning".to_string(),
            normalized_name: "learning".to_string(),
            usage_count: 2,
        }],
    );

    let updated = service
        .update_item(
            &user(),
            item_one,
            UpdateItemRequest {
                tags: Some(vec![" Videos ".to_string(), "videos".to_string()]),
                ..UpdateItemRequest::default()
            },
        )
        .await
        .unwrap();

    assert_eq!(updated.summary.tags.len(), 1);
    assert_eq!(updated.summary.tags[0].display_name, "Videos");
    let corpus = service.list_tag_corpus(&user()).await.unwrap();
    assert_eq!(
        corpus
            .iter()
            .map(|tag| (tag.normalized_name.as_str(), tag.usage_count))
            .collect::<Vec<_>>(),
        vec![("learning", 1), ("videos", 1)]
    );
}

#[tokio::test]
async fn in_memory_update_item_edits_tags_notes_watch_and_inbox() {
    let item_id = Uuid::parse_str("00000000-0000-0000-0000-000000000811").unwrap();
    let service = InMemoryLibraryService::with_user_items(
        "user-sub",
        [item_with_tags(item_id, ["Learning"])],
    );

    let updated = service
        .update_item(
            &user(),
            item_id,
            UpdateItemRequest {
                watch_status: Some(WatchStatus::Watched),
                inbox_status: Some(InboxStatus::Organized),
                notes: Some("Filed for the API design pass".to_string()),
                tags: None,
            },
        )
        .await
        .unwrap();

    assert_eq!(updated.summary.watch_status, WatchStatus::Watched);
    assert_eq!(updated.summary.inbox_status, InboxStatus::Organized);
    assert_eq!(updated.notes, "Filed for the API design pass");
    assert_eq!(updated.summary.tags[0].normalized_name, "learning");
}

#[tokio::test]
async fn in_memory_rename_tag_rejects_collision() {
    let learning_id = Uuid::parse_str("00000000-0000-0000-0000-000000000901").unwrap();
    let videos_id = Uuid::parse_str("00000000-0000-0000-0000-000000000902").unwrap();
    let service = InMemoryLibraryService::new();
    service.set_user_tags(
        "user-sub",
        [
            corpus_tag(learning_id, "Learning", 1),
            corpus_tag(videos_id, "Videos", 1),
        ],
    );

    let result = service
        .rename_tag(
            &user(),
            videos_id,
            RenameTagRequest {
                display_name: " learning ".to_string(),
            },
        )
        .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn in_memory_merge_tags_moves_associations_and_reranks_corpus() {
    let learning_id = Uuid::parse_str("00000000-0000-0000-0000-000000000911").unwrap();
    let videos_id = Uuid::parse_str("00000000-0000-0000-0000-000000000912").unwrap();
    let first_item = Uuid::parse_str("00000000-0000-0000-0000-000000000913").unwrap();
    let second_item = Uuid::parse_str("00000000-0000-0000-0000-000000000914").unwrap();
    let service = InMemoryLibraryService::with_user_items(
        "user-sub",
        [
            item_with_item_tags(
                first_item,
                [
                    item_tag_with_id(learning_id, "Learning"),
                    item_tag_with_id(videos_id, "Videos"),
                ],
            ),
            item_with_item_tags(second_item, [item_tag_with_id(videos_id, "Videos")]),
        ],
    );
    service.set_user_tags(
        "user-sub",
        [
            corpus_tag(learning_id, "Learning", 1),
            corpus_tag(videos_id, "Videos", 2),
        ],
    );

    let corpus = service
        .merge_tags(
            &user(),
            videos_id,
            MergeTagsRequest {
                target_tag_id: learning_id,
            },
        )
        .await
        .unwrap();

    assert_eq!(corpus.len(), 1);
    assert_eq!(corpus[0].normalized_name, "learning");
    assert_eq!(corpus[0].usage_count, 2);
    assert_eq!(
        service
            .get_item(&user(), second_item)
            .await
            .unwrap()
            .summary
            .tags[0]
            .id,
        learning_id
    );
}

fn item_with_tags<const N: usize>(id: Uuid, tags: [&str; N]) -> LibraryItemDetail {
    LibraryItemDetail {
        summary: LibraryItemSummary {
            tags: tags.into_iter().map(test_tag).collect(),
            ..item(id).summary
        },
        notes: String::new(),
    }
}

fn item_with_item_tags<const N: usize>(id: Uuid, tags: [ItemTag; N]) -> LibraryItemDetail {
    LibraryItemDetail {
        summary: LibraryItemSummary {
            tags: tags.into_iter().collect(),
            ..item(id).summary
        },
        notes: String::new(),
    }
}

fn item_tag_with_id(id: Uuid, name: &str) -> ItemTag {
    ItemTag {
        id,
        display_name: name.to_string(),
        normalized_name: name.to_ascii_lowercase(),
    }
}

fn corpus_tag(id: Uuid, name: &str, usage_count: i32) -> TagCorpusEntry {
    TagCorpusEntry {
        id,
        display_name: name.to_string(),
        normalized_name: name.to_ascii_lowercase(),
        usage_count,
    }
}
