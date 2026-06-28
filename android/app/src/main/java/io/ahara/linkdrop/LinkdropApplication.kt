package io.ahara.linkdrop

import android.app.Application
import io.ahara.linkdrop.share.ShareShortcutPublisher

class LinkdropApplication : Application() {
    override fun onCreate() {
        super.onCreate()
        ShareShortcutPublisher.publish(this)
    }
}
