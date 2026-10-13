//! Posters and backdrops: downloaded and decoded on worker threads, uploaded
//! as textures on the UI thread.
use std::collections::HashMap;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

use eframe::egui::{self, ColorImage, TextureHandle, TextureOptions};

use crate::api::Client;

const WORKERS: usize = 6;
const MAX_SIDE: u32 = 1280;

enum State {
    Loading,
    Ready(TextureHandle),
    Failed,
}

type Done = (String, Option<ColorImage>);

pub struct Images {
    map: HashMap<String, State>,
    req: Sender<String>,
    done: Receiver<Done>,
    client: Arc<Mutex<Option<Client>>>,
}

impl Images {
    pub fn new(ctx: &egui::Context) -> Self {
        let (req, req_rx) = mpsc::channel::<String>();
        let (done_tx, done) = mpsc::channel::<Done>();
        let req_rx = Arc::new(Mutex::new(req_rx));
        let client: Arc<Mutex<Option<Client>>> = Arc::default();
        for _ in 0..WORKERS {
            let (rx, tx, client, ctx) =
                (req_rx.clone(), done_tx.clone(), client.clone(), ctx.clone());
            thread::spawn(move || loop {
                let Ok(url) = rx.lock().unwrap().recv() else {
                    return;
                };
                let c = client.lock().unwrap().clone();
                let img = c.and_then(|c| load(&c, &url));
                if tx.send((url, img)).is_err() {
                    return;
                }
                ctx.request_repaint();
            });
        }
        Self {
            map: HashMap::new(),
            req,
            done,
            client,
        }
    }

    /// Credentials for the server the images come from; clears the cache.
    pub fn set_client(&mut self, c: Option<Client>) {
        *self.client.lock().unwrap() = c;
        self.map.clear();
    }

    /// The texture if it's ready; starts loading it otherwise.
    pub fn get(&mut self, url: &str) -> Option<TextureHandle> {
        if !self.map.contains_key(url) {
            self.map.insert(url.to_owned(), State::Loading);
            let _ = self.req.send(url.to_owned());
        }
        match self.map.get(url) {
            Some(State::Ready(t)) => Some(t.clone()),
            _ => None,
        }
    }

    pub fn failed(&self, url: &str) -> bool {
        matches!(self.map.get(url), Some(State::Failed))
    }

    pub fn poll(&mut self, ctx: &egui::Context) {
        while let Ok((url, img)) = self.done.try_recv() {
            let Some(slot) = self.map.get_mut(&url) else {
                continue;
            }; // cache was cleared
            *slot = match img {
                Some(img) => State::Ready(ctx.load_texture(&url, img, TextureOptions::LINEAR)),
                None => State::Failed,
            };
        }
    }
}

fn load(c: &Client, url: &str) -> Option<ColorImage> {
    let bytes = c
        .fetch(url)
        .map_err(|e| eprintln!("image {url}: {e:#}"))
        .ok()?;
    let img = image::load_from_memory(&bytes)
        .map_err(|e| eprintln!("image {url}: {e}"))
        .ok()?;
    let img = if img.width() > MAX_SIDE || img.height() > MAX_SIDE {
        img.resize(MAX_SIDE, MAX_SIDE, image::imageops::FilterType::Triangle)
    } else {
        img
    };
    let rgba = img.to_rgba8();
    Some(ColorImage::from_rgba_unmultiplied(
        [rgba.width() as usize, rgba.height() as usize],
        &rgba,
    ))
}
