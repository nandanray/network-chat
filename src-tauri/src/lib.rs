mod network;

use network::{NetworkMessage, MessageType, NetworkState};
use std::sync::Arc;
use tauri::{AppHandle, Manager};
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

#[tauri::command]
async fn set_username(name: String, _app: AppHandle, state: tauri::State<'_, Arc<NetworkState>>) -> Result<(), String> {
    *state.username.lock().await = name;
    
    let state_clone = (*state).clone();
    tokio::spawn(async move {
        network::advertise_mdns(state_clone).await;
    });

    Ok(())
}

#[tauri::command]
async fn send_message(peer_id: String, text: String, state: tauri::State<'_, Arc<NetworkState>>) -> Result<(), String> {
    let peer = {
        let peers = state.peers.lock().await;
        peers.get(&peer_id).cloned()
    };

    if let Some(peer) = peer {
        let msg = NetworkMessage {
            sender: state.username.lock().await.clone(),
            msg_type: MessageType::Text(text),
        };
        send_to_peer(&peer.ip, peer.port, msg).await?;
        Ok(())
    } else {
        Err("Peer not found".into())
    }
}

#[tauri::command]
async fn send_file(peer_id: String, file_path: String, state: tauri::State<'_, Arc<NetworkState>>) -> Result<(), String> {
    let peer = {
        let peers = state.peers.lock().await;
        peers.get(&peer_id).cloned()
    };

    if let Some(peer) = peer {
        let metadata = tokio::fs::metadata(&file_path).await.map_err(|e| e.to_string())?;
        
        let mut actual_path = file_path.clone();
        let mut file_name = std::path::Path::new(&file_path).file_name().unwrap().to_string_lossy().to_string();
        
        if metadata.is_dir() {
            let zip_name = format!("{}.zip", file_name);
            let mut zip_path = dirs::download_dir().unwrap_or_else(|| std::path::PathBuf::from("/tmp"));
            zip_path.push(&zip_name);
            let zip_path_str = zip_path.to_string_lossy().to_string();
            
            let src_dir = file_path.clone();
            let dst_file = zip_path_str.clone();
            tokio::task::spawn_blocking(move || {
                crate::network::zip_dir(&src_dir, &dst_file).unwrap();
            }).await.map_err(|e| e.to_string())?;
            
            actual_path = zip_path_str;
            file_name = zip_name;
        }

        let actual_metadata = tokio::fs::metadata(&actual_path).await.map_err(|e| e.to_string())?;
        let file_size = actual_metadata.len();
        
        let offer_id = Uuid::new_v4().to_string();
        state.pending_offers.lock().await.insert(offer_id.clone(), actual_path);

        let msg = NetworkMessage {
            sender: state.username.lock().await.clone(),
            msg_type: MessageType::FileOffer { offer_id, file_name, file_size },
        };

        send_to_peer(&peer.ip, peer.port, msg).await?;
        Ok(())
    } else {
        Err("Peer not found".into())
    }
}

#[tauri::command]
async fn accept_file(peer_id: String, offer_id: String, save_path: String, state: tauri::State<'_, Arc<NetworkState>>) -> Result<(), String> {
    let peer = {
        let peers = state.peers.lock().await;
        peers.get(&peer_id).cloned()
    };

    if let Some(peer) = peer {
        state.accepted_offers.lock().await.insert(offer_id.clone(), save_path);

        let msg = NetworkMessage {
            sender: state.username.lock().await.clone(),
            msg_type: MessageType::FileAccept { offer_id },
        };

        send_to_peer(&peer.ip, peer.port, msg).await?;
        Ok(())
    } else {
        Err("Peer not found".into())
    }
}

async fn send_to_peer(ip: &str, port: u16, msg: NetworkMessage) -> Result<(), String> {
    let meta_json = serde_json::to_string(&msg).map_err(|e| e.to_string())?;
    let meta_len = meta_json.len() as u64;
    
    let mut socket = tokio::net::TcpStream::connect(format!("{}:{}", ip, port))
        .await
        .map_err(|e| e.to_string())?;
        
    socket.write_all(&meta_len.to_be_bytes()).await.map_err(|e| e.to_string())?;
    socket.write_all(meta_json.as_bytes()).await.map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
async fn get_peers(state: tauri::State<'_, Arc<NetworkState>>) -> Result<Vec<network::Peer>, String> {
    let peers: Vec<network::Peer> = state.peers.lock().await.values().cloned().collect();
    Ok(peers)
}

#[tauri::command]
async fn create_group(name: String, state: tauri::State<'_, Arc<NetworkState>>) -> Result<network::Group, String> {
    let group = network::Group {
        id: Uuid::new_v4().to_string(),
        name,
        members: vec![state.username.lock().await.clone()],
    };
    state.groups.lock().await.insert(group.id.clone(), group.clone());
    
    // Groups are private initially, no need to broadcast to everyone.
    Ok(group)
}

#[tauri::command]
async fn add_group_member(group_id: String, peer_id: String, state: tauri::State<'_, Arc<NetworkState>>) -> Result<(), String> {
    let mut group = {
        let groups = state.groups.lock().await;
        groups.get(&group_id).cloned().ok_or("Group not found")?
    };
    if !group.members.contains(&peer_id) {
        group.members.push(peer_id.clone());
        state.groups.lock().await.insert(group_id.clone(), group.clone());
        
        let msg = NetworkMessage {
            sender: state.username.lock().await.clone(),
            msg_type: MessageType::GroupUpdate { group: group.clone() },
        };
        
        let peers = state.peers.lock().await.clone();
        for member in group.members.iter() {
            if let Some(peer) = peers.get(member) {
                let _ = send_to_peer(&peer.ip, peer.port, msg.clone()).await;
            }
        }
        
        // Also explicitly send to the removed peer so they know they've been removed!
        if let Some(peer) = peers.get(&peer_id) {
            let _ = send_to_peer(&peer.ip, peer.port, msg.clone()).await;
        }
    }
    Ok(())
}

#[tauri::command]
async fn remove_group_member(group_id: String, peer_id: String, state: tauri::State<'_, Arc<NetworkState>>) -> Result<(), String> {
    let mut group = {
        let groups = state.groups.lock().await;
        groups.get(&group_id).cloned().ok_or("Group not found")?
    };
    if let Some(pos) = group.members.iter().position(|x| *x == peer_id) {
        group.members.remove(pos);
        state.groups.lock().await.insert(group_id.clone(), group.clone());
        
        let msg = NetworkMessage {
            sender: state.username.lock().await.clone(),
            msg_type: MessageType::GroupUpdate { group: group.clone() },
        };
        
        let peers = state.peers.lock().await.clone();
        for member in group.members.iter() {
            if let Some(peer) = peers.get(member) {
                let _ = send_to_peer(&peer.ip, peer.port, msg.clone()).await;
            }
        }
        if let Some(peer) = peers.get(&peer_id) {
            let _ = send_to_peer(&peer.ip, peer.port, msg.clone()).await;
        }
    }
    Ok(())
}

#[tauri::command]
async fn send_group_message(group_id: String, text: String, state: tauri::State<'_, Arc<NetworkState>>) -> Result<(), String> {
    let group = {
        let groups = state.groups.lock().await;
        groups.get(&group_id).cloned().ok_or("Group not found")?
    };
    let my_username = state.username.lock().await.clone();
    
    if !group.members.contains(&my_username) {
        return Err("You are no longer a member of this group".into());
    }
    
    let msg = NetworkMessage {
        sender: my_username.clone(),
        msg_type: MessageType::GroupMessage { group_id: group_id.clone(), text },
    };
    
    let peers = state.peers.lock().await.clone();
    for member in group.members.iter() {
        if member != &my_username {
            if let Some(peer) = peers.get(member) {
                let _ = send_to_peer(&peer.ip, peer.port, msg.clone()).await;
            }
        }
    }
    Ok(())
}

#[tauri::command]
async fn get_groups(state: tauri::State<'_, Arc<NetworkState>>) -> Result<Vec<network::Group>, String> {
    let my_username = state.username.lock().await.clone();
    let groups: Vec<network::Group> = state.groups.lock().await.values()
        .filter(|g| g.members.contains(&my_username))
        .cloned().collect();
    Ok(groups)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let app_handle = app.handle().clone();
            let state = Arc::new(NetworkState::new());
            app.manage(state.clone());
            
            let state_clone1 = state.clone();
            let app_handle_clone = app_handle.clone();
            
            tauri::async_runtime::spawn(async move {
                network::start_tcp_server(app_handle_clone, state_clone1).await;
            });
            
            tauri::async_runtime::spawn(async move {
                network::browse_mdns(app_handle, state).await;
            });
            
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![set_username, send_message, send_file, accept_file, get_peers, create_group, add_group_member, remove_group_member, send_group_message, get_groups])
        .build(tauri::generate_context!())
        .expect("error while running tauri application")
        .run(|app_handle, event| match event {
            tauri::RunEvent::Exit => {
                // By getting the state and explicitly dropping it, the mdns ServiceDaemon 
                // is dropped and sends a Goodbye packet instantly.
                let _state: tauri::State<'_, Arc<NetworkState>> = app_handle.state();
                // We don't strictly need to do anything since process exit will kill it,
                // but dropping it gracefully helps mdns-sd.
            }
            _ => {}
        });
}
