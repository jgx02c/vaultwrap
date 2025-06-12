const { invoke } = window.__TAURI__.core;

// Global state
let currentEnvironment = null;
let environments = [];
let environmentVariables = {};

// DOM elements
const environmentsList = document.getElementById('environments-list');
const noSelection = document.getElementById('no-selection');
const environmentDetails = document.getElementById('environment-details');
const currentEnvName = document.getElementById('current-env-name');
const variablesList = document.getElementById('variables-list');
const statusText = document.getElementById('status-text');
const newEnvModal = document.getElementById('new-env-modal');
const newEnvNameInput = document.getElementById('new-env-name');

// Initialize the app
document.addEventListener('DOMContentLoaded', () => {
    setupEventListeners();
    loadEnvironments();
});

function setupEventListeners() {
    // Refresh button
    document.getElementById('refresh-btn').addEventListener('click', loadEnvironments);
    
    // Add environment button
    document.getElementById('add-env-btn').addEventListener('click', showNewEnvironmentModal);
    
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
}

async function loadEnvironments() {
    try {
        updateStatus('Loading environments...');
        environments = await invoke('list_environments');
        renderEnvironmentsList();
        updateStatus('Ready');
    } catch (error) {
        console.error('Failed to load environments:', error);
        updateStatus(`Error: ${error}`);
        environmentsList.innerHTML = '<div class="loading">Failed to load environments. Make sure vaultd server is running.</div>';
    }
}

function renderEnvironmentsList() {
    if (environments.length === 0) {
        environmentsList.innerHTML = '<div class="loading">No environments found</div>';
        return;
    }
    
    environmentsList.innerHTML = environments.map(env => `
        <div class="environment-item ${env === currentEnvironment ? 'active' : ''}" 
             onclick="selectEnvironment('${env}')">
            ${env}
        </div>
    `).join('');
}

async function selectEnvironment(envName) {
    try {
        updateStatus(`Loading ${envName}...`);
        currentEnvironment = envName;
        
        // Update UI
        renderEnvironmentsList();
        currentEnvName.textContent = envName;
        noSelection.style.display = 'none';
        environmentDetails.style.display = 'flex';
        
        // Load environment variables
        environmentVariables = await invoke('get_environment', { envName });
        renderVariablesList();
        
        updateStatus('Ready');
    } catch (error) {
        console.error('Failed to load environment:', error);
        updateStatus(`Error loading ${envName}: ${error}`);
    }
}

function renderVariablesList() {
    const variables = Object.entries(environmentVariables);
    
    if (variables.length === 0) {
        variablesList.innerHTML = '<div class="loading">No variables in this environment</div>';
        return;
    }
    
    variablesList.innerHTML = variables.map(([key, value]) => `
        <div class="variable-item">
            <input type="text" class="key-input" value="${escapeHtml(key)}" 
                   onchange="updateVariable('${escapeHtml(key)}', this.value, '${escapeHtml(value)}')" />
            <input type="text" class="value-input" value="${escapeHtml(value)}" 
                   onchange="updateVariableValue('${escapeHtml(key)}', this.value)" />
            <button class="delete-var-btn" onclick="deleteVariable('${escapeHtml(key)}')">🗑️</button>
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
        const newInput = document.querySelector(`.variable-item:last-child .key-input`);
        if (newInput) {
            newInput.focus();
            newInput.select();
        }
    }, 100);
}

async function saveEnvironment() {
    if (!currentEnvironment) return;
    
    try {
        updateStatus(`Saving ${currentEnvironment}...`);
        const result = await invoke('save_environment', { 
            envName: currentEnvironment, 
            variables: environmentVariables 
        });
        updateStatus(result);
    } catch (error) {
        console.error('Failed to save environment:', error);
        updateStatus(`Error saving: ${error}`);
    }
}

function deleteEnvironment() {
    if (!currentEnvironment) return;
    
    if (confirm(`Delete environment "${currentEnvironment}"? This action cannot be undone.`)) {
        // TODO: Implement delete environment in backend
        updateStatus(`Deleting ${currentEnvironment} (not yet implemented)`);
    }
}

function showNewEnvironmentModal() {
    newEnvModal.style.display = 'flex';
    newEnvNameInput.value = '';
    newEnvNameInput.focus();
}

function hideNewEnvironmentModal() {
    newEnvModal.style.display = 'none';
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
        updateStatus(`Creating ${envName}...`);
        
        // TODO: Implement create environment in backend
        // For now, we'll just add it to the list and select it
        environments.push(envName);
        environmentVariables = {};
        
        hideNewEnvironmentModal();
        await selectEnvironment(envName);
        
        updateStatus(`Created ${envName}`);
    } catch (error) {
        console.error('Failed to create environment:', error);
        updateStatus(`Error creating environment: ${error}`);
    }
}

function updateStatus(message) {
    statusText.textContent = message;
}

function escapeHtml(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
}

// Handle Enter key in modal
newEnvNameInput.addEventListener('keydown', (e) => {
    if (e.key === 'Enter') {
        createEnvironment();
    }
});
