package com.protagonistss.luma.plugin

import android.app.Activity
import android.content.Intent
import android.util.Log
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.Plugin

@InvokeArg
class OpenPlayerArgs {
  lateinit var channelId: String
  lateinit var name: String
  lateinit var streamUrl: String
}

@TauriPlugin
class PlayerPlugin(private val activity: Activity) : Plugin(activity) {
  private val tag = "PlayerPlugin"

  @Command
  fun openPlayer(invoke: Invoke) {
    val args = invoke.parseArgs(OpenPlayerArgs::class.java)
    Log.d(tag, "openPlayer channel=${args.channelId}")

    val intent = Intent(activity, PlayerActivity::class.java).apply {
      putExtra(PlayerActivity.EXTRA_CHANNEL_ID, args.channelId)
      putExtra(PlayerActivity.EXTRA_CHANNEL_NAME, args.name)
      putExtra(PlayerActivity.EXTRA_STREAM_URL, args.streamUrl)
    }

    activity.startActivity(intent)
    invoke.resolve()
  }
}
