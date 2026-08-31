package com.protagonistss.luma.plugin

import android.content.pm.ActivityInfo
import android.os.Bundle
import android.view.KeyEvent
import android.view.View
import android.view.WindowManager
import android.widget.Button
import android.widget.FrameLayout
import android.widget.LinearLayout
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity
import androidx.media3.common.MediaItem
import androidx.media3.common.PlaybackException
import androidx.media3.common.Player
import androidx.media3.exoplayer.ExoPlayer
import androidx.media3.datasource.DefaultHttpDataSource
import androidx.media3.exoplayer.source.DefaultMediaSourceFactory
import androidx.media3.ui.PlayerView

class PlayerActivity : AppCompatActivity() {
  companion object {
    const val EXTRA_CHANNEL_ID = "channel_id"
    const val EXTRA_CHANNEL_NAME = "channel_name"
    const val EXTRA_STREAM_URL = "stream_url"
    private val RETRY_DELAYS_MS = longArrayOf(2000, 5000, 10000)
  }

  private var player: ExoPlayer? = null
  private lateinit var playerView: PlayerView
  private lateinit var overlay: LinearLayout
  private lateinit var statusText: TextView
  private lateinit var retryButton: Button
  private lateinit var backButton: Button

  private var retryAttempt = 0
  private var streamUrl: String = ""
  private var channelName: String = ""

  override fun onCreate(savedInstanceState: Bundle?) {
    super.onCreate(savedInstanceState)
    requestedOrientation = ActivityInfo.SCREEN_ORIENTATION_SENSOR_LANDSCAPE
    window.addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)

    streamUrl = intent.getStringExtra(EXTRA_STREAM_URL).orEmpty()
    channelName = intent.getStringExtra(EXTRA_CHANNEL_NAME).orEmpty()

    playerView = PlayerView(this)
    overlay = LinearLayout(this).apply {
      orientation = LinearLayout.VERTICAL
      setPadding(48, 48, 48, 48)
      visibility = View.GONE
    }
    statusText = TextView(this)
    retryButton = Button(this).apply {
      text = "重试"
      setOnClickListener { startPlayback(resetRetry = true) }
    }
    backButton = Button(this).apply {
      text = "返回"
      setOnClickListener { finish() }
    }
    overlay.addView(statusText)
    overlay.addView(retryButton)
    overlay.addView(backButton)

    val root = FrameLayout(this)
    root.addView(
      playerView,
      FrameLayout.LayoutParams(
        FrameLayout.LayoutParams.MATCH_PARENT,
        FrameLayout.LayoutParams.MATCH_PARENT,
      ),
    )
    root.addView(
      overlay,
      FrameLayout.LayoutParams(
        FrameLayout.LayoutParams.MATCH_PARENT,
        FrameLayout.LayoutParams.MATCH_PARENT,
      ),
    )
    setContentView(root)

    if (streamUrl.isBlank()) {
      showError("无效的播放地址")
      return
    }

    initializePlayer()
    startPlayback(resetRetry = true)
  }

  private fun initializePlayer() {
    val dataSourceFactory = DefaultHttpDataSource.Factory()
      .setAllowCrossProtocolRedirects(true)
      .setConnectTimeoutMs(15_000)
      .setReadTimeoutMs(15_000)

    val mediaSourceFactory = DefaultMediaSourceFactory(this)
      .setDataSourceFactory(dataSourceFactory)

    player = ExoPlayer.Builder(this)
      .setMediaSourceFactory(mediaSourceFactory)
      .build()
      .also { exoPlayer ->
        playerView.player = exoPlayer
        exoPlayer.addListener(object : Player.Listener {
          override fun onPlaybackStateChanged(playbackState: Int) {
            when (playbackState) {
              Player.STATE_BUFFERING -> showLoading("正在缓冲...")
              Player.STATE_READY -> hideOverlay()
              Player.STATE_ENDED -> showError("直播已结束")
            }
          }

          override fun onPlayerError(error: PlaybackException) {
            if (error.errorCode == PlaybackException.ERROR_CODE_BEHIND_LIVE_WINDOW) {
              exoPlayer.seekToDefaultPosition()
              exoPlayer.prepare()
              return
            }
            scheduleRetry(error.localizedMessage ?: "播放失败")
          }
        })
      }
  }

  private fun startPlayback(resetRetry: Boolean) {
    if (resetRetry) {
      retryAttempt = 0
    }
    hideOverlay()
    showLoading("正在连接直播...")

    val mediaItem = MediaItem.Builder()
      .setUri(streamUrl)
      .setMediaMetadata(
        androidx.media3.common.MediaMetadata.Builder()
          .setTitle(channelName)
          .build(),
      )
      .setLiveConfiguration(
        MediaItem.LiveConfiguration.Builder()
          .setTargetOffsetMs(5000)
          .setMaxPlaybackSpeed(1.02f)
          .build(),
      )
      .build()

    player?.apply {
      setMediaItem(mediaItem)
      prepare()
      playWhenReady = true
    }
  }

  private fun scheduleRetry(message: String) {
    if (retryAttempt >= RETRY_DELAYS_MS.size) {
      showError(message)
      return
    }

    val delay = RETRY_DELAYS_MS[retryAttempt]
    retryAttempt += 1
    showLoading("播放失败，${delay / 1000} 秒后重试...")
    playerView.postDelayed({ startPlayback(resetRetry = false) }, delay)
  }

  private fun showLoading(message: String) {
    overlay.visibility = View.VISIBLE
    statusText.text = message
    retryButton.visibility = View.GONE
    backButton.visibility = View.GONE
  }

  private fun showError(message: String) {
    overlay.visibility = View.VISIBLE
    statusText.text = "$channelName\n$message"
    retryButton.visibility = View.VISIBLE
    backButton.visibility = View.VISIBLE
  }

  private fun hideOverlay() {
    overlay.visibility = View.GONE
  }

  override fun onStop() {
    super.onStop()
    player?.pause()
  }

  override fun onDestroy() {
    playerView.player = null
    player?.release()
    player = null
    super.onDestroy()
  }

  override fun onKeyDown(keyCode: Int, event: KeyEvent?): Boolean {
    if (keyCode == KeyEvent.KEYCODE_BACK) {
      finish()
      return true
    }
    return super.onKeyDown(keyCode, event)
  }
}
