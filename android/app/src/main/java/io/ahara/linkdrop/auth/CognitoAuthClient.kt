package io.ahara.linkdrop.auth

import android.content.Context
import io.ahara.linkdrop.config.LinkdropConfig
import org.json.JSONObject
import java.net.HttpURLConnection
import java.net.URL

sealed class AuthFlowResult {
    data object SignedIn : AuthFlowResult()
    data class MfaRequired(val username: String) : AuthFlowResult()
    data class MfaSetup(
        val username: String,
        val secretCode: String,
        val otpAuthUri: String,
    ) : AuthFlowResult()
}

class CognitoAuthClient(
    context: Context,
    private val tokenStore: AuthTokenStore = AuthTokenStore(context.applicationContext),
    private val config: CognitoAuthConfig = CognitoAuthConfig.fromLinkdropConfig(),
) : AuthRepository {
    private var pendingChallenge: PendingChallenge? = null

    override fun freshBearerToken(): String? {
        val tokens = tokenStore.read() ?: return null
        if (tokens.isFresh()) {
            return tokens.accessToken
        }
        val refreshToken = tokens.refreshToken?.takeIf(String::isNotBlank) ?: return null
        return runCatching { refreshAccessToken(refreshToken) }.getOrNull()
    }

    override fun hasFreshToken(): Boolean = tokenStore.read()?.isFresh() == true

    fun signIn(username: String, password: String): AuthFlowResult {
        val body = JSONObject()
            .put("AuthFlow", "USER_PASSWORD_AUTH")
            .put("ClientId", config.clientId)
            .put(
                "AuthParameters",
                JSONObject()
                    .put("USERNAME", username)
                    .put("PASSWORD", password),
            )
        return handleAuthResponse(post("InitiateAuth", body), username)
    }

    fun confirmMfa(code: String): AuthFlowResult {
        val pending = requirePending("SOFTWARE_TOKEN_MFA")
        val body = challengeBody("SOFTWARE_TOKEN_MFA", pending)
            .put(
                "ChallengeResponses",
                JSONObject()
                    .put("USERNAME", pending.username)
                    .put("SOFTWARE_TOKEN_MFA_CODE", code),
            )
        return handleAuthResponse(post("RespondToAuthChallenge", body), pending.username)
    }

    fun verifyMfaSetup(code: String): AuthFlowResult {
        val pending = requirePending("MFA_SETUP")
        val verified = post(
            "VerifySoftwareToken",
            JSONObject()
                .put("Session", pending.session)
                .put("UserCode", code)
                .put("FriendlyDeviceName", config.productName),
        )
        val verifiedSession = requiredString(verified, "Session")
        val body = challengeBody("MFA_SETUP", pending.copy(session = verifiedSession))
            .put("ChallengeResponses", JSONObject().put("USERNAME", pending.username))
        return handleAuthResponse(post("RespondToAuthChallenge", body), pending.username)
    }

    fun signOut() {
        pendingChallenge = null
        tokenStore.clear()
    }

    private fun refreshAccessToken(refreshToken: String): String {
        val response = post(
            "InitiateAuth",
            JSONObject()
                .put("AuthFlow", "REFRESH_TOKEN_AUTH")
                .put("ClientId", config.clientId)
                .put("AuthParameters", JSONObject().put("REFRESH_TOKEN", refreshToken)),
        )
        val auth = response.getJSONObject("AuthenticationResult")
        saveAuthTokens(auth, refreshToken)
        return auth.getString("AccessToken")
    }

    private fun handleAuthResponse(response: JSONObject, username: String): AuthFlowResult {
        response.optJSONObject("AuthenticationResult")?.let { auth ->
            saveAuthTokens(auth, null)
            pendingChallenge = null
            return AuthFlowResult.SignedIn
        }

        val challenge = requiredString(response, "ChallengeName")
        val session = requiredString(response, "Session")
        val challengeUsername = challengeUsername(response, username)
        return when (challenge) {
            "SOFTWARE_TOKEN_MFA" -> {
                pendingChallenge = PendingChallenge(challengeUsername, session, challenge)
                AuthFlowResult.MfaRequired(challengeUsername)
            }
            "MFA_SETUP" -> beginMfaSetup(challengeUsername, session)
            "SELECT_MFA_TYPE" -> selectSoftwareTokenMfa(challengeUsername, session)
            "NEW_PASSWORD_REQUIRED" -> throw CognitoAuthException("new password required")
            else -> throw CognitoAuthException("unsupported Cognito challenge: $challenge")
        }
    }

    private fun beginMfaSetup(username: String, session: String): AuthFlowResult {
        val associated = post("AssociateSoftwareToken", JSONObject().put("Session", session))
        val secret = requiredString(associated, "SecretCode")
        val nextSession = requiredString(associated, "Session")
        pendingChallenge = PendingChallenge(username, nextSession, "MFA_SETUP")
        return AuthFlowResult.MfaSetup(
            username = username,
            secretCode = secret,
            otpAuthUri = totpUri(username, secret),
        )
    }

    private fun selectSoftwareTokenMfa(username: String, session: String): AuthFlowResult {
        val pending = PendingChallenge(username, session, "SELECT_MFA_TYPE")
        val body = challengeBody("SELECT_MFA_TYPE", pending)
            .put(
                "ChallengeResponses",
                JSONObject()
                    .put("USERNAME", username)
                    .put("ANSWER", "SOFTWARE_TOKEN_MFA"),
            )
        return handleAuthResponse(post("RespondToAuthChallenge", body), username)
    }

    private fun saveAuthTokens(auth: JSONObject, fallbackRefreshToken: String?) {
        val refreshToken = auth.optString("RefreshToken").takeIf(String::isNotBlank)
            ?: fallbackRefreshToken
        tokenStore.save(
            AuthTokens(
                accessToken = auth.getString("AccessToken"),
                refreshToken = refreshToken,
                expiresAtEpochSeconds = nowEpochSeconds() + auth.optLong("ExpiresIn", 3600),
            ),
        )
    }

    private fun challengeBody(challengeName: String, pending: PendingChallenge): JSONObject =
        JSONObject()
            .put("ChallengeName", challengeName)
            .put("ClientId", config.clientId)
            .put("Session", pending.session)

    private fun requirePending(challengeName: String): PendingChallenge {
        val pending = pendingChallenge
            ?: throw CognitoAuthException("missing authentication challenge")
        if (pending.challengeName != challengeName) {
            throw CognitoAuthException("expected ${pending.challengeName}, not $challengeName")
        }
        return pending
    }

    private fun post(operation: String, body: JSONObject): JSONObject {
        val connection = (URL(config.idpEndpoint).openConnection() as HttpURLConnection).apply {
            requestMethod = "POST"
            doOutput = true
            setRequestProperty("Content-Type", "application/x-amz-json-1.1")
            setRequestProperty("X-Amz-Target", "AWSCognitoIdentityProviderService.$operation")
            outputStream.use { stream ->
                stream.write(body.toString().toByteArray(Charsets.UTF_8))
            }
        }
        val status = connection.responseCode
        val stream = if (status in 200..299) connection.inputStream else connection.errorStream
        val responseBody = stream?.bufferedReader()?.use { it.readText() }.orEmpty()
        if (status !in 200..299) {
            throw CognitoAuthException(cognitoErrorMessage(responseBody, status))
        }
        return JSONObject(responseBody)
    }

    private fun totpUri(username: String, secretCode: String): String {
        val issuer = encode(config.productName)
        val account = encode("${config.productName}:$username")
        return "otpauth://totp/$account?secret=${encode(secretCode)}&issuer=$issuer"
    }
}

data class CognitoAuthConfig(
    val productName: String,
    val clientId: String,
    val idpEndpoint: String,
) {
    companion object {
        fun fromLinkdropConfig(): CognitoAuthConfig =
            CognitoAuthConfig(
                productName = LinkdropConfig.productName,
                clientId = LinkdropConfig.cognitoClientId,
                idpEndpoint = idpEndpointFromIssuer(LinkdropConfig.cognitoIssuer),
            )
    }
}

data class CognitoAuthException(
    override val message: String,
) : RuntimeException(message)

private data class PendingChallenge(
    val username: String,
    val session: String,
    val challengeName: String,
)

private fun challengeUsername(response: JSONObject, fallback: String): String =
    response
        .optJSONObject("ChallengeParameters")
        ?.optString("USERNAME")
        ?.takeIf(String::isNotBlank)
        ?: fallback

private fun requiredString(payload: JSONObject, key: String): String =
    payload.optString(key).takeIf(String::isNotBlank)
        ?: throw CognitoAuthException("missing Cognito response field: $key")

private fun cognitoErrorMessage(responseBody: String, status: Int): String {
    val parsed = runCatching { JSONObject(responseBody) }.getOrNull()
    return parsed?.optString("message")?.takeIf(String::isNotBlank)
        ?: parsed?.optString("__type")?.substringAfterLast('#')?.takeIf(String::isNotBlank)
        ?: "Cognito request failed with status $status"
}

private fun idpEndpointFromIssuer(issuer: String): String {
    val url = URL(issuer)
    return "${url.protocol}://${url.host}/"
}

private fun encode(value: String): String =
    java.net.URLEncoder.encode(value, Charsets.UTF_8.name()).replace("+", "%20")

private fun nowEpochSeconds(): Long = System.currentTimeMillis() / 1000
