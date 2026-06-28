package io.ahara.linkdrop.share

import android.content.Context
import android.view.Gravity
import android.widget.Button
import android.widget.LinearLayout
import io.ahara.linkdrop.api.TagCorpusEntry

class TagChipRow(context: Context) : LinearLayout(context) {
    init {
        orientation = HORIZONTAL
        gravity = Gravity.CENTER
    }

    fun render(
        tags: List<TagCorpusEntry>,
        selected: Set<String>,
        onToggle: (TagCorpusEntry) -> Unit,
    ) {
        removeAllViews()
        tags.forEach { tag ->
            addView(
                Button(context).apply {
                    text = tag.displayName
                    isSelected = selected.contains(tag.normalizedName)
                    setOnClickListener {
                        onToggle(tag)
                        render(tags, selected.toggle(tag.normalizedName), onToggle)
                    }
                },
            )
        }
    }
}

private fun Set<String>.toggle(value: String): Set<String> =
    if (contains(value)) this - value else this + value
