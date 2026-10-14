// Full-screen playback with Media3/ExoPlayer, which uses the TV's hardware
// decoders. Remembers where you stopped and plays the next episode on its own.
package io.github.okaza03.localplayer.ui

import androidx.annotation.OptIn
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.width
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.compose.ui.viewinterop.AndroidView
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleEventObserver
import androidx.lifecycle.compose.LocalLifecycleOwner
import androidx.media3.common.AudioAttributes
import androidx.media3.common.C
import androidx.media3.common.MediaItem
import androidx.media3.common.PlaybackException
import androidx.media3.common.Player
import androidx.media3.common.util.UnstableApi
import androidx.media3.datasource.okhttp.OkHttpDataSource
import androidx.media3.exoplayer.DefaultRenderersFactory
import androidx.media3.exoplayer.ExoPlayer
import androidx.media3.exoplayer.source.DefaultMediaSourceFactory
import androidx.media3.ui.PlayerView
import androidx.tv.material3.Text
import io.github.okaza03.localplayer.AppModel
import io.github.okaza03.localplayer.Client
import io.github.okaza03.localplayer.Item
import io.github.okaza03.localplayer.Screen
import kotlinx.coroutines.delay

/** Save the position this often while playing. */
private const val SAVE_EVERY_MS = 5_000L

@OptIn(UnstableApi::class)
@Composable
fun PlayerScreen(model: AppModel, item: Item, screen: Screen.Player) {
    val episode = screen.episodeId?.let { id -> item.episodes.find { it.id == id } }
    val key = episode?.id ?: item.id
    val url = episode?.stream ?: item.stream ?: return
    val context = LocalContext.current
    var error by remember(key) { mutableStateOf<String?>(null) }

    val player = remember(key) {
        val auth = model.client?.authFor(url)
        val http = OkHttpDataSource.Factory(Client.http)
            .setDefaultRequestProperties(if (auth != null) mapOf("Authorization" to auth) else emptyMap())
        ExoPlayer.Builder(context)
            .setMediaSourceFactory(DefaultMediaSourceFactory(http))
            // try the next decoder when the preferred one fails on this device
            .setRenderersFactory(DefaultRenderersFactory(context).setEnableDecoderFallback(true))
            .setAudioAttributes(
                AudioAttributes.Builder().setUsage(C.USAGE_MEDIA).setContentType(C.AUDIO_CONTENT_TYPE_MOVIE).build(),
                true,
            )
            .setSeekBackIncrementMs(10_000)
            .setSeekForwardIncrementMs(10_000)
            .build()
            .apply {
                val resume = model.entry(key)?.takeIf { it.inProgress && !screen.fromStart }
                setMediaItem(MediaItem.fromUri(url), ((resume?.pos ?: 0.0) * 1000).toLong())
                prepare()
                playWhenReady = true
            }
    }

    fun save() {
        val dur = player.duration.takeIf { it != C.TIME_UNSET && it > 0 } ?: return
        model.save(key, player.currentPosition / 1000.0, dur / 1000.0)
    }

    DisposableEffect(player) {
        val listener = object : Player.Listener {
            override fun onPlaybackStateChanged(state: Int) {
                if (state != Player.STATE_ENDED) return
                save()
                val next = if (episode != null) model.episodeAfter(item, episode.id) else null
                if (next != null) model.swap(Screen.Player(item.id, next.id)) else model.back()
            }

            override fun onPlayerError(e: PlaybackException) {
                error = "Can't play this: ${e.errorCodeName}" + (e.cause?.message?.let { "\n$it" } ?: "")
            }
        }
        player.addListener(listener)
        onDispose {
            save()
            player.removeListener(listener)
            player.release()
            model.playerClosed()
        }
    }

    // pause when the TV's Home button sends the app to the background
    val lifecycle = LocalLifecycleOwner.current.lifecycle
    DisposableEffect(lifecycle, player) {
        val observer = LifecycleEventObserver { _, event ->
            if (event == Lifecycle.Event.ON_STOP) {
                player.pause()
                save()
                model.pushNow()
            }
        }
        lifecycle.addObserver(observer)
        onDispose { lifecycle.removeObserver(observer) }
    }

    LaunchedEffect(player) {
        while (true) {
            delay(SAVE_EVERY_MS)
            if (player.isPlaying) save()
        }
    }

    Box(Modifier.fillMaxSize().background(Color.Black)) {
        // a fresh view per episode, so the remote keeps working after autoplay
        androidx.compose.runtime.key(key) {
        AndroidView(
            factory = { ctx ->
                PlayerView(ctx).apply {
                    useController = true
                    controllerShowTimeoutMs = 4_000
                    setShowSubtitleButton(true)
                    setShowNextButton(false)
                    setShowPreviousButton(false)
                    setShowBuffering(PlayerView.SHOW_BUFFERING_ALWAYS)
                    keepScreenOn = true
                    isFocusable = true
                    isFocusableInTouchMode = true
                    post { requestFocus() }
                }
            },
            update = { it.player = player },
            modifier = Modifier.fillMaxSize(),
        )
        }
        error?.let {
            Column(Modifier.align(Alignment.Center).width(640.dp)) {
                Text(it, color = ERROR, fontSize = 18.sp)
                Spacer(Modifier.height(12.dp))
                Text("Press Back to return.", color = MUTED, fontSize = 15.sp)
            }
        }
    }
}
