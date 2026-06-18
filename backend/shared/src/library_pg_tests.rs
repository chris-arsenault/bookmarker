use super::{ITEM_SELECT, TAG_CORPUS, UPDATE_ITEM_ORGANIZATION};

#[test]
fn postgres_item_queries_scope_by_authenticated_user() {
    assert!(ITEM_SELECT.contains("WHERE items.user_id = $1"));
    assert!(UPDATE_ITEM_ORGANIZATION.contains("WHERE id = $1 AND user_id = $2"));
}

#[test]
fn tag_corpus_query_orders_by_usage_then_name() {
    assert!(TAG_CORPUS.contains(
        "ORDER BY COALESCE(tag_usage_counts.usage_count, 0) DESC, tags.normalized_name ASC"
    ));
}
