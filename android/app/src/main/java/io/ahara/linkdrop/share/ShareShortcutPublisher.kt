package io.ahara.linkdrop.share

import android.content.Context
import android.content.Intent
import android.content.pm.ShortcutInfo
import android.content.pm.ShortcutManager
import android.graphics.drawable.Icon
import android.os.Build
import io.ahara.linkdrop.MainActivity
import io.ahara.linkdrop.R

object ShareShortcutPublisher {
    const val TEXT_SHORTCUT_ID = "linkdrop-direct-text"
    const val IMAGE_SHORTCUT_ID = "linkdrop-direct-image"

    private const val TEXT_CATEGORY = "io.ahara.linkdrop.category.TEXT_SHARE_TARGET"
    private const val IMAGE_CATEGORY = "io.ahara.linkdrop.category.IMAGE_SHARE_TARGET"

    fun publish(context: Context) {
        val manager = shortcutManager(context) ?: return
        if (manager.isRateLimitingActive) {
            return
        }
        runCatching {
            manager.setDynamicShortcuts(
                listOf(
                    textShortcut(context),
                    imageShortcut(context),
                ),
            )
        }
    }

    fun reportUsed(context: Context, shortcutId: String) {
        if (shortcutId != TEXT_SHORTCUT_ID && shortcutId != IMAGE_SHORTCUT_ID) {
            return
        }
        runCatching { shortcutManager(context)?.reportShortcutUsed(shortcutId) }
    }

    private fun textShortcut(context: Context): ShortcutInfo =
        shareShortcut(
            context = context,
            shortcutId = TEXT_SHORTCUT_ID,
            shortLabel = R.string.shortcut_drop_text_short,
            longLabel = R.string.shortcut_drop_text_long,
            category = TEXT_CATEGORY,
        )

    private fun imageShortcut(context: Context): ShortcutInfo =
        shareShortcut(
            context = context,
            shortcutId = IMAGE_SHORTCUT_ID,
            shortLabel = R.string.shortcut_drop_image_short,
            longLabel = R.string.shortcut_drop_image_long,
            category = IMAGE_CATEGORY,
        )

    private fun shareShortcut(
        context: Context,
        shortcutId: String,
        shortLabel: Int,
        longLabel: Int,
        category: String,
    ): ShortcutInfo {
        val builder = ShortcutInfo.Builder(context, shortcutId)
            .setShortLabel(context.getString(shortLabel))
            .setLongLabel(context.getString(longLabel))
            .setIcon(Icon.createWithResource(context, R.drawable.ic_linkdrop_share))
            .setCategories(setOf(category))
            .setIntent(shortcutLauncherIntent(context, shortcutId))
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
            builder.setLongLived(true)
        }
        return builder.build()
    }

    private fun shortcutLauncherIntent(context: Context, shortcutId: String): Intent =
        Intent(context, MainActivity::class.java)
            .setAction(Intent.ACTION_VIEW)
            .putExtra(Intent.EXTRA_SHORTCUT_ID, shortcutId)

    private fun shortcutManager(context: Context): ShortcutManager? =
        context.getSystemService(ShortcutManager::class.java)
}
