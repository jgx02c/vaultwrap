<script>
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import ConnectionScreen from './lib/ConnectionScreen.svelte';
  import MainApp from './lib/MainApp.svelte';

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
      
      // Load environments
      await loadEnvironments();
    } catch (error) {
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
      console.error('Failed to load environments:', error);
      throw error;
    }
  }

  async function selectEnvironment(envName) {
    try {
      currentEnvironment = envName;
      environmentVariables = await invoke('get_environment', { envName });
    } catch (error) {
      console.error('Failed to load environment:', error);
      throw error;
    }
  }

  async function saveEnvironment() {
    if (!currentEnvironment) return;
    
    try {
      await invoke('save_environment', { 
        envName: currentEnvironment, 
        variables: environmentVariables 
      });
      return true;
    } catch (error) {
      console.error('Failed to save environment:', error);
      throw error;
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
  }

  function deleteVariable(key) {
    delete environmentVariables[key];
    environmentVariables = { ...environmentVariables };
  }

  function addVariable() {
    const newKey = `NEW_VAR_${Date.now()}`;
    environmentVariables[newKey] = '';
    environmentVariables = { ...environmentVariables };
  }

  async function deleteEnvironment(envName) {
    try {
      await invoke('delete_environment', { envName });
      await loadEnvironments();
      if (currentEnvironment === envName) {
        currentEnvironment = null;
        environmentVariables = {};
      }
    } catch (error) {
      console.error('Failed to delete environment:', error);
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
      onSaveEnvironment={saveEnvironment}
      onUpdateVariable={updateVariable}
      onDeleteVariable={deleteVariable}
      onAddVariable={addVariable}
      onDeleteEnvironment={deleteEnvironment}
    />
  {/if}
</main> 