const { invoke } = window.__TAURI__.core;

// Global state
let currentEnvironment = null;
let environments = [];
let environmentVariables = {};
let isConnected = false;
let serverConfig = {
    host: '127.0.0.1',
    port: 4000,
    username: '',
    password: ''
};

// DOM elements
const connectionScreen = document.getElementById('connection-screen');
const mainApp = document.getElementById('main-app');
const connectionForm = document.getElementById('connection-form');
const connectionStatus = document.getElementById('connection-status');
const environmentsList = document.getElementById('environments-list');
const noSelection = document.getElementById('no-selection');
const environmentDetails = document.getElementById('environment-details');
const currentEnvName = document.getElementById('current-env-name');
const variablesList = document.getElementById('variables-list');
const connectedServerEl = document.getElementById('connected-server');
const newEnvModal = document.getElementById('new-env-modal');
const newEnvNameInput = document.getElementById('new-env-name');

// Initialize the app
document.addEventListener('DOMContentLoaded', () => {
    setupEventListeners();
    loadSavedConnection();
});

function setupEventListeners() {
    // Connection form
    connectionForm.addEventListener('submit', handleConnection);
    
    // Disconnect button
    document.getElementById('disconnect-btn').addEventListener('click', disconnect);
    
    // Refresh button
    document.getElementById('refresh-btn').addEventListener('click', loadEnvironments);
    
    // Add environment buttons
    document.getElementById('add-env-btn').addEventListener('click', showNewEnvironmentModal);
    document.getElementById('add-env-from-empty').addEventListener('click', showNewEnvironmentModal);
    
    // Add variable button
    document.getElementById('add-var-btn').addEventListener('click', addVariable);
    
    // Save button
    document.getElementById('save-btn').addEventListener('click', saveEnvironment);
    
    // Delete environment button
    document.getElementById('delete-env-btn').addEventListener('click', deleteEnvironment);
    
    // Modal buttons
    document.getElementById('create-env-btn').addEventListener('click', createEnvironment);
    document.getElementById('cancel-env-btn').addEventListener('click', hideNewEnvironmentModal);
    
    // Close modal on background click
    newEnvModal.addEventListener('click', (e) => {
        if (e.target === newEnvModal) {
            hideNewEnvironmentModal();
        }
    });
    
    // Handle Enter key in new environment input
    newEnvNameInput.addEventListener('keydown', (e) => {
        if (e.key === 'Enter') {
            e.preventDefault();
            createEnvironment();
        }
    });
}

function loadSavedConnection() {
    const saved = localStorage.getItem('vaultd_connection');
    if (saved) {
        try {
            const config = JSON.parse(saved);
            document.getElementById('server-host').value = config.host || '127.0.0.1';
            document.getElementById('server-port').value = config.port || 4000;
            document.getElementById('username').value = config.username || '';
            document.getElementById('password').value = config.password || '';
            document.getElementById('remember-connection').checked = true;
            
            // Auto-connect if remember is enabled
            if (config.autoConnect) {
                setTimeout(() => handleConnection({ preventDefault: () => {} }), 500);
            }
        } catch (e) {
            console.error('Failed to load saved connection:', e);
        }
    }
}

async function handleConnection(event) {
    event.preventDefault();
    
    const host = document.getElementById('server-host').value.trim();
    const port = parseInt(document.getElementById('server-port').value);
    const username = document.getElementById('username').value.trim();
    const password = document.getElementById('password').value.trim();
    const remember = document.getElementById('remember-connection').checked;
    
    if (!host || !port) {
        showConnectionStatus('Please enter a valid server address and port', 'error');
        return;
    }
    
    serverConfig = { host, port, username, password };
    
    // Show connecting status
    showConnectionStatus('Connecting to server...', 'connecting');
    setConnectButtonState(true, 'Connecting...');
    
    try {
        // Test connection first
        await invoke('test_connection', { host, port });
        
        // Set server configuration
        await invoke('set_server_config', { 
            host, 
            port, 
            username: username || null, 
            password: password || null 
        });
        
        // Connection successful
        isConnected = true;
        connectedServerEl.textContent = `${host}:${port}`;
        
        // Save connection if remember is checked
        if (remember) {
            localStorage.setItem('vaultd_connection', JSON.stringify({
                ...serverConfig,
                autoConnect: true
            }));
        }
        
        // Switch to main app
        connectionScreen.classList.add('hidden');
        mainApp.classList.remove('hidden');
        
        // Load environments
        await loadEnvironments();
        
    } catch (error) {
        console.error('Connection failed:', error);
        showConnectionStatus(`Failed to connect: ${error}`, 'error');
        setConnectButtonState(false, 'Connect to Server');
    }
}

function disconnect() {
    isConnected = false;
    currentEnvironment = null;
    environments = [];
    environmentVariables = {};
    
    // Clear saved connection
    localStorage.removeItem('vaultd_connection');
    
    // Reset form
    connectionForm.reset();
    document.getElementById('server-host').value = '127.0.0.1';
    document.getElementById('server-port').value = '4000';
    
    // Hide status
    connectionStatus.classList.add('hidden');
    
    // Switch back to connection screen
    mainApp.classList.add('hidden');
    connectionScreen.classList.remove('hidden');
}

function setQuickConnect(host, port) {
    document.getElementById('server-host').value = host;
    document.getElementById('server-port').value = port;
}

function showConnectionStatus(message, type) {
    const statusEl = connectionStatus;
    statusEl.classList.remove('hidden');
    
    let classes = 'p-4 rounded-lg text-sm font-medium ';
    let icon = '';
    
    switch (type) {
        case 'connecting':
            classes += 'bg-blue-50 text-blue-700 border border-blue-200';
            icon = '<svg class="w-4 h-4 mr-2 animate-spin" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" /></svg>';
            break;
        case 'error':
            classes += 'bg-red-50 text-red-700 border border-red-200';
            icon = '<svg class="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.964-.833-2.732 0L3.732 16.5c-.77.833.192 2.5 1.732 2.5z" /></svg>';
            break;
        case 'success':
            classes += 'bg-green-50 text-green-700 border border-green-200';
            icon = '<svg class="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" /></svg>';
            break;
    }
    
    statusEl.innerHTML = `<div class="${classes}"><div class="flex items-center">${icon}${message}</div></div>`;
}

function setConnectButtonState(loading, text) {
    const btn = document.getElementById('connect-btn');
    btn.disabled = loading;
    btn.innerHTML = loading 
        ? `<svg class="w-5 h-5 mr-2 animate-spin" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" /></svg>${text}`
        : `<svg class="w-5 h-5 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z" /></svg>${text}`;
}

async function loadEnvironments() {
    try {
        environments = await invoke('list_environments');
        renderEnvironmentsList();
    } catch (error) {
        console.error('Failed to load environments:', error);
        environmentsList.innerHTML = `
            <div class="text-center py-8 text-red-600">
                <svg class="w-12 h-12 mx-auto mb-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.964-.833-2.732 0L3.732 16.5c-.77.833.192 2.5 1.732 2.5z" />
                </svg>
                <p class="text-sm">Failed to load environments</p>
                <p class="text-xs text-gray-500 mt-1">Make sure the server is running</p>
            </div>
        `;
    }
}

function renderEnvironmentsList() {
    if (environments.length === 0) {
        environmentsList.innerHTML = `
            <div class="text-center py-8 text-gray-500">
                <svg class="w-12 h-12 mx-auto mb-4 text-gray-300" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10" />
                </svg>
                <p class="text-sm">No environments found</p>
                <p class="text-xs text-gray-400 mt-1">Create your first environment to get started</p>
            </div>
        `;
        return;
    }
    
    environmentsList.innerHTML = environments.map(env => `
        <div class="env-item p-3 rounded-lg border transition-all duration-200 cursor-pointer ${env === currentEnvironment ? 'border-primary-500 bg-primary-50 shadow-md' : 'border-gray-200 bg-white hover:border-gray-300 hover:shadow-sm'}" 
             onclick="selectEnvironment('${env}')">
            <div class="flex items-center space-x-3">
                <div class="w-8 h-8 bg-gradient-to-br from-primary-500 to-primary-600 rounded-lg flex items-center justify-center flex-shrink-0">
                    <svg class="w-4 h-4 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10" />
                    </svg>
                </div>
                <div class="min-w-0 flex-1">
                    <p class="font-medium text-gray-900 truncate">${env}</p>
                    <p class="text-xs text-gray-500">Environment</p>
                </div>
            </div>
        </div>
    `).join('');
}

async function selectEnvironment(envName) {
    try {
        currentEnvironment = envName;
        
        // Update UI
        renderEnvironmentsList();
        currentEnvName.textContent = envName;
        noSelection.classList.add('hidden');
        environmentDetails.classList.remove('hidden');
        
        // Load environment variables
        environmentVariables = await invoke('get_environment', { envName });
        renderVariablesList();
        
    } catch (error) {
        console.error('Failed to load environment:', error);
        // Show error in variables list
        variablesList.innerHTML = `
            <div class="text-center py-8 text-red-600">
                <svg class="w-8 h-8 mx-auto mb-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.964-.833-2.732 0L3.732 16.5c-.77.833.192 2.5 1.732 2.5z" />
                </svg>
                <p class="text-sm">Failed to load environment</p>
            </div>
        `;
    }
}

function renderVariablesList() {
    const variables = Object.entries(environmentVariables);
    
    if (variables.length === 0) {
        variablesList.innerHTML = `
            <div class="text-center py-8 text-gray-500">
                <svg class="w-8 h-8 mx-auto mb-2 text-gray-300" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
                </svg>
                <p class="text-sm">No variables in this environment</p>
                <p class="text-xs text-gray-400 mt-1">Add your first variable to get started</p>
            </div>
        `;
        return;
    }
    
    variablesList.innerHTML = variables.map(([key, value]) => `
        <div class="variable-item bg-white border border-gray-200 rounded-lg p-4 space-y-3">
            <div class="grid grid-cols-1 gap-3">
                <div>
                    <label class="block text-xs font-medium text-gray-700 mb-1">Variable Name</label>
                    <input type="text" class="input text-sm font-mono" value="${escapeHtml(key)}" 
                           onchange="updateVariable('${escapeHtml(key)}', this.value, '${escapeHtml(value)}')" />
                </div>
                <div>
                    <label class="block text-xs font-medium text-gray-700 mb-1">Value</label>
                    <input type="text" class="input text-sm font-mono" value="${escapeHtml(value)}" 
                           onchange="updateVariableValue('${escapeHtml(key)}', this.value)" />
                </div>
                <div class="flex justify-end">
                    <button class="btn btn-danger text-xs px-2 py-1" onclick="deleteVariable('${escapeHtml(key)}')">
                        <svg class="w-3 h-3 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                        </svg>
                        Delete
                    </button>
                </div>
            </div>
        </div>
    `).join('');
}

function updateVariable(oldKey, newKey, value) {
    if (oldKey !== newKey) {
        delete environmentVariables[oldKey];
        environmentVariables[newKey] = value;
    }
}

function updateVariableValue(key, newValue) {
    environmentVariables[key] = newValue;
}

function deleteVariable(key) {
    if (confirm(`Delete variable "${key}"?`)) {
        delete environmentVariables[key];
        renderVariablesList();
    }
}

function addVariable() {
    const newKey = `NEW_VAR_${Date.now()}`;
    environmentVariables[newKey] = '';
    renderVariablesList();
    
    // Focus on the new variable key input
    setTimeout(() => {
        const newInput = document.querySelector('.variable-item:last-child input[type="text"]');
        if (newInput) {
            newInput.focus();
            newInput.select();
        }
    }, 100);
}

async function saveEnvironment() {
    if (!currentEnvironment) return;
    
    try {
        const result = await invoke('save_environment', { 
            envName: currentEnvironment, 
            variables: environmentVariables 
        });
        
        // Show success notification (you could add a toast notification here)
        console.log('Environment saved:', result);
    } catch (error) {
        console.error('Failed to save environment:', error);
        alert(`Failed to save environment: ${error}`);
    }
}

function deleteEnvironment() {
    if (!currentEnvironment) return;
    
    if (confirm(`Delete environment "${currentEnvironment}"? This action cannot be undone.`)) {
        // TODO: Implement delete environment in backend
        alert(`Deleting ${currentEnvironment} (not yet implemented)`);
    }
}

function showNewEnvironmentModal() {
    newEnvModal.classList.remove('hidden');
    newEnvNameInput.value = '';
    newEnvNameInput.focus();
}

function hideNewEnvironmentModal() {
    newEnvModal.classList.add('hidden');
}

async function createEnvironment() {
    const envName = newEnvNameInput.value.trim();
    
    if (!envName) {
        alert('Please enter an environment name');
        return;
    }
    
    if (environments.includes(envName)) {
        alert('Environment already exists');
        return;
    }
    
    try {
        // TODO: Implement create environment in backend
        // For now, we'll just add it to the list and select it
        environments.push(envName);
        environmentVariables = {};
        
        hideNewEnvironmentModal();
        await selectEnvironment(envName);
        
    } catch (error) {
        console.error('Failed to create environment:', error);
        alert(`Failed to create environment: ${error}`);
    }
}

// Helper function to send requests to vaultd server
async function sendRequest(command, environment) {
    // This function is no longer needed as we use the Tauri commands directly
    return await invoke('list_environments');
}

function escapeHtml(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
}
