mod atproto;
mod cache;
mod decode;
mod image;
use sha2::{Digest, Sha256};
use std::{io, str::FromStr, thread, time};
use tiny_http::{Header, Request, Response};

use crate::cache::Cache;

const CONTENT_TYPE_SVG: &str = "Content-Type: image/svg+xml";
const CACHE_CONTROL: &str = "Cache-Control: public, max-age=43200";

struct Server {
    http: tiny_http::Server,
    shared_secret: String,
    cache: Cache,
    atproto: atproto::Proto,
}

impl Server {
    pub fn boot(&self) {
        println!("Running");
        thread::scope(|s| {
            for request in self.http.incoming_requests() {
                s.spawn(|| {
                    self.handle(request);
                });
            }
        })
    }

    fn handle(&self, req: Request) {
        let url = req.url().to_string();
        let (path, params) = url.split_once("?").unwrap_or_else(|| (&url, ""));
        let response: Response<_> = match path {
            "/" => Response::from_string("This is a private avatar backend for tangled."),
            _ => self.avatar(path, params),
        };

        let _ = req.respond(response);
    }

    fn avatar(&self, path: &str, params: &str) -> Response<io::Cursor<Vec<u8>>> {
        let cache_key = hex::encode(Sha256::digest(path));
        if let Some(bin) = self.cache.get(&cache_key) {
            let content_type = self
                .cache
                .get(&format!("{}.ct", &cache_key))
                .and_then(|b| String::from_utf8(b).ok())
                .unwrap_or(String::from("image/jpeg"));
            let (bin_start, _) = bin.split_at(8);
            let header = Header::from_str(
                str::from_utf8(bin_start)
                    .is_ok_and(|s| s.starts_with("<s"))
                    .then_some(CONTENT_TYPE_SVG)
                    .unwrap_or(&format!("Content-Type: {}", content_type)),
            )
            .expect("cache content-type");

            return Response::from_data(bin)
                .with_header(header)
                .with_header(Header::from_str(CACHE_CONTROL).expect("cache cache-control"));
        };

        let Some((secret, actor)) = path
            .trim_start_matches("/")
            .trim_end_matches("/")
            .split_once("/")
        else {
            return Response::from_string("Bad URL").with_status_code(400);
        };

        if !decode::is_valid(&self.shared_secret, &actor, &secret) {
            return Response::from_string("Invalid signature").with_status_code(403);
        };

        let Some((avatar, content_type)) = self
            .atproto
            .resolve_did(actor)
            .and_then(|(did, pds)| self.atproto.fetch_avatar(&did, &pds))
        else {
            let svg = image::fallback(&cache_key, params.find("size=tiny").is_some());
            let response = Response::from_string(&svg);
            self.cache
                .put(&format!("{}.ct", &cache_key), "image/xml+svg".as_bytes());
            self.cache.put(&cache_key, svg.as_bytes());

            return response
                .with_header(Header::from_str(CONTENT_TYPE_SVG).expect("content-type header"))
                .with_header(Header::from_str(CACHE_CONTROL).expect("cache-control header"));
        };

        let content_type = content_type
            .and_then(|v| v.to_str().ok().map(String::from))
            .unwrap_or_else(|| String::from("image/jpeg"));
        self.cache
            .put(&format!("{}.ct", &cache_key), content_type.as_bytes());
        self.cache.put(&cache_key, &avatar);

        Response::from_data(avatar)
            .with_header(
                Header::from_str(&format!("Content-Type: {}", content_type))
                    .expect("content-type avatar"),
            )
            .with_header(Header::from_str(CACHE_CONTROL).expect("cache-control avatar"))
    }
}

fn main() {
    let port = std::env::var("PORT")
        .ok()
        .and_then(|v| v.parse::<u16>().ok())
        .unwrap_or(3000);
    let secret = std::env::var("AVATAR_SHARED_SECRET").expect("AVATAR_SHARED_SECRET is required");

    let cache =
        cache::Cache::new(std::env::var("CACHE_DIR").unwrap_or_else(|_| "/var/lib/avatars".into()));

    let subprocess_cache = cache.clone();
    thread::spawn(move || {
        loop {
            subprocess_cache.cleanup();
            thread::sleep(time::Duration::from_secs(3600));
        }
    });

    let http = tiny_http::Server::http(format!("0.0.0.0:{port}")).expect("failed to start server");
    let server = Server {
        http,
        shared_secret: secret,
        cache,
        atproto: atproto::Proto::new(),
    };

    server.boot();
}
