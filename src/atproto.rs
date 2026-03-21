use reqwest::header::HeaderValue;
use serde::Deserialize;

#[derive(Deserialize)]
struct DidDocument {
    #[serde(rename = "id")]
    pub _id: String,
    pub service: Vec<Service>,
}

#[derive(Deserialize)]
struct Service {
    #[serde(rename = "type")]
    pub service_type: String,
    #[serde(rename = "serviceEndpoint")]
    pub service_endpoint: String,
}

#[derive(Deserialize, Default)]
struct CollectionRecord {
    value: Option<TangledProfile>,
}

#[derive(Deserialize)]
struct TangledProfile {
    avatar: Option<Avatar>,
}

#[derive(Deserialize, Default)]
struct BskyProfile {
    avatar: Option<String>,
}

#[derive(Deserialize)]
struct Avatar {
    #[serde(rename = "ref")]
    r#ref: Option<Ref>,
}

#[derive(Deserialize)]
struct Ref {
    #[serde(rename = "$link")]
    link: Option<String>,
}

#[derive(Deserialize)]
struct DohResponse {
    #[serde(rename = "Answer")]
    answer: Option<Vec<DohAnswer>>,
}

#[derive(Deserialize)]
struct DohAnswer {
    data: String,
}

pub struct Proto {
    client: reqwest::blocking::Client,
}

impl Proto {
    pub fn new() -> Self {
        Proto {
            client: reqwest::blocking::Client::new(),
        }
    }

    pub fn resolve_did(&self, actor: &str) -> Option<(String, String)> {
        let did = if actor.starts_with("did:") {
            actor.to_string()
        } else {
            let url = format!("https://{}/.well-known/atproto-did", actor);
            let did = self
                .client
                .get(url)
                .send()
                .ok()
                .and_then(|res| res.text().ok())
                .or(self.resolve_atproto_record(actor));

            did?
        };

        println!("Resolving {:?}", did);

        let doc_url = if did.starts_with("did:web:") {
            format!(
                "https://{}/.well-known/did.json",
                did.strip_prefix("did:web:").expect("did web")
            )
        } else {
            format!("https://plc.directory/{}", did)
        };

        let doc = self
            .client
            .get(doc_url)
            .send()
            .ok()?
            .json::<DidDocument>()
            .ok()?;
        let pds = doc
            .service
            .into_iter()
            .find(|s| s.service_type == "AtprotoPersonalDataServer")?
            .service_endpoint;

        println!("PDS: {:?}", pds);

        Some((did, pds))
    }

    pub fn fetch_avatar(&self, did: &str, pds: &str) -> Option<(Vec<u8>, Option<HeaderValue>)> {
        let avatar_url = self
            .resolve_tangled_avatar(did, pds)
            .or_else(|| self.resolve_bsky_avatar(did))?;
        let mut res = self.client.get(avatar_url).send().ok()?;

        let content_type = res.headers_mut().remove("content-type");
        let avatar = res.bytes().ok()?.into();

        Some((avatar, content_type))
    }

    fn resolve_atproto_record(&self, actor: &str) -> Option<String> {
        let url = format!(
            "https://cloudflare-dns.com/dns-query?name=_atproto.{}&type=TXT",
            actor
        );

        let response = self
            .client
            .get(url)
            .header("accept", "application/dns-json")
            .send()
            .ok()?
            .json::<DohResponse>()
            .ok()?;

        response
            .answer?
            .into_iter()
            .next()
            .map(|a| a.data)
            .and_then(|s| Some(String::from(s.trim_matches('"').trim_start_matches("did="))))
    }

    fn resolve_tangled_avatar(&self, did: &str, pds: &str) -> Option<String> {
        let url = format!(
            "{}/xrpc/com.atproto.repo.getRecord?repo={}&collection=sh.tangled.actor.profile&rkey=self",
            pds.trim_end_matches('/'),
            did
        );

        let res: serde_json::Value = self.client.get(url).send().ok()?.json().ok()?;
        let collection_record = serde_json::from_value::<CollectionRecord>(res).unwrap_or_default();
        let cid = collection_record.value?.avatar?.r#ref?.link?;

        Some(format!(
            "{}/xrpc/com.atproto.sync.getBlob?did={}&cid={}",
            pds.trim_end_matches('/'),
            did,
            cid
        ))
    }

    fn resolve_bsky_avatar(&self, did: &str) -> Option<String> {
        let url = format!(
            "https://public.api.bsky.app/xrpc/app.bsky.actor.getProfile?actor={}",
            did
        );
        let res: serde_json::Value = self.client.get(url).send().ok()?.json().ok()?;
        let profile: BskyProfile = serde_json::from_value(res).unwrap_or_default();

        profile.avatar
    }
}
