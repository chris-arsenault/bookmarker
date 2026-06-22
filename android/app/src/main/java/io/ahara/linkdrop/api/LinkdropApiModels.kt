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

data class CaptureImageUploadAttempt(
    val contentType: String,
    val title: String? = null,
    val originalFilename: String? = null,
    val byteSize: Long? = null,
    val sourceApp: String? = "Android share",
    val sourceDevice: String? = "android",
    val captureMethod: String? = "android_share",
    val tags: List<String> = emptyList(),
    val clientCaptureId: String = UUID.randomUUID().toString(),
) {
    fun toJson(): String =
        JSONObject()
            .put("content_type", contentType)
            .put("title", title ?: JSONObject.NULL)
            .put("original_filename", originalFilename ?: JSONObject.NULL)
            .put("byte_size", byteSize ?: JSONObject.NULL)
            .put("source_app", sourceApp ?: JSONObject.NULL)
            .put("source_device", sourceDevice ?: JSONObject.NULL)
            .put("capture_method", captureMethod ?: JSONObject.NULL)
            .put("tags", JSONArray(tags))
            .put("client_capture_id", clientCaptureId)
            .toString()
}

data class ImageUploadResult(
    val itemId: String,
    val upload: ImageUploadTarget,
    val created: Boolean,
)

data class ImageUploadTarget(
    val url: String,
    val headers: Map<String, String>,
)

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
