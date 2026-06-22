package io.ahara.linkdrop.api

import io.ahara.linkdrop.auth.AuthRepository
import io.ahara.linkdrop.config.LinkdropConfig
import org.json.JSONArray
import org.json.JSONObject
import java.io.InputStream
import java.net.HttpURLConnection
import java.net.URL

class LinkdropApiClient(
    private val authRepository: AuthRepository,
    private val apiBaseUrl: String = LinkdropConfig.apiBaseUrl,
) {
    fun listTags(): List<TagCorpusEntry> {
        val response = request("GET", "/tags")
        val payload = JSONArray(response.body)
        return (0 until payload.length()).map { index ->
            val item = payload.getJSONObject(index)
            TagCorpusEntry(
                displayName = item.getString("display_name"),
                normalizedName = item.getString("normalized_name"),
                usageCount = item.getInt("usage_count"),
            )
        }
    }

    fun capture(attempt: CaptureAttempt): CaptureResult {
        val response = request("POST", "/items", attempt.toJson())
        return CaptureResult(
            rawJson = response.body,
            created = response.statusCode == HttpURLConnection.HTTP_CREATED,
        )
    }

    fun captureText(attempt: CaptureTextAttempt): CaptureResult {
        val response = request("POST", "/items/text", attempt.toJson())
        return CaptureResult(
            rawJson = response.body,
            created = response.statusCode == HttpURLConnection.HTTP_CREATED,
        )
    }

    fun createImageUpload(attempt: CaptureImageUploadAttempt): ImageUploadResult {
        val response = request("POST", "/items/images/uploads", attempt.toJson())
        val payload = JSONObject(response.body)
        return ImageUploadResult(
            itemId = payload.getJSONObject("item").getJSONObject("summary").getString("id"),
            upload = uploadTarget(payload.getJSONObject("upload")),
            created = response.statusCode == HttpURLConnection.HTTP_CREATED,
        )
    }

    fun uploadImage(
        target: ImageUploadTarget,
        input: InputStream,
        byteSize: Long?,
    ) {
        val connection = (URL(target.url).openConnection() as HttpURLConnection).apply {
            requestMethod = "PUT"
            doOutput = true
            target.headers.forEach { (name, value) -> setRequestProperty(name, value) }
            byteSize?.takeIf { it > 0 }?.let { size -> setFixedLengthStreamingMode(size) }
        }
        input.use { source ->
            connection.outputStream.use { output -> source.copyTo(output) }
        }
        val status = connection.responseCode
        if (status !in 200..299) {
            throw LinkdropApiException(status, connection.errorStreamText())
        }
    }

    fun completeImageUpload(itemId: String): CaptureResult {
        val response = request("POST", "/items/$itemId/image-upload/complete")
        return CaptureResult(rawJson = response.body, created = false)
    }

    private fun request(
        method: String,
        path: String,
        body: String? = null,
    ): ApiResponse {
        val token = authRepository.freshBearerToken() ?: throw AuthRequiredException()
        val connection = (URL("$apiBaseUrl$path").openConnection() as HttpURLConnection).apply {
            requestMethod = method
            setRequestProperty("Authorization", "Bearer $token")
            setRequestProperty("Accept", "application/json")
            if (body != null) {
                doOutput = true
                setRequestProperty("Content-Type", "application/json")
                outputStream.use { stream ->
                    stream.write(body.toByteArray(Charsets.UTF_8))
                }
            }
        }

        val status = connection.responseCode
        val stream = if (status in 200..299) connection.inputStream else connection.errorStream
        val responseBody = stream?.bufferedReader()?.use { it.readText() }.orEmpty()
        if (status !in 200..299) {
            throw LinkdropApiException(status, responseBody)
        }
        return ApiResponse(statusCode = status, body = responseBody)
    }
}

private fun uploadTarget(payload: JSONObject): ImageUploadTarget {
    val headersPayload = payload.getJSONObject("headers")
    val headers = headersPayload.keys().asSequence().associateWith { key ->
        headersPayload.getString(key)
    }
    return ImageUploadTarget(url = payload.getString("url"), headers = headers)
}

private fun HttpURLConnection.errorStreamText(): String =
    errorStream?.bufferedReader()?.use { it.readText() }.orEmpty()

data class LinkdropApiException(
    val statusCode: Int,
    val errorBody: String,
) : RuntimeException("Linkdrop API request failed with status $statusCode")

private data class ApiResponse(
    val statusCode: Int,
    val body: String,
)
