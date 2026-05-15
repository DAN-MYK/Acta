use serde_json;

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
enum DocumentDirection {
    Outgoing,
    Incoming,
}

fn main() {
    let outgoing = DocumentDirection::Outgoing;
    let json = serde_json::to_string(&outgoing).unwrap();
    println!("Serialized Outgoing: {}", json);
    
    let incoming = DocumentDirection::Incoming;
    let json = serde_json::to_string(&incoming).unwrap();
    println!("Serialized Incoming: {}", json);
    
    let deserialized: DocumentDirection = serde_json::from_str("\"outgoing\"").unwrap();
    println!("Deserialized outgoing successfully");
    
    let deserialized: DocumentDirection = serde_json::from_str("\"incoming\"").unwrap();
    println!("Deserialized incoming successfully");
}
