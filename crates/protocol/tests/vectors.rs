//! Conformance with `contracts/protocol/vectors.json` (the same vectors the TypeScript decoder passes).

use kurzhaar_protocol::decode;
use serde_json::Value;

#[test]
fn passes_shared_vectors() {
    let raw = include_str!("../../../contracts/protocol/vectors.json");
    let doc: Value = serde_json::from_str(raw).unwrap();
    let vectors = doc["vectors"].as_array().unwrap();
    assert!(!vectors.is_empty());

    for v in vectors {
        let name = v["name"].as_str().unwrap();
        let body = v["body"].as_str().unwrap();
        match v.get("error") {
            Some(err) => {
                let e = decode(body).expect_err(name);
                assert!(e.to_string().contains(err.as_str().unwrap()), "{name}: {e}");
            }
            None => {
                let got =
                    serde_json::to_value(decode(body).unwrap_or_else(|e| panic!("{name}: {e}")))
                        .unwrap();
                assert_eq!(got, v["expected"], "{name}");
            }
        }
    }
}
