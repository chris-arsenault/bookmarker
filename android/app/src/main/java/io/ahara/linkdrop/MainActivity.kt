package io.ahara.linkdrop

import android.app.Activity
import android.os.Bundle
import android.text.InputType
import android.view.Gravity
import android.view.View
import android.view.ViewGroup
import android.widget.Button
import android.widget.EditText
import android.widget.LinearLayout
import android.widget.TextView
import io.ahara.linkdrop.auth.AuthFlowResult
import io.ahara.linkdrop.auth.CognitoAuthClient
import io.ahara.linkdrop.config.LinkdropConfig

class MainActivity : Activity() {
    private lateinit var authClient: CognitoAuthClient

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        authClient = CognitoAuthClient(this)
        renderHome()
    }

    private fun renderHome(message: String = "") {
        val statusText = if (authClient.hasFreshToken()) {
            getString(R.string.auth_signed_in)
        } else {
            getString(R.string.auth_signed_out)
        }
        setContentView(
            screen().apply {
                addView(titleView())
                addView(centeredText(statusText))
                if (message.isNotBlank()) {
                    addView(centeredText(message))
                }
                if (authClient.hasFreshToken()) {
                    addView(actionButton(R.string.auth_sign_out) {
                        authClient.signOut()
                        renderHome()
                    })
                } else {
                    addView(signInForm())
                }
            },
        )
    }

    private fun signInForm(): View {
        val username = EditText(this).apply {
            hint = getString(R.string.auth_username)
            setSingleLine(true)
            layoutParams = fullWidthLayoutParams(topMargin = 16)
        }
        val password = EditText(this).apply {
            hint = getString(R.string.auth_password)
            inputType = InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_VARIATION_PASSWORD
            setSingleLine(true)
            layoutParams = fullWidthLayoutParams(topMargin = 12)
        }
        return formLayout().apply {
            addView(username)
            addView(password)
            addView(actionButton(R.string.auth_sign_in) {
                runAuthAction { authClient.signIn(username.text.toString(), password.text.toString()) }
            })
        }
    }

    private fun renderMfaForm(setup: AuthFlowResult.MfaSetup? = null) {
        val code = EditText(this).apply {
            hint = getString(R.string.auth_code)
            inputType = InputType.TYPE_CLASS_NUMBER
            setSingleLine(true)
            layoutParams = fullWidthLayoutParams(topMargin = 16)
        }
        setContentView(
            screen().apply {
                addView(titleView())
                if (setup != null) {
                    addView(centeredText(getString(R.string.auth_setup_prompt)))
                    addView(centeredText(setup.secretCode))
                    addView(centeredText("${getString(R.string.auth_setup_uri)}\n${setup.otpAuthUri}"))
                }
                addView(code)
                val action = if (setup == null) {
                    { authClient.confirmMfa(code.text.toString()) }
                } else {
                    { authClient.verifyMfaSetup(code.text.toString()) }
                }
                addView(actionButton(R.string.auth_verify_code) { runAuthAction(action) })
            },
        )
    }

    private fun runAuthAction(action: () -> AuthFlowResult) {
        Thread {
            runCatching { action() }
                .onSuccess { result -> runOnUiThread { handleAuthResult(result) } }
                .onFailure { error ->
                    runOnUiThread {
                        renderHome(error.message ?: getString(R.string.auth_failed))
                    }
                }
        }.start()
    }

    private fun handleAuthResult(result: AuthFlowResult) {
        when (result) {
            AuthFlowResult.SignedIn -> renderHome()
            is AuthFlowResult.MfaRequired -> renderMfaForm()
            is AuthFlowResult.MfaSetup -> renderMfaForm(result)
        }
    }

    private fun screen(): LinearLayout =
        LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            gravity = Gravity.CENTER
            setPadding(48, 96, 48, 96)
        }

    private fun formLayout(): LinearLayout =
        LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            gravity = Gravity.CENTER
            layoutParams = fullWidthLayoutParams()
        }

    private fun titleView(): TextView =
        centeredText(LinkdropConfig.productName).apply { textSize = 24f }

    private fun actionButton(label: Int, onClick: () -> Unit): Button =
        Button(this).apply {
            text = getString(label)
            layoutParams = fullWidthLayoutParams(topMargin = 16)
            setOnClickListener { onClick() }
        }

    private fun centeredText(value: String): TextView =
        TextView(this).apply {
            text = value
            gravity = Gravity.CENTER
            layoutParams = fullWidthLayoutParams(topMargin = 8)
        }

    private fun fullWidthLayoutParams(topMargin: Int = 0): LinearLayout.LayoutParams =
        LinearLayout.LayoutParams(
            ViewGroup.LayoutParams.MATCH_PARENT,
            ViewGroup.LayoutParams.WRAP_CONTENT,
        ).apply {
            this.topMargin = topMargin
        }
}
