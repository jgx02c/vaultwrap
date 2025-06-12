<script>
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import ConnectionScreen from './lib/ConnectionScreen.svelte';
  import MainApp from './lib/MainApp.svelte';

  // Toast state
  let toast = { show: false, message: '', type: 'success' };
  let toastTimeout;
  function showToast(message, type = 'success') {
    toast = { show: true, message, type };
    clearTimeout(toastTimeout);
    toastTimeout = setTimeout(() => toast.show = false, 2500);
  }

  // Global state
  let isConnected = false;
  let serverConfig = {
    host: '127.0.0.1',
    port: 4000,
    username: '',
    password: ''
  };

  let environments = [];
  let currentEnvironment = null;
  let environmentVariables = {};

  // Connection functions
  async function handleConnect(config) {
    try {
      await invoke('test_connection', { host: config.host, port: config.port });
      await invoke('set_server_config', { 
        host: config.host, 
        port: config.port, 
        username: config.username || null, 
        password: config.password || null 
      });
      serverConfig = config;
      isConnected = true;
      await loadEnvironments();
    } catch (error) {
      showToast('Failed to connect: ' + error, 'error');
      throw error;
    }
  }

  function handleDisconnect() {
    isConnected = false;
    currentEnvironment = null;
    environments = [];
    environmentVariables = {};
  }

  async function loadEnvironments() {
    try {
      environments = await invoke('list_environments');
    } catch (error) {
      showToast('Failed to load environments', 'error');
      throw error;
    }
  }

  async function selectEnvironment(envName) {
    try {
      currentEnvironment = envName;
      if (envName) {
        environmentVariables = await invoke('get_environment', { envName });
      } else {
        environmentVariables = {};
      }
    } catch (error) {
      showToast('Failed to load environment', 'error');
      throw error;
    }
  }

  async function autoSaveEnvironment() {
    if (!currentEnvironment) return;
    try {
      await invoke('save_environment', { 
        envName: currentEnvironment, 
        variables: environmentVariables 
      });
      showToast('Saved!', 'success');
    } catch (error) {
      showToast('Auto-save failed: ' + error, 'error');
    }
  }

  function updateVariable(key, newKey, value) {
    if (key !== newKey) {
      delete environmentVariables[key];
      environmentVariables[newKey] = value;
    } else {
      environmentVariables[key] = value;
    }
    environmentVariables = { ...environmentVariables };
    autoSaveEnvironment();
  }

  function deleteVariable(key) {
    delete environmentVariables[key];
    environmentVariables = { ...environmentVariables };
    autoSaveEnvironment();
  }

  function addVariable() {
    const newKey = `NEW_VAR_${Date.now()}`;
    environmentVariables[newKey] = '';
    environmentVariables = { ...environmentVariables };
    autoSaveEnvironment();
  }

  async function deleteEnvironment(envName) {
    try {
      await invoke('delete_environment', { envName });
      await loadEnvironments();
      if (currentEnvironment === envName) {
        currentEnvironment = null;
        environmentVariables = {};
      }
      showToast('Environment deleted!', 'success');
    } catch (error) {
      showToast('Failed to delete environment', 'error');
      throw error;
    }
  }

  async function createEnvironment(envName) {
    try {
      await invoke('create_environment', { envName });
      await loadEnvironments();
      await selectEnvironment(envName);
      showToast('Environment created!', 'success');
    } catch (error) {
      showToast('Failed to create environment', 'error');
      throw error;
    }
  }
</script>

<main class="h-full w-full">
  {#if !isConnected}
    <ConnectionScreen 
      {serverConfig} 
      onConnect={handleConnect} 
    />
  {:else}
    <MainApp
      {serverConfig}
      {environments}
      {currentEnvironment}
      {environmentVariables}
      onDisconnect={handleDisconnect}
      onRefresh={loadEnvironments}
      onSelectEnvironment={selectEnvironment}
      onUpdateVariable={updateVariable}
      onDeleteVariable={deleteVariable}
      onAddVariable={addVariable}
      onDeleteEnvironment={deleteEnvironment}
      onCreateEnvironment={createEnvironment}
    />
    {#if toast.show}
      <div class="fixed bottom-6 right-6 z-50">
        <div class="px-4 py-3 rounded-lg shadow-lg text-white flex items-center space-x-3"
          class:bg-green-600={toast.type === 'success'}
          class:bg-red-600={toast.type === 'error'}>
          {#if toast.type === 'success'}
            <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7" /></svg>
          {:else}
            <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" /></svg>
          {/if}
          <span>{toast.message}</span>
        </div>
      </div>
    {/if}
  {/if}
</main> 