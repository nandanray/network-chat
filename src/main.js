import { open, save } from '@tauri-apps/plugin-dialog';

const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

let currentPeerId = null;
let isGroupChat = false;
let peers = [];
let groups = [];
let messagesMap = {}; // { peerId: [{ sender, text, isFile }] }
let unreadCounts = {}; // { peerId: number }
let myUsername = '';

// Prevent right-click context menu
document.addEventListener('contextmenu', e => e.preventDefault());

const setupScreen = document.getElementById('setup-screen');
const chatScreen = document.getElementById('chat-screen');
const setupForm = document.getElementById('setup-form');
const usernameInput = document.getElementById('username-input');

const peerList = document.getElementById('peer-list');
const currentPeerName = document.getElementById('current-peer-name');
const messagesContainer = document.getElementById('messages-container');
const chatForm = document.getElementById('chat-form');
const messageInput = document.getElementById('message-input');
const attachBtn = document.getElementById('attach-btn');
const attachFolderBtn = document.getElementById('attach-folder-btn');
const groupList = document.getElementById('group-list');
const createGroupBtn = document.getElementById('create-group-btn');
const groupActions = document.getElementById('group-actions');
const addMemberBtn = document.getElementById('add-member-btn');
const removeMemberBtn = document.getElementById('remove-member-btn');

const createGroupModal = document.getElementById('create-group-modal');
const newGroupName = document.getElementById('new-group-name');
const cancelCreateGroupBtn = document.getElementById('cancel-create-group-btn');
const confirmCreateGroupBtn = document.getElementById('confirm-create-group-btn');

const addMemberModal = document.getElementById('add-member-modal');
const availablePeersList = document.getElementById('available-peers-list');
const cancelAddMemberBtn = document.getElementById('cancel-add-member-btn');
const confirmAddMemberBtn = document.getElementById('confirm-add-member-btn');

const sidebar = document.getElementById('sidebar');
const sidebarToggleBtn = document.getElementById('sidebar-toggle-btn');
const closeSidebarBtn = document.getElementById('close-sidebar-btn');
const mobileBackdrop = document.getElementById('mobile-backdrop');

function closeSidebar() {
  if (window.innerWidth <= 768) {
    sidebar.classList.remove('open');
    mobileBackdrop.classList.add('hidden');
  }
}

sidebarToggleBtn.addEventListener('click', () => {
  if (window.innerWidth <= 768) {
    sidebar.classList.add('open');
    mobileBackdrop.classList.remove('hidden');
  } else {
    sidebar.classList.toggle('collapsed');
  }
});

closeSidebarBtn.addEventListener('click', closeSidebar);
mobileBackdrop.addEventListener('click', closeSidebar);

window.addEventListener('resize', () => {
  if (window.innerWidth > 768) {
    sidebar.classList.remove('open');
    mobileBackdrop.classList.add('hidden');
  } else {
    sidebar.classList.remove('collapsed');
  }
});

setupForm.addEventListener('submit', async (e) => {
  e.preventDefault();
  myUsername = usernameInput.value.trim();
  if (myUsername) {
    try {
      await invoke('set_username', { name: myUsername });
      setupScreen.classList.remove('active');
      chatScreen.classList.add('active');
      fetchPeers();
    } catch (err) {
      alert('Error setting username: ' + err);
    }
  }
});

function renderPeers() {
  peerList.innerHTML = '';
  peers.forEach(peer => {
    const li = document.createElement('li');
    li.className = `peer-item ${currentPeerId === peer.id ? 'active' : ''}`;
    
    const initial = peer.id.charAt(0).toUpperCase();
    const unread = unreadCounts[peer.id] || 0;
    const badgeHtml = unread > 0 ? `<div class="unread-badge">${unread}</div>` : '';

    li.innerHTML = `
      <div class="peer-avatar">${initial}</div>
      <div class="peer-info">
        <div class="peer-name">${peer.id}</div>
        ${badgeHtml}
      </div>
    `;
    
    li.addEventListener('click', () => {
      currentPeerId = peer.id;
      isGroupChat = false;
      unreadCounts[peer.id] = 0; // Clear unread
      currentPeerName.textContent = `Chatting with ${peer.id}`;
      chatForm.classList.remove('hidden');
      groupActions.classList.add('hidden');
      renderPeers();
      if(groups.length > 0) renderGroups();
      renderMessages();
      closeSidebar();
    });
    
    peerList.appendChild(li);
  });
}

function renderMessages() {
  messagesContainer.innerHTML = '';
  if (!currentPeerId || !messagesMap[currentPeerId]) {
    messagesContainer.innerHTML = '<div class="empty-state">No messages yet.</div>';
    return;
  }
  
  const msgs = messagesMap[currentPeerId];
  if (msgs.length === 0) {
    messagesContainer.innerHTML = '<div class="empty-state">No messages yet.</div>';
    return;
  }

  msgs.forEach(msg => {
    const div = document.createElement('div');
    div.className = `message ${msg.sender === myUsername ? 'mine' : 'peer'}`;
    
    const senderHtml = `<div class="message-sender">${msg.sender}</div>`;

    if (msg.isOffer) {
      div.innerHTML = `
        ${senderHtml}
        <div>Incoming File Offer</div>
        <div class="file-message">
          <svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" stroke-width="2"><path d="M13 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9z"></path><polyline points="13 2 13 9 20 9"></polyline></svg>
          <span>${msg.fileName} (${(msg.fileSize / 1024 / 1024).toFixed(2)} MB)</span>
          <button class="download-btn" data-offer="${msg.offerId}" data-name="${msg.fileName}" style="margin-left:auto; padding:0.25rem 0.5rem; font-size:0.8rem;">Download</button>
        </div>
      `;
    } else if (msg.isFile) {
      div.innerHTML = `
        ${senderHtml}
        <div>File Transferred</div>
        <div class="file-message">
          <svg viewBox="0 0 24 24" width="20" height="20" fill="none" stroke="currentColor" stroke-width="2"><path d="M13 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9z"></path><polyline points="13 2 13 9 20 9"></polyline></svg>
          <span>${msg.fileName}</span>
        </div>
      `;
    } else {
      div.innerHTML = `
        ${senderHtml}
        <div class="message-content"></div>
      `;
      div.querySelector('.message-content').textContent = msg.text;
    }
    
    messagesContainer.appendChild(div);
  });
  
  messagesContainer.scrollTop = messagesContainer.scrollHeight;

  document.querySelectorAll('.download-btn').forEach(btn => {
    btn.addEventListener('click', async (e) => {
      const offerId = e.target.dataset.offer;
      const defaultName = e.target.dataset.name;
      try {
        const savePath = await save({ defaultPath: defaultName, title: 'Save File' });
        if (savePath) {
          e.target.textContent = 'Downloading...';
          e.target.disabled = true;
          await invoke('accept_file', { peerId: currentPeerId, offerId, savePath });
        }
      } catch (err) {
        alert("Failed to save: " + err);
      }
    });
  });
}

chatForm.addEventListener('submit', async (e) => {
  e.preventDefault();
  if (!currentPeerId) return;
  
  const text = messageInput.value.trim();
  if (text) {
    try {
      if (isGroupChat) {
        await invoke('send_group_message', { groupId: currentPeerId, text });
      } else {
        await invoke('send_message', { peerId: currentPeerId, text });
      }
      if (!messagesMap[currentPeerId]) messagesMap[currentPeerId] = [];
      messagesMap[currentPeerId].push({ sender: myUsername, text });
      messageInput.value = '';
      renderMessages();
    } catch (err) {
      alert('Failed to send: ' + err);
    }
  }
});

attachBtn.addEventListener('click', async () => {
  if (!currentPeerId) return;
  try {
    const path = await open({
      multiple: false,
      title: 'Select a file to send'
    });
    if (path) {
      await invoke('send_file', { peerId: currentPeerId, filePath: path });
      const fileName = path.split('/').pop().split('\\').pop();
      if (!messagesMap[currentPeerId]) messagesMap[currentPeerId] = [];
      messagesMap[currentPeerId].push({ sender: myUsername, isFile: true, fileName, path });
      renderMessages();
    }
  } catch (err) {
    alert('Failed to send file: ' + err);
  }
});

attachFolderBtn.addEventListener('click', async () => {
  if (!currentPeerId) return;
  try {
    const path = await open({
      directory: true,
      multiple: false,
      title: 'Select a folder to send'
    });
    if (path) {
      attachFolderBtn.style.opacity = '0.5';
      await invoke('send_file', { peerId: currentPeerId, filePath: path });
      const folderName = path.split('/').pop().split('\\').pop();
      if (!messagesMap[currentPeerId]) messagesMap[currentPeerId] = [];
      messagesMap[currentPeerId].push({ sender: myUsername, isFile: true, fileName: folderName + '.zip', path });
      renderMessages();
      attachFolderBtn.style.opacity = '1';
    }
  } catch (err) {
    alert('Failed to send folder: ' + err);
    attachFolderBtn.style.opacity = '1';
  }
});

// Listeners
listen('file-offer', (event) => {
  const { sender, offer_id, file_name, file_size } = event.payload;
  if (!messagesMap[sender]) messagesMap[sender] = [];
  messagesMap[sender].push({ sender, isOffer: true, offerId: offer_id, fileName: file_name, fileSize: file_size });
  if (currentPeerId === sender) {
    renderMessages();
  } else {
    unreadCounts[sender] = (unreadCounts[sender] || 0) + 1;
    renderPeers();
  }
});
listen('peer-update', (event) => {
  peers = event.payload;
  renderPeers();
});

listen('chat-message', (event) => {
  const { sender, text } = event.payload;
  if (!messagesMap[sender]) messagesMap[sender] = [];
  messagesMap[sender].push({ sender, text });
  if (currentPeerId === sender) {
    renderMessages();
  } else {
    unreadCounts[sender] = (unreadCounts[sender] || 0) + 1;
    renderPeers();
  }
});

listen('file-transfer-complete', (event) => {
  const { sender, file_name, path } = event.payload;
  if (!messagesMap[sender]) messagesMap[sender] = [];
  messagesMap[sender].push({ sender, isFile: true, fileName: file_name, path });
  if (currentPeerId === sender) {
    renderMessages();
  } else {
    unreadCounts[sender] = (unreadCounts[sender] || 0) + 1;
    renderPeers();
  }
});

async function fetchPeers() {
  if (!myUsername) return;
  try {
    const newPeers = await invoke('get_peers');
    if (JSON.stringify(newPeers) !== JSON.stringify(peers)) {
      peers = newPeers;
      renderPeers();
    }
  } catch (err) {
    console.error("Failed to fetch peers:", err);
  }
}

async function fetchGroups() {
  if (!myUsername) return;
  try {
    const newGroups = await invoke('get_groups');
    if (JSON.stringify(newGroups) !== JSON.stringify(groups)) {
      groups = newGroups;
      renderGroups();
    }
  } catch (err) {
    console.error("Failed to fetch groups:", err);
  }
}

function renderGroups() {
  groupList.innerHTML = '';
  groups.forEach(group => {
    const li = document.createElement('li');
    li.className = `peer-item ${currentPeerId === group.id ? 'active' : ''}`;
    
    const initial = group.name.charAt(0).toUpperCase();
    const unread = unreadCounts[group.id] || 0;
    const badgeHtml = unread > 0 ? `<div class="unread-badge">${unread}</div>` : '';

    li.innerHTML = `
      <div class="peer-avatar" style="background: rgba(255, 200, 100, 0.2); color: #ffd166;">${initial}</div>
      <div class="peer-info">
        <div class="peer-name">${group.name}</div>
        <div style="font-size: 0.7rem; opacity: 0.7;">${group.members.length} members</div>
        ${badgeHtml}
      </div>
    `;
    
    li.addEventListener('click', () => {
      currentPeerId = group.id;
      isGroupChat = true;
      unreadCounts[group.id] = 0; // Clear unread
      currentPeerName.textContent = `${group.name} (Group)`;
      chatForm.classList.remove('hidden');
      groupActions.classList.remove('hidden');
      renderPeers();
      renderGroups();
      renderMessages();
      closeSidebar();
    });
    
    groupList.appendChild(li);
  });
}

createGroupBtn.addEventListener('click', () => {
  newGroupName.value = '';
  createGroupModal.classList.remove('hidden');
});

cancelCreateGroupBtn.addEventListener('click', () => {
  createGroupModal.classList.add('hidden');
});

confirmCreateGroupBtn.addEventListener('click', async () => {
  const name = newGroupName.value.trim();
  if (name) {
    try {
      await invoke('create_group', { name });
      fetchGroups();
      createGroupModal.classList.add('hidden');
    } catch (err) {
      alert("Failed to create group: " + err);
    }
  }
});

addMemberBtn.addEventListener('click', () => {
  if (!isGroupChat) return;
  const group = groups.find(g => g.id === currentPeerId);
  if (!group) return;
  
  availablePeersList.innerHTML = '';
  const availablePeers = peers.filter(p => !group.members.includes(p.id) && p.id !== myUsername);
  
  if(availablePeers.length === 0) {
    availablePeersList.innerHTML = '<div style="opacity:0.5; padding: 1rem; text-align:center;">No available peers</div>';
  } else {
    availablePeers.forEach(peer => {
      const label = document.createElement('label');
      label.style.display = 'flex';
      label.style.alignItems = 'center';
      label.style.marginBottom = '0.5rem';
      label.style.cursor = 'pointer';
      
      const input = document.createElement('input');
      input.type = 'checkbox';
      input.value = peer.id;
      input.style.marginRight = '0.5rem';
      
      label.appendChild(input);
      label.appendChild(document.createTextNode(peer.id));
      availablePeersList.appendChild(label);
    });
  }
  
  addMemberModal.classList.remove('hidden');
});

cancelAddMemberBtn.addEventListener('click', () => {
  addMemberModal.classList.add('hidden');
});

confirmAddMemberBtn.addEventListener('click', async () => {
  const checkboxes = availablePeersList.querySelectorAll('input[type="checkbox"]:checked');
  for(let box of checkboxes) {
    try {
      await invoke('add_group_member', { groupId: currentPeerId, peerId: box.value });
    } catch (err) {
      console.error(err);
    }
  }
  addMemberModal.classList.add('hidden');
});

removeMemberBtn.addEventListener('click', async () => {
  if (!isGroupChat) return;
  const username = prompt("Enter the exact username to remove from this group:");
  if (username) {
    try {
      await invoke('remove_group_member', { groupId: currentPeerId, peerId: username });
    } catch (err) {
      alert("Failed to remove member: " + err);
    }
  }
});

listen('group-message', (event) => {
  const { sender, group_id, text } = event.payload;
  if (!messagesMap[group_id]) messagesMap[group_id] = [];
  messagesMap[group_id].push({ sender, text });
  if (currentPeerId === group_id) {
    renderMessages();
  } else {
    unreadCounts[group_id] = (unreadCounts[group_id] || 0) + 1;
    renderGroups();
  }
});

listen('group-update', (event) => {
  groups = event.payload;
  if (isGroupChat && currentPeerId) {
    const activeGroup = groups.find(g => g.id === currentPeerId);
    if (!activeGroup) {
      // We were removed!
      currentPeerId = null;
      isGroupChat = false;
      currentPeerName.textContent = 'Select a peer to chat';
      chatForm.classList.add('hidden');
      groupActions.classList.add('hidden');
      messagesContainer.innerHTML = '<div class="empty-state">No messages yet.</div>';
    }
  }
  renderGroups();
});

setInterval(() => {
  fetchPeers();
  fetchGroups();
}, 3000);
