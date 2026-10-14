// The wordmark shown after signing in, while the library loads.
package io.github.okaza03.valo.ui

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.width
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.snapshotFlow
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.unit.dp
import io.github.okaza03.valo.R
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.launch

/** On the way to the library it stays at least this long, so it doesn't just flash. */
private const val HOLD_MS = 1200L

/**
 * Fades in, waits for `ready`, then fades out and calls `done`. `hold` (the
 * library opened) keeps it up for HOLD_MS; an error shows straight away.
 */
@Composable
fun Splash(ready: Boolean, hold: Boolean, done: () -> Unit) {
    val isReady by rememberUpdatedState(ready)
    val isHold by rememberUpdatedState(hold)
    val logo = remember { Animatable(0f) }
    val cover = remember { Animatable(1f) }
    LaunchedEffect(Unit) {
        val start = System.currentTimeMillis()
        launch { logo.animateTo(1f, tween(450)) }
        snapshotFlow { isReady }.first { it }
        if (isHold) delay(HOLD_MS - (System.currentTimeMillis() - start))
        cover.animateTo(0f, tween(350))
        done()
    }
    Box(
        Modifier.fillMaxSize().alpha(cover.value).background(BG),
        contentAlignment = Alignment.Center,
    ) {
        Image(
            painterResource(R.drawable.wordmark),
            contentDescription = "Valo",
            modifier = Modifier.width(280.dp).graphicsLayer {
                alpha = logo.value
                val s = 0.94f + 0.06f * logo.value
                scaleX = s
                scaleY = s
            },
        )
    }
}
