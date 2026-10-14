package io.github.okaza03.localplayer

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.BackHandler
import androidx.activity.compose.setContent
import androidx.activity.viewModels
import androidx.compose.runtime.Composable
import coil3.ImageLoader
import coil3.compose.setSingletonImageLoaderFactory
import coil3.network.okhttp.OkHttpNetworkFetcherFactory
import coil3.request.crossfade
import io.github.okaza03.localplayer.ui.DetailsScreen
import io.github.okaza03.localplayer.ui.HomeScreen
import io.github.okaza03.localplayer.ui.LoadingScreen
import io.github.okaza03.localplayer.ui.LocalPlayerTheme
import io.github.okaza03.localplayer.ui.LoginScreen
import io.github.okaza03.localplayer.ui.PlayerScreen
import io.github.okaza03.localplayer.ui.SetupScreen

class MainActivity : ComponentActivity() {
    private val model: AppModel by viewModels()

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        // images on our server need the sign-in token; ones on a CDN don't get it
        val images = Client.http.newBuilder().addInterceptor { chain ->
            val req = chain.request()
            val auth = model.client?.authFor(req.url.toString())
            chain.proceed(if (auth != null) req.newBuilder().header("Authorization", auth).build() else req)
        }.build()
        setContent {
            setSingletonImageLoaderFactory { ctx ->
                ImageLoader.Builder(ctx)
                    .components { add(OkHttpNetworkFetcherFactory(callFactory = { images })) }
                    .crossfade(true)
                    .build()
            }
            LocalPlayerTheme {
                BackHandler { if (!model.back()) finish() }
                App(model)
            }
        }
    }

    override fun onStop() {
        super.onStop()
        model.pushNow()
    }
}

@Composable
private fun App(model: AppModel) {
    val lib = model.library
    when (val s = model.screen) {
        is Screen.Loading -> LoadingScreen(s.message)
        is Screen.Setup -> SetupScreen(model, s.error)
        is Screen.Login -> LoginScreen(model, s.error)
        Screen.Home -> if (lib != null) HomeScreen(model, lib)
        is Screen.Details -> lib?.find(s.itemId)?.let { DetailsScreen(model, it) }
        is Screen.Player -> lib?.find(s.itemId)?.let { PlayerScreen(model, it, s) }
    }
}
