// Shared look: colors, artwork, cards, buttons and text fields sized for a
// TV across the room and driven by the remote's D-pad.
package io.github.okaza03.localplayer.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxScope
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.focus.onFocusChanged
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.text.input.VisualTransformation
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.tv.material3.Border
import androidx.tv.material3.Button
import androidx.tv.material3.ButtonDefaults
import androidx.tv.material3.Card
import androidx.tv.material3.CardDefaults
import androidx.tv.material3.MaterialTheme
import androidx.tv.material3.Text
import androidx.tv.material3.darkColorScheme
import coil3.compose.AsyncImage
import io.github.okaza03.localplayer.Item
import io.github.okaza03.localplayer.Kind
import kotlin.math.roundToInt

// the desktop app's palette (apps/localplayer/src/ui/mod.rs)
val BG = Color(0xFF121317)
val SURFACE = Color(0xFF191A1F)
val SURFACE_HI = Color(0xFF222329)
val ACCENT = Color(0xFF4A7CF7)
val TEXT = Color(0xFFECECEE)
val MUTED = Color(0xFF8B8D94)
val ERROR = Color(0xFFFF786E)

/** Space kept clear at the screen's edges, which TVs may crop. */
val EDGE = 48.dp

@Composable
fun LocalPlayerTheme(content: @Composable () -> Unit) {
    MaterialTheme(
        colorScheme = darkColorScheme(
            primary = ACCENT,
            onPrimary = Color.White,
            background = BG,
            surface = SURFACE,
            onSurface = TEXT,
            surfaceVariant = SURFACE_HI,
            onSurfaceVariant = MUTED,
            error = ERROR,
        ),
        content = content,
    )
}

/** A server image; a plain tile while it loads or when there is none. */
@Composable
fun Art(url: String?, modifier: Modifier = Modifier, scale: ContentScale = ContentScale.Crop) {
    Box(modifier.background(SURFACE_HI)) {
        if (url != null) {
            AsyncImage(model = url, contentDescription = null, contentScale = scale, modifier = Modifier.fillMaxSize())
        }
    }
}

/** Big artwork behind a screen, fading into the background on the left and bottom. */
@Composable
fun BoxScope.Backdrop(url: String?) {
    if (url == null) return
    Box(Modifier.align(Alignment.TopEnd).fillMaxWidth(0.72f).fillMaxHeight(0.75f)) {
        AsyncImage(model = url, contentDescription = null, contentScale = ContentScale.Crop, modifier = Modifier.fillMaxSize())
        Box(Modifier.fillMaxSize().background(Brush.horizontalGradient(0f to BG, 0.45f to BG.copy(alpha = 0.35f), 1f to Color.Transparent)))
        Box(Modifier.fillMaxSize().background(Brush.verticalGradient(0.5f to Color.Transparent, 1f to BG)))
    }
}

@Composable
fun ProgressBar(fraction: Float, modifier: Modifier = Modifier) {
    Box(modifier.height(4.dp).background(Color.White.copy(alpha = 0.25f))) {
        Box(Modifier.fillMaxHeight().fillMaxWidth(fraction.coerceIn(0f, 1f)).background(ACCENT))
    }
}

/** One artwork card with a caption, for rows. */
@Composable
fun ArtCard(
    art: String?,
    title: String,
    sub: String?,
    width: Dp,
    ratio: Float,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    progress: Float? = null,
    onLongClick: (() -> Unit)? = null,
    onFocus: () -> Unit = {},
    focusRequester: FocusRequester? = null,
) {
    var focused by remember { mutableStateOf(false) }
    Column(modifier.width(width)) {
        Card(
            onClick = onClick,
            onLongClick = onLongClick,
            shape = CardDefaults.shape(RoundedCornerShape(8.dp)),
            border = CardDefaults.border(
                focusedBorder = Border(androidx.compose.foundation.BorderStroke(3.dp, Color.White), shape = RoundedCornerShape(8.dp)),
            ),
            modifier = Modifier
                .fillMaxWidth()
                .aspectRatio(ratio)
                .then(if (focusRequester != null) Modifier.focusRequester(focusRequester) else Modifier)
                .onFocusChanged {
                    focused = it.isFocused
                    if (it.isFocused) onFocus()
                },
        ) {
            Box(Modifier.fillMaxSize()) {
                Art(art, Modifier.fillMaxSize())
                if (art == null) {
                    Text(title, color = MUTED, fontSize = 13.sp, modifier = Modifier.align(Alignment.Center).padding(8.dp))
                }
                if (progress != null && progress > 0f) {
                    ProgressBar(progress, Modifier.align(Alignment.BottomStart).fillMaxWidth())
                }
            }
        }
        Spacer(Modifier.height(8.dp))
        Text(
            title, maxLines = 1, overflow = TextOverflow.Ellipsis, fontSize = 14.sp,
            color = if (focused) TEXT else MUTED, fontWeight = if (focused) FontWeight.SemiBold else FontWeight.Normal,
        )
        if (sub != null) {
            Text(sub, maxLines = 1, overflow = TextOverflow.Ellipsis, fontSize = 12.sp, color = MUTED)
        }
    }
}

@Composable
fun TvButton(
    text: String,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    primary: Boolean = false,
    selected: Boolean = false,
    enabled: Boolean = true,
) {
    Button(
        onClick = onClick,
        enabled = enabled,
        modifier = modifier,
        colors = ButtonDefaults.colors(
            containerColor = if (primary || selected) ACCENT.copy(alpha = if (primary) 1f else 0.35f) else SURFACE_HI,
            contentColor = TEXT,
            focusedContainerColor = Color.White,
            focusedContentColor = Color.Black,
        ),
    ) {
        Text(text, fontSize = 15.sp, fontWeight = FontWeight.SemiBold)
    }
}

/** A text field that shows the on-screen keyboard when the remote's OK button is pressed. */
@Composable
fun TvTextField(
    value: String,
    onChange: (String) -> Unit,
    label: String,
    modifier: Modifier = Modifier,
    password: Boolean = false,
    keyboard: KeyboardType = KeyboardType.Text,
    imeAction: ImeAction = ImeAction.Next,
    onDone: () -> Unit = {},
) {
    var focused by remember { mutableStateOf(false) }
    Column(modifier) {
        Text(label, color = MUTED, fontSize = 14.sp)
        Spacer(Modifier.height(6.dp))
        BasicTextField(
            value = value,
            onValueChange = onChange,
            singleLine = true,
            textStyle = TextStyle(color = TEXT, fontSize = 20.sp),
            cursorBrush = SolidColor(ACCENT),
            visualTransformation = if (password) PasswordVisualTransformation() else VisualTransformation.None,
            keyboardOptions = KeyboardOptions(
                keyboardType = if (password) KeyboardType.Password else keyboard,
                imeAction = imeAction,
                autoCorrectEnabled = false,
            ),
            keyboardActions = KeyboardActions(onDone = { onDone() }, onGo = { onDone() }),
            modifier = Modifier
                .fillMaxWidth()
                .onFocusChanged { focused = it.isFocused }
                .clip(RoundedCornerShape(8.dp))
                .background(SURFACE_HI)
                .border(2.dp, if (focused) Color.White else Color.Transparent, RoundedCornerShape(8.dp))
                .padding(horizontal = 16.dp, vertical = 14.dp),
        )
    }
}

fun fmtRuntime(seconds: Double): String {
    val m = (seconds / 60).roundToInt()
    return if (m >= 60) "${m / 60}h ${m % 60}m" else "${m}m"
}

/** "2010 · 2h 28m · PG-13 · Sci-Fi, Action" */
fun metaLine(item: Item): String = buildList {
    item.year?.let { y -> add(if (item.endYear != null && item.endYear != y) "$y–${item.endYear}" else "$y") }
    when (item.kind) {
        Kind.Movie -> item.duration?.takeIf { it > 0 }?.let { add(fmtRuntime(it)) }
        Kind.Series -> {
            val n = item.seasons.size
            add("$n season${if (n == 1) "" else "s"}")
        }
    }
    item.rating?.let { add(it) }
    item.score?.let { add("★ %.1f".format(it)) }
    if (item.genres.isNotEmpty()) add(item.genres.take(3).joinToString(", "))
}.joinToString("  ·  ")
