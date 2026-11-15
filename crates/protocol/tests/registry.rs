//! Every key in `contracts/protocol/fields.json` is decoded (REQ-PROTOCOL-J5CJ).

use kurzhaar_protocol::decode;
use serde_json::Value;

/// A minimal batch in which the field under test is present once.
fn wire(f: &Value) -> String {
    let key = f["key"].as_str().unwrap();
    let example = f["example"].as_str().unwrap();
    let key = if f["family"].as_bool().unwrap_or(false) {
        format!("p.{}", f["exampleName"].as_str().unwrap())
    } else {
        key.to_string()
    };
    if f["scope"] == "header" {
        return if key == "v" {
            "v::1\nts::1".to_string()
        } else {
            format!("v::1|{key}::{example}\nts::1")
        };
    }
    match key.as_str() {
        "ts" => format!("v::1\nts::{example}"),
        "d" => format!("v::1\nts::1\nd::{example}"),
        _ => {
            let y = if key == "y" {
                example.to_string()
            } else {
                f["types"][0].as_u64().unwrap_or(0).to_string()
            };
            format!("v::1\nts::1|y::{y}|{key}::{example}")
        }
    }
}

#[test]
fn decodes_every_registered_key() {
    let doc: Value =
        serde_json::from_str(include_str!("../../../contracts/protocol/fields.json")).unwrap();
    let fields = doc["fields"].as_array().unwrap();
    assert!(!fields.is_empty());

    for f in fields {
        let key = f["key"].as_str().unwrap();
        let example = f["example"].as_str().unwrap();
        let batch = decode(&wire(f)).unwrap_or_else(|e| panic!("{key}: {e}"));
        let out = serde_json::to_value(&batch).unwrap();

        let Some(decoded) = f["decoded"].as_str() else {
            if key == "d" {
                let want = 1.0 + example.parse::<f64>().unwrap();
                assert_eq!(out["events"][1]["ts"].as_f64().unwrap(), want, "d");
            }
            continue;
        };
        let got = if f["scope"] == "header" {
            &out[decoded]
        } else {
            &out["events"][0][decoded]
        };
        if f["family"].as_bool().unwrap_or(false) {
            assert_eq!(
                got[f["exampleName"].as_str().unwrap()],
                Value::from(example),
                "{key}"
            );
        } else if f["type"] == "string" {
            assert_eq!(got.as_str(), Some(example), "{key}");
        } else {
            assert_eq!(got.as_f64(), Some(example.parse::<f64>().unwrap()), "{key}");
        }
    }
}
