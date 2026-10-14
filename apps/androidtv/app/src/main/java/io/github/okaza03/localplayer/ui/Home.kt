// The home page: the focused title's artwork and details on top, rows of
// titles below (Continue watching, Recently added, Movies, TV shows, genres).
package io.github.okaza03.localplayer.ui

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
import androidx.compose.foundation.lazy.LazyListState
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.tv.material3.Text
import io.github.okaza03.localplayer.AppModel
import io.github.okaza03.localplayer.Item
import io.github.okaza03.localplayer.Kind
import io.github.okaza03.localplayer.Library
import io.github.okaza03.localplayer.Screen

private class HomeCard(
    val key: String,
    val item: Item,
    val title: String,
    val sub: String?,
    val art: String?,
    val progress: Float?,
    val open: Screen,
)

private class HomeRow(val title: String, val wide: Boolean, val cards: List<HomeCard>)

private fun rows(model: AppModel, lib: Library): List<HomeRow> {
    val out = ArrayList<HomeRow>()
    fun posters(title: String, items: List<Item>) = HomeRow(title, false, items.map { i ->
        HomeCard(
            "$title/${i.id}", i, i.title, i.year?.toString(), i.poster,
            if (i.kind == Kind.Movie) model.entry(i.id)?.takeIf { it.inProgress }?.fraction else null,
            Screen.Details(i.id),
        )
    })

    val cont = model.continueWatching(lib).map { (item, next) ->
        if (next == null) {
            val e = model.entry(item.id)
            HomeCard(
                "cont/${item.id}", item, item.title, e?.let { "${fmtRuntime(it.dur - it.pos)} left" },
                item.backdrop ?: item.poster, e?.fraction, Screen.Player(item.id),
            )
        } else {
            val ep = next.episode
            HomeCard(
                "cont/${item.id}", item, item.title, "S${next.season.number} E${ep.number} · ${ep.title}",
                ep.still ?: item.backdrop ?: item.poster, next.resume?.fraction ?: 0f, Screen.Player(item.id, ep.id),
            )
        }
    }
    if (cont.isNotEmpty()) out += HomeRow("Continue watching", true, cont)

    val recent = lib.all.filter { it.added != null }.sortedByDescending { it.added }.take(24)
    if (recent.isNotEmpty()) out += posters("Recently added", recent)
    if (lib.movies.isNotEmpty()) out += posters("Movies", lib.movies)
    if (lib.series.isNotEmpty()) out += posters("TV shows", lib.series)
    for (g in lib.genres().take(8)) {
        val items = lib.all.filter { g in it.genres }.take(30)
        if (items.size >= 2) out += posters(g, items)
    }
    return out
}

@Composable
fun HomeScreen(model: AppModel, lib: Library) {
    val rows = rows(model, lib)
    var focused by remember { mutableStateOf(rows.firstOrNull()?.cards?.firstOrNull()?.item) }
    val target = model.homeFocus?.takeIf { k -> rows.any { r -> r.cards.any { it.key == k } } }
        ?: rows.firstOrNull()?.cards?.firstOrNull()?.key
    val requester = remember { FocusRequester() }
    LaunchedEffect(Unit) { runCatching { requester.requestFocus() } }

    Box(Modifier.fillMaxSize().background(BG)) {
        Backdrop(focused?.backdrop ?: focused?.poster)
        Column(Modifier.fillMaxSize()) {
            TopBar(model)
            Hero(focused)
            if (rows.isEmpty()) {
                Text(
                    "No movies or series were found on this server.",
                    color = MUTED, fontSize = 18.sp, modifier = Modifier.padding(horizontal = EDGE),
                )
            }
            LazyColumn(
                state = model.homeList,
                contentPadding = PaddingValues(bottom = EDGE),
                verticalArrangement = Arrangement.spacedBy(24.dp),
                modifier = Modifier.fillMaxWidth().weight(1f),
            ) {
                items(rows, key = { it.title }) { row ->
                    val state = model.homeRows.getOrPut(row.title) { LazyListState() }
                    CardRow(row, state, target, requester, onFocus = { card ->
                        focused = card.item
                        model.homeFocus = card.key
                    }, onClick = { model.push(it.open) })
                }
            }
        }
    }
}

@Composable
private fun TopBar(model: AppModel) {
    Row(
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(12.dp),
        modifier = Modifier.fillMaxWidth().padding(start = EDGE, end = EDGE, top = 24.dp),
    ) {
        Text(model.client?.name ?: "", color = MUTED, fontSize = 15.sp, modifier = Modifier.weight(1f))
        TvButton(if (model.busy) "Refreshing…" else "Refresh", { model.loadLibrary() }, enabled = !model.busy)
        if (model.client?.info?.auth == true) TvButton("Sign out", model::signOut)
        TvButton("Change server", model::changeServer)
    }
}

/** The focused title, large. */
@Composable
private fun Hero(item: Item?) {
    Column(Modifier.fillMaxWidth().height(230.dp).padding(start = EDGE, top = 16.dp)) {
        if (item == null) return@Column
        if (item.logo != null) {
            Art(item.logo, Modifier.height(80.dp).width(320.dp), scale = ContentScale.Fit)
        } else {
            Text(
                item.title, fontSize = 36.sp, fontWeight = FontWeight.Bold, color = TEXT,
                maxLines = 2, overflow = TextOverflow.Ellipsis, modifier = Modifier.fillMaxWidth(0.55f),
            )
        }
        Spacer(Modifier.height(10.dp))
        Text(metaLine(item), color = MUTED, fontSize = 15.sp)
        Spacer(Modifier.height(10.dp))
        Text(
            item.overview, color = TEXT, fontSize = 15.sp, maxLines = 3, overflow = TextOverflow.Ellipsis,
            modifier = Modifier.fillMaxWidth(0.5f),
        )
    }
}

@Composable
private fun CardRow(
    row: HomeRow,
    state: LazyListState,
    target: String?,
    requester: FocusRequester,
    onFocus: (HomeCard) -> Unit,
    onClick: (HomeCard) -> Unit,
) {
    Column {
        Text(
            row.title, fontSize = 20.sp, fontWeight = FontWeight.SemiBold, color = TEXT,
            modifier = Modifier.padding(start = EDGE, bottom = 12.dp),
        )
        LazyRow(
            state = state,
            contentPadding = PaddingValues(horizontal = EDGE),
            horizontalArrangement = Arrangement.spacedBy(16.dp),
        ) {
            items(row.cards, key = { it.key }) { card ->
                ArtCard(
                    art = card.art, title = card.title, sub = card.sub,
                    width = if (row.wide) 260.dp else 140.dp,
                    ratio = if (row.wide) 16f / 9f else 2f / 3f,
                    progress = card.progress,
                    onClick = { onClick(card) },
                    onFocus = { onFocus(card) },
                    focusRequester = if (card.key == target) requester else null,
                )
            }
        }
    }
}
