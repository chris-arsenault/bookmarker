package io.ahara.linkdrop.share

import android.content.Intent

object ShareIntentParser {
    private val urlPattern = Regex("""https?://\S+""")

    fun parse(intent: Intent): SharedCapture? {
        if (intent.action != Intent.ACTION_SEND || !intent.type.orEmpty().startsWith("text/")) {
            return null
        }
        val text = intent.getStringExtra(Intent.EXTRA_TEXT).orEmpty().trim()
        val url = urlPattern.find(text)?.value?.trimEnd('.', ',', ')')
        if (url != null) {
            return SharedCapture.Url(url)
        }
        return text.takeIf { it.isNotBlank() }?.let(SharedCapture::Text)
    }

    fun parseUrl(intent: Intent): String? {
        return (parse(intent) as? SharedCapture.Url)?.url
    }
}

sealed class SharedCapture {
    abstract val preview: String

    data class Url(val url: String) : SharedCapture() {
        override val preview = url
    }

    data class Text(val plainText: String) : SharedCapture() {
        override val preview = plainText
    }
}
