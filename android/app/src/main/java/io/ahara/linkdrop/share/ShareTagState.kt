package io.ahara.linkdrop.share

import io.ahara.linkdrop.api.TagCorpusEntry

class ShareTagState {
    var availableTags: List<TagCorpusEntry> = emptyList()
        private set
    val selectedNormalizedNames: Set<String>
        get() = selectedNames.toSet()

    private val selectedNames = linkedSetOf<String>()
    private var freeTextTag: String = ""

    fun setCorpus(tags: List<TagCorpusEntry>) {
        availableTags = tags
        selectedNames.removeAll { selected ->
            tags.none { tag -> tag.normalizedName == selected }
        }
    }

    fun toggle(tag: TagCorpusEntry) {
        if (!selectedNames.add(tag.normalizedName)) {
            selectedNames.remove(tag.normalizedName)
        }
    }

    fun setFreeText(value: String) {
        freeTextTag = value.trim()
    }

    fun selectedTagValues(): List<String> {
        val chipTags = availableTags
            .filter { selectedNames.contains(it.normalizedName) }
            .map { it.displayName }
        val freeText = freeTextTag.takeIf(String::isNotBlank)?.let(::listOf).orEmpty()
        return (chipTags + freeText).distinctBy { it.trim().lowercase() }
    }
}
