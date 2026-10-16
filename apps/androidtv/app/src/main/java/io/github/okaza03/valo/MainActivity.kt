package io.github.okaza03.valo

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.BackHandler
import androidx.activity.compose.setContent
import androidx.activity.viewModels
import androidx.compose.foundation.layout.Box
import androidx.compose.runtime.Composable
import coil3.ImageLoader
import coil3.compose.setSingletonImageLoaderFactory
import coil3.network.okhttp.OkHttpNetworkFetcherFactory
import coil3.request.crossfade
import io.github.okaza03.valo.ui.DetailsScreen
import io.github.okaza03.valo.ui.GridScreen
import io.github.okaza03.valo.ui.HomeScreen
import io.github.okaza03.valo.ui.LoadingScreen
import io.github.okaza03.valo.ui.ValoTheme
import io.github.okaza03.valo.ui.LoginScreen
import io.github.okaza03.valo.ui.PlayerScreen
import io.github.okaza03.valo.ui.SetupScreen
import io.github.okaza03.valo.ui.Splash
import io.github.okaza03.valo.ui.WhatsNew

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
            ValoTheme {
                BackHandler { if (!model.back()) finish() }
                Box {
                    App(model)
                    val notes = model.whatsNew
                    if (notes != null && !model.splash && model.screen == Screen.Home) {
                        WhatsNew(notes, model::closeWhatsNew)
                    }
                    if (model.splash) Splash(
                        ready = model.screen !is Screen.Loading,
                        hold = model.screen == Screen.Home,
                    ) { model.splash = false }
                }
            }
        }
    }

    override fun onResume() {
        super.onResume()
        model.resumed()
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
        is Screen.Grid -> if (lib != null) GridScreen(model, lib, s.kind)
        is Screen.Details -> lib?.find(s.itemId)?.let { DetailsScreen(model, it) }
        is Screen.Player -> lib?.find(s.itemId)?.let { PlayerScreen(model, it, s) }
    }
}
