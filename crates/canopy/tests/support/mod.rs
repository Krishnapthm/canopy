use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::time::{Duration, Instant};

/// Replies can be interleaved with tree batches; only the matching request completes a call.
pub fn socket(home: &std::path::Path) -> impl FnMut(Value) -> Value {
    let stream = UnixStream::connect(home.join("sock")).unwrap();
    stream.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
    stream.set_write_timeout(Some(Duration::from_secs(2))).unwrap();
    let mut reader = BufReader::new(stream);
    let mut next = 0u64;
    move |mut op| {
        next += 1;
        op["id"] = json!(next);
        writeln!(reader.get_mut(), "{op}").unwrap();
        loop {
            let mut line = String::new();
            assert!(reader.read_line(&mut line).unwrap() > 0, "daemon closed socket before reply {next}");
            let msg: Value = serde_json::from_str(&line).unwrap();
            if msg["id"] == next {
                assert!(msg["t"] == "res" || msg["t"] == "err", "unexpected reply: {msg}");
                return msg;
            }
            assert_eq!(msg["t"], "batch", "unexpected socket message: {msg}");
        }
    }
}

pub fn spawn(home: &std::path::Path, harness: &str, prompt: &str) -> String {
    let r = socket(home)(json!({"op":"spawn","harness":harness,"prompt":prompt,"cwd":home,"policy":"ask"}));
    assert_eq!(r["t"], "res", "{r}");
    r["result"]["session"].as_str().unwrap().to_owned()
}

pub fn settle(home: &std::path::Path, id: &str) {
    let mut call = socket(home);
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        let r = call(json!({"op":"sub_tree"}));
        let node =
            r["result"]["nodes"].as_array().unwrap().iter().find(|n| n["id"] == id || n["id"].as_str().unwrap().ends_with(id)).unwrap();
        let status: protocol::Status = serde_json::from_value(node["status"].clone()).unwrap();
        if status.is_settled() {
            return;
        }
        assert!(Instant::now() < deadline, "session did not settle: {node}");
        std::thread::sleep(Duration::from_millis(10));
    }
}

pub fn transcript(home: &std::path::Path, id: &str) -> String {
    let r = socket(home)(json!({"op":"sub_session","session":id}));
    assert_eq!(r["t"], "res", "{r}");
    r["result"]["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| {
            let kind = i["kind"].as_str().unwrap();
            let label = kind;
            if kind == "permission" && i["state"] == "streaming" {
                format!("permission: {} [request {}]", i["body"]["title"].as_str().unwrap_or(""), i["id"].as_str().unwrap())
            } else {
                format!("{label}: {}", i["body"]["text"].as_str().map(str::to_owned).unwrap_or_else(|| i["body"].to_string()))
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}
