// A movie's or series' page: artwork, details, play buttons, and for series
// the seasons and their episodes.
package io.github.okaza03.valo.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.tv.material3.Text
import io.github.okaza03.valo.AppModel
import io.github.okaza03.valo.Item
import io.github.okaza03.valo.Kind
import io.github.okaza03.valo.Screen

@Composable
fun DetailsScreen(model: AppModel, item: Item) {
    val play = remember { FocusRequester() }
    LaunchedEffect(item.id) { runCatching { play.requestFocus() } }
    val next = if (item.kind == Kind.Series) model.nextUp(item) else null
    // start on the season of the episode that's up next
    var season by rememberSaveable(item.id) {
        mutableIntStateOf(item.seasons.indexOfFirst { it == next?.season }.coerceAtLeast(0))
    }

    Box(Modifier.fillMaxSize().background(BG)) {
        Backdrop(item.backdrop ?: item.poster)
        LazyColumn(
            contentPadding = PaddingValues(top = EDGE, bottom = EDGE),
            verticalArrangement = Arrangement.spacedBy(24.dp),
            modifier = Modifier.fillMaxSize(),
        ) {
            item {
                Column(Modifier.padding(horizontal = EDGE).fillMaxWidth(0.55f)) {
                    if (item.logo != null) {
                        Logo(item.logo, item.title, Modifier.height(100.dp).width(400.dp))
                    } else {
                        Text(item.title, fontSize = 40.sp, fontWeight = FontWeight.Bold, color = TEXT)
                    }
                    Spacer(Modifier.height(12.dp))
                    Text(metaLine(item), color = MUTED, fontSize = 16.sp)
                    Spacer(Modifier.height(12.dp))
                    Text(item.overview, color = TEXT, fontSize = 16.sp, maxLines = 5, overflow = TextOverflow.Ellipsis)
                    credits(item).forEach { (label, text) ->
                        Spacer(Modifier.height(8.dp))
                        Text(
                            "$label  $text", color = MUTED, fontSize = 14.sp,
                            maxLines = 1, overflow = TextOverflow.Ellipsis,
                        )
                    }
                    Spacer(Modifier.height(24.dp))
                    Row(horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                        when {
                            item.kind == Kind.Movie -> {
                                val e = model.entry(item.id)?.takeIf { it.inProgress }
                                TvButton(
                                    if (e != null) "Resume  ·  ${fmtRuntime(e.dur - e.pos)} left" else "Play",
                                    { model.push(Screen.Player(item.id)) },
                                    primary = true, modifier = Modifier.focusRequester(play),
                                )
                                if (e != null) {
                                    TvButton("Start over", { model.push(Screen.Player(item.id, fromStart = true)) })
                                }
                            }
                            next != null -> {
                                val ep = next.episode
                                val what = "S${next.season.number} E${ep.number}"
                                TvButton(
                                    when {
                                        next.resume != null -> "Resume $what"
                                        next.started -> "Play $what"
                                        else -> "Play"
                                    },
                                    { model.push(Screen.Player(item.id, ep.id)) },
                                    primary = true, modifier = Modifier.focusRequester(play),
                                )
                                if (next.resume != null) {
                                    TvButton("Start over", { model.push(Screen.Player(item.id, ep.id, fromStart = true)) })
                                }
                            }
                        }
                    }
                }
            }

            if (item.kind == Kind.Series && item.seasons.size > 1) {
                item {
                    LazyRow(
                        contentPadding = PaddingValues(horizontal = EDGE),
                        horizontalArrangement = Arrangement.spacedBy(8.dp),
                    ) {
                        itemsIndexed(item.seasons) { i, s ->
                            TvButton(s.label, { season = i }, selected = i == season)
                        }
                    }
                }
            }

            if (item.kind == Kind.Series) {
                item(key = "episodes/$season") {
                    val s = item.seasons.getOrNull(season) ?: return@item
                    Column {
                        MonoText(s.label, 13.sp, modifier = Modifier.padding(start = EDGE, bottom = 10.dp))
                        val state = rememberLazyListState(
                            s.episodes.indexOfFirst { it.id == next?.episode?.id }.coerceAtLeast(0),
                        )
                        LazyRow(
                            state = state,
                            contentPadding = PaddingValues(horizontal = EDGE),
                            horizontalArrangement = Arrangement.spacedBy(16.dp),
                        ) {
                            items(s.episodes, key = { it.id }) { ep ->
                                val e = model.entry(ep.id)
                                val sub = buildList {
                                    add("E${ep.number}")
                                    ep.duration?.takeIf { it > 0 }?.let { add(fmtRuntime(it)) }
                                    if (e?.watched == true) add("Watched")
                                }.joinToString(" · ")
                                ArtCard(
                                    art = ep.still ?: item.backdrop, title = ep.title.ifEmpty { "Episode ${ep.number}" },
                                    sub = sub, width = 280.dp, ratio = 16f / 9f,
                                    progress = e?.takeIf { it.inProgress }?.fraction,
                                    onClick = { model.push(Screen.Player(item.id, ep.id)) },
                                    // hold OK to play from the beginning
                                    onLongClick = { model.push(Screen.Player(item.id, ep.id, fromStart = true)) },
                                )
                            }
                        }
                    }
                }
            }
        }
    }
}

private fun credits(item: Item): List<Pair<String, String>> = buildList {
    item.director?.let { add("Director" to it) }
    if (item.creators.isNotEmpty()) add("Created by" to item.creators.joinToString(", "))
    if (item.cast.isNotEmpty()) add("Starring" to item.cast.take(5).joinToString(", ") { it.name })
    item.network?.let { add("Network" to it) }
}
