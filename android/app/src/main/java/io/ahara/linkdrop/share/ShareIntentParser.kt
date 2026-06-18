package io.ahara.linkdrop.share

import android.content.Intent

object ShareIntentParser {
    private val urlPattern = Regex("""https?://\S+""")

    fun parse(intent: Intent): SharedCapture? {
        if (intent.action != Intent.ACTION_SEND || !intent.type.orEmpty().startsWith("text/")) {
            return null
        }
        val text = intent.getStringExtra(Intent.EXTRA_TEXT).orEmpty().trim()
        val urlMatch = urlPattern.find(text)
        if (urlMatch != null) {
            return SharedCapture.Url(
                url = cleanUrl(urlMatch.value),
                title = sharedTitle(intent, text, urlMatch),
            )
        }
        return text.takeIf { it.isNotBlank() }?.let(SharedCapture::Text)
    }

    fun parseUrl(intent: Intent): String? {
        return (parse(intent) as? SharedCapture.Url)?.url
    }

    private fun sharedTitle(intent: Intent, text: String, urlMatch: MatchResult): String? {
        val candidates = listOf(
            intent.getStringExtra(Intent.EXTRA_TITLE),
            intent.getStringExtra(Intent.EXTRA_SUBJECT),
            text.removeRange(urlMatch.range),
        )
        return candidates.firstNotNullOfOrNull(::cleanTitle)
    }

    private fun cleanUrl(value: String): String =
        value.trim().trimEnd('.', ',', ')')

    private fun cleanTitle(value: String?): String? {
        val trimmed = value
            ?.trim()
            ?.trim('-', ':', '|', '(', ')', '[', ']', '.', ',')
            ?.trim()
            .orEmpty()
        return trimmed.takeIf { it.isNotBlank() && urlPattern.find(it) == null }
    }
}

sealed class SharedCapture {
    abstract val preview: String

    data class Url(val url: String, val title: String?) : SharedCapture() {
        override val preview = listOfNotNull(title, url).joinToString("\n")
    }

    data class Text(val plainText: String) : SharedCapture() {
        override val preview = plainText
    }
}
