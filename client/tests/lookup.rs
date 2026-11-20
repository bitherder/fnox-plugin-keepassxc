use keepassxc_client::{
    IdKeyPair, KxcClient, KxcError, Signal,
    crypto::{NONCE_LEN, SessionCrypto, b64, increment_nonce},
    transport::mock::MockTransport,
};
use kpxc_client::{Lookup, lookup};
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};

type RequestedUrl = Arc<Mutex<Option<String>>>;

enum Scenario {
    Found { password: &'static str },
    Ambiguous,
    NotFound,
    Locked,
    NotAssociated,
    FoundAfterSignal,
}

/// Plays KeePassXC on the other end of the NaCl box.
struct FakeKeePassXc {
    scenario: Scenario,
    crypto: SessionCrypto,
    requested_url: RequestedUrl,
}

impl FakeKeePassXc {
    fn handle(&mut self, frame: &Value) -> Vec<Value> {
        match frame["action"].as_str().unwrap() {
            "change-public-keys" => {
                self.crypto
                    .set_peer_public_key_b64(frame["publicKey"].as_str().unwrap())
                    .unwrap();
                vec![json!({
                    "action": "change-public-keys",
                    "publicKey": self.crypto.public_key_b64(),
                    "nonce": b64::enc(&reply_nonce(frame)),
                    "success": "true",
                    "version": "2.7.9",
                })]
            }
            "associate" => {
                let body = json!({"success": "true", "id": "fnox-test", "hash": "x"});
                vec![self.seal(frame, body)]
            }
            "get-logins" => self.get_logins(frame),
            other => panic!("fake KeePassXC: unexpected action {other}"),
        }
    }

    fn get_logins(&mut self, frame: &Value) -> Vec<Value> {
        let request = self.open(frame);
        *self.requested_url.lock().unwrap() = request["url"].as_str().map(str::to_owned);
        match self.scenario {
            Scenario::Found { password } => vec![self.found_reply(frame, password)],
            Scenario::Ambiguous => vec![self.seal(
                frame,
                json!({
                    "success": "true",
                    "count": 2,
                    "entries": [entry("u1", "one"), entry("u2", "two")],
                }),
            )],
            Scenario::NotFound => vec![self.seal(
                frame,
                json!({"success": "false", "error": "No logins found.", "errorCode": "15"}),
            )],
            Scenario::Locked => vec![json!({
                "action": "get-logins",
                "error": "Database locked",
                "errorCode": "1",
            })],
            Scenario::NotAssociated => vec![json!({
                "action": "get-logins",
                "error": "Association failed",
                "errorCode": "8",
            })],
            Scenario::FoundAfterSignal => vec![
                json!({"action": "database-locked"}),
                self.found_reply(frame, "s3cr3t"),
            ],
        }
    }

    fn found_reply(&self, request: &Value, password: &str) -> Value {
        self.seal(
            request,
            json!({
                "success": "true",
                "count": 1,
                "entries": [entry("u1", password)],
            }),
        )
    }

    fn open(&self, frame: &Value) -> Value {
        let ciphertext = b64::dec(frame["message"].as_str().unwrap()).unwrap();
        let plaintext = self
            .crypto
            .decrypt(&ciphertext, &request_nonce(frame))
            .unwrap();
        serde_json::from_slice(&plaintext).unwrap()
    }

    fn seal(&self, request: &Value, body: Value) -> Value {
        let nonce = reply_nonce(request);
        let ciphertext = self
            .crypto
            .encrypt(&serde_json::to_vec(&body).unwrap(), &nonce)
            .unwrap();
        json!({
            "action": request["action"],
            "message": b64::enc(&ciphertext),
            "nonce": b64::enc(&nonce),
            "clientID": request["clientID"],
        })
    }
}

fn entry(uuid: &str, password: &str) -> Value {
    json!({"uuid": uuid, "name": "fnox-test", "login": "user", "password": password})
}

fn request_nonce(frame: &Value) -> [u8; NONCE_LEN] {
    b64::dec(frame["nonce"].as_str().unwrap())
        .unwrap()
        .try_into()
        .unwrap()
}

fn reply_nonce(frame: &Value) -> [u8; NONCE_LEN] {
    increment_nonce(&request_nonce(frame))
}

async fn associated_client(scenario: Scenario) -> (KxcClient<MockTransport>, RequestedUrl) {
    let requested_url: RequestedUrl = Arc::new(Mutex::new(None));
    let mut fake = FakeKeePassXc {
        scenario,
        crypto: SessionCrypto::new(),
        requested_url: requested_url.clone(),
    };
    let transport = MockTransport::with_responder(move |frame| fake.handle(frame));
    let mut client = KxcClient::new(transport);
    client.change_public_keys().await.unwrap();
    client.associate(&IdKeyPair::generate()).await.unwrap();
    (client, requested_url)
}

#[tokio::test]
async fn finds_password_of_single_matching_entry() {
    let (mut client, requested_url) =
        associated_client(Scenario::Found { password: "s3cr3t" }).await;

    let result = lookup(&mut client, "github-token").await.unwrap();

    assert_eq!(result, Lookup::Found("s3cr3t".into()));
    assert_eq!(
        requested_url.lock().unwrap().as_deref(),
        Some("fnox://github-token")
    );
}

#[tokio::test]
async fn reports_not_found_when_no_entries_match() {
    let (mut client, _) = associated_client(Scenario::NotFound).await;

    let result = lookup(&mut client, "missing").await.unwrap();

    assert_eq!(result, Lookup::NotFound);
}

#[tokio::test]
async fn reports_ambiguous_when_several_entries_match() {
    let (mut client, _) = associated_client(Scenario::Ambiguous).await;

    let result = lookup(&mut client, "github-token").await.unwrap();

    assert_eq!(result, Lookup::Ambiguous(2));
}

#[tokio::test]
async fn reports_database_locked() {
    let (mut client, _) = associated_client(Scenario::Locked).await;

    let result = lookup(&mut client, "github-token").await;

    assert!(matches!(result, Err(KxcError::DatabaseLocked)));
}

#[tokio::test]
async fn reports_not_associated() {
    let (mut client, _) = associated_client(Scenario::NotAssociated).await;

    let result = lookup(&mut client, "github-token").await;

    assert!(matches!(result, Err(KxcError::NotAssociated)));
}

#[tokio::test]
async fn keeps_lookup_result_when_a_signal_arrives_mid_reply() {
    let (mut client, _) = associated_client(Scenario::FoundAfterSignal).await;

    let result = lookup(&mut client, "github-token").await.unwrap();

    assert_eq!(result, Lookup::Found("s3cr3t".into()));
    assert!(client.take_signals().contains(&Signal::DatabaseLocked));
}
