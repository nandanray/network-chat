use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use walkdir::WalkDir;
use zip::write::SimpleFileOptions;
use std::io::{Write, Read};
use std::fs::File;

pub const SERVICE_TYPE: &str = "_networkchat._tcp.local.";

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Peer {
    pub id: String, // usually username or hostname
    pub ip: String,
    pub port: u16,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Group {
    pub id: String,
    pub name: String,
    pub members: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum MessageType {
    Text(String),
    FileOffer { offer_id: String, file_name: String, file_size: u64 },
    FileAccept { offer_id: String },
    FileStart { offer_id: String, file_name: String, file_size: u64 },
    GroupMessage { group_id: String, text: String },
    GroupUpdate { group: Group },
    VoiceCallSignal { signal_type: String, data: String },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NetworkMessage {
    pub sender: String,
    pub msg_type: MessageType,
}

pub struct NetworkState {
    pub username: Mutex<String>,
    pub port: Mutex<u16>,
    pub peers: Mutex<HashMap<String, Peer>>,
    pub mdns: ServiceDaemon,
    pub pending_offers: Mutex<HashMap<String, String>>,
    pub accepted_offers: Mutex<HashMap<String, String>>,
    pub groups: Mutex<HashMap<String, Group>>,
}

impl NetworkState {
    pub fn new() -> Self {
        Self {
            username: Mutex::new(String::new()),
            port: Mutex::new(0),
            peers: Mutex::new(HashMap::new()),
            mdns: ServiceDaemon::new().expect("Failed to create mdns daemon"),
            pending_offers: Mutex::new(HashMap::new()),
            accepted_offers: Mutex::new(HashMap::new()),
            groups: Mutex::new(HashMap::new()),
        }
    }
}

pub async fn start_tcp_server(app: AppHandle, state: Arc<NetworkState>) {
    let listener = TcpListener::bind("0.0.0.0:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    *state.port.lock().await = port;

    println!("TCP Server listening on port {}", port);

    // We can't advertise mDNS yet until we know the username.
    // The UI will call `set_username` which will trigger the mDNS advertisement.

    loop {
        if let Ok((mut socket, addr)) = listener.accept().await {
            let app_clone = app.clone();
            let state_clone = state.clone();
            tokio::spawn(async move {
                handle_connection(&mut socket, addr, app_clone, state_clone).await;
            });
        }
    }
}

async fn handle_connection(socket: &mut TcpStream, _addr: SocketAddr, app: AppHandle, state: Arc<NetworkState>) {
    let mut len_buf = [0u8; 8];
    if let Err(_) = socket.read_exact(&mut len_buf).await { return; }
    let meta_len = u64::from_be_bytes(len_buf) as usize;

    let mut meta_buf = vec![0u8; meta_len];
    if let Err(_) = socket.read_exact(&mut meta_buf).await { return; }

    let msg: NetworkMessage = match serde_json::from_slice(&meta_buf) {
        Ok(m) => m,
        Err(_) => return,
    };

    match msg.msg_type {
        MessageType::Text(text) => {
            let _ = app.emit("chat-message", serde_json::json!({ "sender": msg.sender, "text": text }));
        }
        MessageType::GroupUpdate { group } => {
            let my_username = state.username.lock().await.clone();
            let mut groups = state.groups.lock().await;
            groups.insert(group.id.clone(), group.clone());
            
            let filtered_groups: Vec<Group> = groups.values()
                .filter(|g| g.members.contains(&my_username))
                .cloned().collect();
                
            let _ = app.emit("group-update", filtered_groups);
        }
        MessageType::GroupMessage { group_id, text } => {
            let _ = app.emit("group-message", serde_json::json!({
                "sender": msg.sender,
                "group_id": group_id,
                "text": text,
            }));
        }
        MessageType::FileOffer { offer_id, file_name, file_size } => {
            let _ = app.emit("file-offer", serde_json::json!({
                "sender": msg.sender, "offer_id": offer_id, "file_name": file_name, "file_size": file_size,
            }));
        }
        MessageType::FileAccept { offer_id } => {
            let path = state.pending_offers.lock().await.remove(&offer_id);
            if let Some(file_path) = path {
                let peers = state.peers.lock().await.clone();
                let peer = peers.get(&msg.sender).cloned();
                let my_username = state.username.lock().await.clone();
                
                if let Some(peer) = peer {
                    tokio::spawn(async move {
                        if let Ok(mut file) = tokio::fs::File::open(&file_path).await {
                            let metadata = file.metadata().await.unwrap();
                            let file_size = metadata.len();
                            let file_name = std::path::Path::new(&file_path).file_name().unwrap().to_string_lossy().to_string();
                            
                            let target_addr = format!("{}:{}", peer.ip, peer.port);
                            if let Ok(mut socket) = TcpStream::connect(target_addr).await {
                                let outgoing_msg = NetworkMessage { sender: my_username, msg_type: MessageType::FileStart { offer_id: offer_id.clone(), file_name, file_size } };
                                let meta_json = serde_json::to_string(&outgoing_msg).unwrap();
                                let meta_len = meta_json.len() as u64;
                                let _ = socket.write_all(&meta_len.to_be_bytes()).await;
                                let _ = socket.write_all(meta_json.as_bytes()).await;
                                
                                let mut buf = [0u8; 8192];
                                while let Ok(n) = file.read(&mut buf).await {
                                    if n == 0 { break; }
                                    let _ = socket.write_all(&buf[..n]).await;
                                }
                            }
                        }
                    });
                }
            }
        }
        MessageType::FileStart { offer_id, file_name, file_size } => {
            let save_path = state.accepted_offers.lock().await.remove(&offer_id);
            if let Some(save_path) = save_path {
                let _ = app.emit("file-transfer-start", serde_json::json!({ "sender": msg.sender, "file_name": file_name, "file_size": file_size }));

                if let Ok(mut file) = tokio::fs::File::create(&save_path).await {
                    let mut remaining = file_size;
                    let mut buf = [0u8; 8192];
                    while remaining > 0 {
                        let to_read = std::cmp::min(remaining, buf.len() as u64) as usize;
                        match socket.read_exact(&mut buf[..to_read]).await {
                            Ok(_) => { if let Err(_) = file.write_all(&buf[..to_read]).await { break; } remaining -= to_read as u64; }
                            Err(_) => break,
                        }
                    }
                }
                let _ = app.emit("file-transfer-complete", serde_json::json!({ "sender": msg.sender, "file_name": file_name, "path": save_path }));
            }
        }
        MessageType::VoiceCallSignal { signal_type, data } => {
            let _ = app.emit("voice-call-signal", serde_json::json!({ "sender": msg.sender, "signal_type": signal_type, "data": data }));
        }
    }
}

pub async fn browse_mdns(app: AppHandle, state: Arc<NetworkState>) {
    let receiver = state.mdns.browse(SERVICE_TYPE).expect("Failed to browse");

    while let Ok(event) = receiver.recv_async().await {
        match event {
            ServiceEvent::ServiceResolved(info) => {
                let id = info.get_fullname().replace(SERVICE_TYPE, "").trim_end_matches('.').to_string();
                let ip = info.get_addresses().iter().next().map(|ip| ip.to_string()).unwrap_or_default();
                let port = info.get_port();

                // Do not add ourselves
                let current_username = state.username.lock().await.clone();
                if id != current_username && !ip.is_empty() {
                    let peer = Peer { id: id.clone(), ip, port };
                    state.peers.lock().await.insert(id.clone(), peer.clone());
                    
                    // Notify UI
                    let peers: Vec<Peer> = state.peers.lock().await.values().cloned().collect();
                    let _ = app.emit("peer-update", peers);
                }
            }
            ServiceEvent::ServiceRemoved(_type_name, fullname) => {
                let id = fullname.replace(SERVICE_TYPE, "").trim_end_matches('.').to_string();
                state.peers.lock().await.remove(&id);
                
                let peers: Vec<Peer> = state.peers.lock().await.values().cloned().collect();
                let _ = app.emit("peer-update", peers);
            }
            _ => {}
        }
    }
}

pub async fn advertise_mdns(state: Arc<NetworkState>) {
    let username = state.username.lock().await.clone();
    let port = *state.port.lock().await;

    if username.is_empty() || port == 0 {
        return;
    }

    let ip = local_ip_address::local_ip().unwrap().to_string();
    
    let my_info = ServiceInfo::new(
        SERVICE_TYPE,
        &username,
        &format!("{}.local.", username),
        &ip,
        port,
        None,
    ).unwrap();

    state.mdns.register(my_info).expect("Failed to register mDNS service");
}

pub fn zip_dir(src_dir: &str, dst_file: &str) -> zip::result::ZipResult<()> {
    let file = File::create(dst_file)?;
    let walkdir = WalkDir::new(src_dir);
    let it = walkdir.into_iter();
    let mut zip = zip::ZipWriter::new(file);
    let options = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o755);

    for entry in it.filter_map(|e| e.ok()) {
        let path = entry.path();
        let name = path.strip_prefix(std::path::Path::new(src_dir)).unwrap();
        if path.is_file() {
            zip.start_file(name.to_string_lossy().to_string(), options)?;
            let mut f = File::open(path)?;
            let mut buffer = Vec::new();
            f.read_to_end(&mut buffer)?;
            zip.write_all(&buffer)?;
        } else if !name.as_os_str().is_empty() {
            zip.add_directory(name.to_string_lossy().to_string(), options)?;
        }
    }
    zip.finish()?;
    Ok(())
}
