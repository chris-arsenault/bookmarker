package io.ahara.linkdrop.api

import org.json.JSONArray
import org.json.JSONObject
import java.util.UUID

data class CaptureAttempt(
    val url: String,
    val title: String? = null,
    val tags: List<String> = emptyList(),
    val clientCaptureId: String = UUID.randomUUID().toString(),
) {
    fun toJson(): String =
        JSONObject()
            .put("url", url)
            .put("title", title ?: JSONObject.NULL)
            .put("tags", JSONArray(tags))
            .put("client_capture_id", clientCaptureId)
            .toString()
}

data class CaptureTextAttempt(
    val plainText: String,
    val title: String? = null,
    val tags: List<String> = emptyList(),
    val clientCaptureId: String = UUID.randomUUID().toString(),
) {
    fun toJson(): String =
        JSONObject()
            .put("plain_text", plainText)
            .put("title", title ?: JSONObject.NULL)
            .put("html", JSONObject.NULL)
            .put("source_app", "Android share")
            .put("source_device", "android")
            .put("capture_method", "android_share")
            .put("tags", JSONArray(tags))
            .put("client_capture_id", clientCaptureId)
            .toString()
}

data class CaptureResult(
    val rawJson: String,
    val created: Boolean,
)

data class TagCorpusEntry(
    val displayName: String,
    val normalizedName: String,
    val usageCount: Int,
)

class AuthRequiredException : IllegalStateException("A fresh bearer token is required")
