

let json_string = serde_json::to_string_pretty(&wv-hall_request).unwrap();

fs::write(temp.json, &json_string).expect("abc");