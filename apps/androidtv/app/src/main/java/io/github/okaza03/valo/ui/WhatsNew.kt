// "What's new": after an update, what changed (the desktop app's
// apps/valo/src/ui/whats_new.rs). Only the big things go in NOTES.
package io.github.okaza03.valo.ui

import androidx.activity.compose.BackHandler
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.tv.material3.Text
import io.github.okaza03.valo.Updates

class Note(val title: String, val text: String)

/** Newest last. TV-side changes only; leave small fixes out. */
private val NOTES: List<Pair<String, List<Note>>> = listOf(
    "0.8.2" to listOf(
        Note("Updates in one go", "Choosing Update downloads the new version and opens the installer right away."),
        Note("What's new", "After an update, this page shows what changed."),
    ),
    "0.8.3" to listOf(
        Note("Subtitles", "Subtitles in a movie's or series' Subs folder on the server can be picked with the subtitle button, and the language you pick comes on by itself next time."),
    ),
)

/** The notes for versions after `seen`, up to this build; a fallback when none were written. */
fun notesSince(seen: String): List<Note> {
    val notes = NOTES.filter { (v, _) -> Updates.newer(v, seen) && !Updates.newer(v, Updates.VERSION) }
        .flatMap { it.second }
    return notes.ifEmpty { listOf(Note("Fixes and small improvements", "Valo ${Updates.VERSION} makes things work better.")) }
}

@Composable
fun WhatsNew(notes: List<Note>, onClose: () -> Unit) {
    val ok = remember { FocusRequester() }
    LaunchedEffect(Unit) { runCatching { ok.requestFocus() } }
    BackHandler(onBack = onClose)
    Box(Modifier.fillMaxSize().background(Color.Black.copy(alpha = 0.7f)), contentAlignment = Alignment.Center) {
        Column(
            Modifier
                .width(620.dp)
                .clip(RoundedCornerShape(4.dp))
                .background(BG)
                .border(1.dp, LINE, RoundedCornerShape(4.dp))
                .padding(32.dp),
            verticalArrangement = Arrangement.spacedBy(6.dp),
        ) {
            MonoText("Valo ${Updates.VERSION}", 12.sp, MUTED)
            Text("What's new", fontSize = 28.sp, fontWeight = FontWeight.Bold, color = TEXT)
            Spacer(Modifier.height(10.dp))
            for (n in notes) {
                Text(n.title, fontSize = 18.sp, fontWeight = FontWeight.Bold, color = TEXT)
                Text(n.text, fontSize = 15.sp, color = MUTED)
                Spacer(Modifier.height(8.dp))
            }
            Spacer(Modifier.height(8.dp))
            TvButton("Got it", onClose, Modifier.focusRequester(ok), primary = true)
            Spacer(Modifier.height(4.dp))
            Text("Full release notes: github.com/Okaza03/Valo/releases", fontSize = 13.sp, color = MUTED)
        }
    }
}
