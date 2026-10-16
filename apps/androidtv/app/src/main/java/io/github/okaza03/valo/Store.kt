// What the app remembers between runs: the server, the sign-in and where
// each title was left off.
package io.github.okaza03.valo

import android.content.Context
import kotlinx.serialization.Serializable
import java.io.File

class Prefs(context: Context) {
    private val p = context.getSharedPreferences("valo", Context.MODE_PRIVATE)

    var server: String?
        get() = p.getString("server", null)
        set(v) = p.edit().putString("server", v).apply()

    var token: String?
        get() = p.getString("token", null)
        set(v) = p.edit().putString("token", v).apply()

    var user: String?
        get() = p.getString("user", null)
        set(v) = p.edit().putString("user", v).apply()

    /** Subtitles in this language are turned on by themselves; null = off. */
    var subtitleLanguage: String?
        get() = p.getString("subtitle_language", null)
        set(v) = p.edit().putString("subtitle_language", v).apply()

    /** The last version whose "What's new" was shown. */
    var seenVersion: String?
        get() = p.getString("seen_version", null)
        set(v) = p.edit().putString("seen_version", v).apply()
}

@Serializable
data class Entry(val pos: Double, val dur: Double, val updated: Long) {
    val fraction get() = if (dur > 0) (pos / dur).coerceIn(0.0, 1.0).toFloat() else 0f

    /** Started, but not finished (95% watched counts as finished, which skips the end credits). */
    val inProgress get() = pos > 30 && (dur <= 0 || pos < dur * 0.95)

    val watched get() = dur > 0 && !inProgress && pos > 30
}

/**
 * Resume points keyed by "<server>|<id>". An entry at position 0 is a cleared
 * one, kept so the clearing reaches the server (see Sync).
 */
class Progress(private val file: File) {
    @Serializable
    private data class Saved(val entries: Map<String, Entry> = emptyMap(), val unsent: Set<String> = emptySet())

    private val entries = HashMap<String, Entry>()
    private val unsent = HashSet<String>()

    init {
        runCatching { json.decodeFromString<Saved>(file.readText()) }.onSuccess {
            entries.putAll(it.entries)
            unsent.addAll(it.unsent)
        }
    }

    @Synchronized
    fun get(key: String): Entry? = entries[key]?.takeIf { it.pos > 0 }

    @Synchronized
    fun set(key: String, pos: Double, dur: Double) {
        entries[key] = Entry(pos, dur, System.currentTimeMillis() / 1000)
        unsent += key
    }

    /** Entries for one server, newest first. */
    @Synchronized
    fun recent(prefix: String): List<Pair<String, Entry>> = entries
        .filter { (k, e) -> e.pos > 0 && k.startsWith(prefix) }
        .map { (k, e) -> k.removePrefix(prefix) to e }
        .sortedByDescending { it.second.updated }

    /** Take the server's entries where they're newer. Returns whether anything changed. */
    @Synchronized
    fun merge(prefix: String, remote: List<Pair<String, Entry>>): Boolean {
        var changed = false
        for ((id, e) in remote) {
            val key = prefix + id
            if ((entries[key]?.updated ?: Long.MIN_VALUE) >= e.updated) continue
            unsent -= key
            entries[key] = e
            changed = true
        }
        return changed
    }

    /** Changes for one server that still have to be sent. */
    @Synchronized
    fun unsent(prefix: String): List<Pair<String, Entry>> = unsent
        .filter { it.startsWith(prefix) }
        .mapNotNull { k -> entries[k]?.let { k.removePrefix(prefix) to it } }

    /**
     * The server has `id` as it was in `sent`. Unless it changed here in the
     * meantime, stop sending it and take the server's timestamp, so the next
     * pull doesn't mistake the server's copy for a newer one.
     */
    @Synchronized
    fun sent(prefix: String, id: String, sent: Entry, serverUpdated: Long?) {
        val key = prefix + id
        val local = entries[key] ?: return
        if (local.updated != sent.updated) return
        if (serverUpdated != null) entries[key] = local.copy(updated = serverUpdated)
        unsent -= key
    }

    @Synchronized
    fun flush() {
        runCatching {
            val tmp = File(file.path + ".tmp")
            tmp.writeText(json.encodeToString(Saved.serializer(), Saved(entries.toMap(), unsent.toSet())))
            tmp.renameTo(file)
        }
    }
}
