package io.ahara.linkdrop.share

import android.app.Activity
import android.content.Intent
import android.net.Uri
import android.os.Bundle
import android.provider.OpenableColumns
import android.widget.Button
import android.widget.EditText
import android.widget.LinearLayout
import android.widget.TextView
import android.widget.Toast
import io.ahara.linkdrop.MainActivity
import io.ahara.linkdrop.R
import io.ahara.linkdrop.api.AuthRequiredException
import io.ahara.linkdrop.api.CaptureImageUploadAttempt
import io.ahara.linkdrop.api.CaptureAttempt
import io.ahara.linkdrop.api.CaptureTextAttempt
import io.ahara.linkdrop.api.LinkdropApiClient
import io.ahara.linkdrop.auth.CognitoAuthClient
import java.util.UUID

class ShareActivity : Activity() {
    private lateinit var apiClient: LinkdropApiClient
    private lateinit var sharedCaptures: List<SharedCapture>
    private lateinit var clientCaptureIds: List<String>
    private lateinit var tagState: ShareTagState
    private lateinit var tagChipRow: TagChipRow
    private lateinit var freeTextInput: EditText

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val parsedCaptures = ShareIntentParser.parse(intent)
        if (parsedCaptures.isEmpty()) {
            Toast.makeText(this, R.string.share_missing_payload, Toast.LENGTH_SHORT).show()
            finish()
            return
        }

        apiClient = LinkdropApiClient(CognitoAuthClient(this))
        tagState = ShareTagState()
        sharedCaptures = parsedCaptures
        clientCaptureIds = parsedCaptures.map { UUID.randomUUID().toString() }
        setContentView(contentView(sharePreview(parsedCaptures)))
    }

    private fun contentView(sharedUrl: String): LinearLayout {
        tagChipRow = TagChipRow(this)
        freeTextInput = EditText(this).apply {
            hint = "Tag"
            setSingleLine(true)
        }
        val dropButton = Button(this).apply {
            text = getString(R.string.share_drop)
            setOnClickListener { saveNow(this) }
        }
        val cancelButton = Button(this).apply {
            text = getString(R.string.share_cancel)
            setOnClickListener { finish() }
        }
        return LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(32, 32, 32, 32)
            addView(TextView(this@ShareActivity).apply { text = sharedUrl })
            addView(tagChipRow)
            addView(freeTextInput)
            addView(dropButton)
            addView(cancelButton)
            loadTagCorpus()
        }
    }

    private fun saveNow(dropButton: Button) {
        dropButton.isEnabled = false
        tagState.setFreeText(freeTextInput.text.toString())
        val tags = tagState.selectedTagValues()
        Thread {
            runCatching { saveCapture(tags) }
                .onSuccess {
                    toast(R.string.share_saved)
                    runOnUiThread { finish() }
                }
                .onFailure {
                    runOnUiThread {
                        if (it is AuthRequiredException) {
                            Toast.makeText(this, R.string.share_sign_in_required, Toast.LENGTH_SHORT).show()
                            startActivity(Intent(this, MainActivity::class.java))
                            finish()
                        } else {
                            Toast.makeText(this, R.string.share_failed, Toast.LENGTH_SHORT).show()
                            dropButton.isEnabled = true
                        }
                    }
                }
        }.start()
    }

    private fun saveCapture(tags: List<String>) {
        sharedCaptures.forEachIndexed { index, capture ->
            saveOneCapture(capture, clientCaptureIds[index], tags)
        }
    }

    private fun saveOneCapture(capture: SharedCapture, clientCaptureId: String, tags: List<String>) {
        when (capture) {
            is SharedCapture.Url -> apiClient.capture(
                CaptureAttempt(
                    url = capture.url,
                    title = capture.title,
                    tags = tags,
                    clientCaptureId = clientCaptureId,
                ),
            )
            is SharedCapture.Text -> apiClient.captureText(
                CaptureTextAttempt(
                    plainText = capture.plainText,
                    tags = tags,
                    clientCaptureId = clientCaptureId,
                ),
            )
            is SharedCapture.Image -> saveImageCapture(capture, clientCaptureId, tags)
        }
    }

    private fun saveImageCapture(
        capture: SharedCapture.Image,
        clientCaptureId: String,
        tags: List<String>,
    ) {
        val metadata = imageMetadata(capture)
        val upload = apiClient.createImageUpload(
            CaptureImageUploadAttempt(
                contentType = metadata.contentType,
                title = capture.title ?: metadata.displayName,
                originalFilename = metadata.displayName,
                byteSize = metadata.byteSize,
                tags = tags,
                clientCaptureId = clientCaptureId,
            ),
        )
        val input = contentResolver.openInputStream(capture.uri)
            ?: throw IllegalStateException("image stream unavailable")
        apiClient.uploadImage(upload.upload, input, metadata.byteSize)
        apiClient.completeImageUpload(upload.itemId)
    }

    private fun loadTagCorpus() {
        Thread {
            runCatching { apiClient.listTags() }
                .onSuccess { tags ->
                    runOnUiThread {
                        tagState.setCorpus(tags)
                        tagChipRow.render(
                            tags = tagState.availableTags,
                            selected = tagState.selectedNormalizedNames,
                            onToggle = { tag ->
                                tagState.toggle(tag)
                                tagChipRow.render(
                                    tags = tagState.availableTags,
                                    selected = tagState.selectedNormalizedNames,
                                    onToggle = tagState::toggle,
                                )
                            },
                        )
                    }
                }
        }.start()
    }

    private fun toast(message: Int) {
        runOnUiThread {
            Toast.makeText(this, message, Toast.LENGTH_SHORT).show()
        }
    }

    private fun imageMetadata(capture: SharedCapture.Image): ImageMetadata {
        val query = contentResolver.query(capture.uri, null, null, null, null)
        query.use { cursor ->
            val displayName = cursor?.value(OpenableColumns.DISPLAY_NAME)
            val byteSize = cursor?.longValue(OpenableColumns.SIZE)?.takeIf { it > 0 }
            return ImageMetadata(
                contentType = resolvedImageContentType(capture.uri, capture.contentType),
                displayName = displayName,
                byteSize = byteSize,
            )
        }
    }

    private fun resolvedImageContentType(uri: Uri, fallback: String): String {
        val resolved = contentResolver.getType(uri)
            ?.takeIf { it.startsWith("image/") && it != "image/*" }
        val provided = fallback.takeIf { it.startsWith("image/") && it != "image/*" }
        return resolved ?: provided ?: "image/jpeg"
    }
}

private data class ImageMetadata(
    val contentType: String,
    val displayName: String?,
    val byteSize: Long?,
)

private fun sharePreview(captures: List<SharedCapture>): String =
    captures.joinToString(separator = "\n\n") { capture -> capture.preview }

private fun android.database.Cursor.value(columnName: String): String? {
    if (!moveToFirst()) {
        return null
    }
    val index = getColumnIndex(columnName)
    return index.takeIf { it >= 0 }?.let(::getString)?.takeIf(String::isNotBlank)
}

private fun android.database.Cursor.longValue(columnName: String): Long? {
    if (!moveToFirst()) {
        return null
    }
    val index = getColumnIndex(columnName)
    return index.takeIf { it >= 0 }?.let(::getLong)
}
