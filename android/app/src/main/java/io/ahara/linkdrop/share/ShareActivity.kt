package io.ahara.linkdrop.share

import android.app.Activity
import android.content.Intent
import android.os.Bundle
import android.widget.Button
import android.widget.EditText
import android.widget.LinearLayout
import android.widget.TextView
import android.widget.Toast
import io.ahara.linkdrop.MainActivity
import io.ahara.linkdrop.R
import io.ahara.linkdrop.api.AuthRequiredException
import io.ahara.linkdrop.api.CaptureAttempt
import io.ahara.linkdrop.api.CaptureTextAttempt
import io.ahara.linkdrop.api.LinkdropApiClient
import io.ahara.linkdrop.auth.CognitoAuthClient
import java.util.UUID

class ShareActivity : Activity() {
    private lateinit var apiClient: LinkdropApiClient
    private lateinit var sharedCapture: SharedCapture
    private lateinit var clientCaptureId: String
    private lateinit var tagState: ShareTagState
    private lateinit var tagChipRow: TagChipRow
    private lateinit var freeTextInput: EditText

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val parsedCapture = ShareIntentParser.parse(intent)
        if (parsedCapture == null) {
            Toast.makeText(this, R.string.share_missing_payload, Toast.LENGTH_SHORT).show()
            finish()
            return
        }

        apiClient = LinkdropApiClient(CognitoAuthClient(this))
        tagState = ShareTagState()
        sharedCapture = parsedCapture
        clientCaptureId = UUID.randomUUID().toString()
        setContentView(contentView(parsedCapture.preview))
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
        when (val capture = sharedCapture) {
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
        }
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
}
