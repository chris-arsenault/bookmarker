package io.ahara.linkdrop.share

import android.content.Intent
import android.net.Uri

object ShareIntentParser {
    private val urlPattern = Regex("""https?://\S+""")

    fun parse(intent: Intent): List<SharedCapture> {
        val type = intent.type.orEmpty()
        return when {
            intent.action == Intent.ACTION_SEND && type.startsWith("text/") ->
                parseText(intent)?.let(::listOf).orEmpty()
            intent.action == Intent.ACTION_SEND && type.startsWith("image/") ->
                parseSingleImage(intent, type)?.let(::listOf).orEmpty()
            intent.action == Intent.ACTION_SEND_MULTIPLE && type.startsWith("image/") ->
                parseMultipleImages(intent, type)
            else -> emptyList()
        }
    }

    private fun parseText(intent: Intent): SharedCapture? {
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
        return (parse(intent).firstOrNull() as? SharedCapture.Url)?.url
    }

    @Suppress("DEPRECATION")
    private fun parseSingleImage(intent: Intent, type: String): SharedCapture.Image? {
        val uri = intent.getParcelableExtra<Uri>(Intent.EXTRA_STREAM) ?: return null
        return SharedCapture.Image(uri = uri, contentType = type, title = imageTitle(intent))
    }

    @Suppress("DEPRECATION")
    private fun parseMultipleImages(intent: Intent, type: String): List<SharedCapture.Image> {
        val uris = intent.getParcelableArrayListExtra<Uri>(Intent.EXTRA_STREAM).orEmpty()
        return uris.map { uri ->
            SharedCapture.Image(uri = uri, contentType = type, title = imageTitle(intent))
        }
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

    private fun imageTitle(intent: Intent): String? =
        cleanTitle(intent.getStringExtra(Intent.EXTRA_TITLE))
            ?: cleanTitle(intent.getStringExtra(Intent.EXTRA_SUBJECT))
}

sealed class SharedCapture {
    abstract val preview: String

    data class Url(val url: String, val title: String?) : SharedCapture() {
        override val preview = listOfNotNull(title, url).joinToString("\n")
    }

    data class Text(val plainText: String) : SharedCapture() {
        override val preview = plainText
    }

    data class Image(
        val uri: Uri,
        val contentType: String,
        val title: String?,
    ) : SharedCapture() {
        override val preview = listOfNotNull(title, uri.lastPathSegment).joinToString("\n")
            .ifBlank { uri.toString() }
    }
}
