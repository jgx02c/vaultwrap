<script>
  export let serverConfig;
  export let environments;
  export let currentEnvironment;
  export let environmentVariables;
  export let onDisconnect;
  export let onRefresh;
  export let onSelectEnvironment;
  export let onSaveEnvironment;
  export let onUpdateVariable;
  export let onDeleteVariable;
  export let onAddVariable;
  export let onDeleteEnvironment;

  let showNewEnvModal = false;
  let newEnvName = '';
  let saveStatus = '';
  let loading = false;
  let showDeleteConfirm = false;
  let envToDelete = null;
  let deleteStatus = '';

  async function handleRefresh() {
    loading = true;
    try {
      await onRefresh();
    } catch (error) {
      console.error('Failed to refresh:', error);
    } finally {
      loading = false;
    }
  }

  async function handleSave() {
    try {
      await onSaveEnvironment();
      saveStatus = 'Saved successfully!';
      setTimeout(() => saveStatus = '', 2000);
    } catch (error) {
      saveStatus = `Save failed: ${error}`;
      setTimeout(() => saveStatus = '', 3000);
    }
  }

  async function createEnvironment() {
    if (!newEnvName.trim()) return;
    
    // For now, just add to list and select it
    // TODO: Implement backend creation
    environments = [...environments, newEnvName.trim()];
    await onSelectEnvironment(newEnvName.trim());
    showNewEnvModal = false;
    newEnvName = '';
  }

  function handleDisconnect() {
    // Clear saved connection
    localStorage.removeItem('vaultd_connection');
    onDisconnect();
  }

  function updateVariableKey(oldKey, newKey) {
    if (oldKey === newKey) return;
    const value = environmentVariables[oldKey];
    onUpdateVariable(oldKey, newKey, value);
  }

  function updateVariableValue(key, value) {
    onUpdateVariable(key, key, value);
  }

  async function handleDeleteEnvironment(env) {
    showDeleteConfirm = true;
    envToDelete = env;
  }

  async function confirmDeleteEnvironment() {
    if (!envToDelete) return;
    try {
      await onDeleteEnvironment(envToDelete);
      deleteStatus = 'Environment deleted!';
      setTimeout(() => deleteStatus = '', 2000);
      if (envToDelete === currentEnvironment) {
        await onSelectEnvironment(null);
      }
      envToDelete = null;
      showDeleteConfirm = false;
    } catch (error) {
      deleteStatus = `Delete failed: ${error}`;
      setTimeout(() => deleteStatus = '', 3000);
    }
  }

  function cancelDeleteEnvironment() {
    showDeleteConfirm = false;
    envToDelete = null;
  }
</script>

<div class="h-full w-full flex flex-col bg-gray-50">
  <!-- Header -->
  <div class="bg-white border-b border-gray-200 px-6 py-4 flex items-center justify-between">
    <div class="flex items-center space-x-4">
      <div class="flex items-center space-x-3">
        <div class="w-8 h-8 bg-gradient-to-br from-blue-600 to-blue-700 rounded-lg flex items-center justify-center">
          <svg class="w-4 h-4 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z" />
          </svg>
        </div>
        <h1 class="text-lg font-semibold text-gray-900">VaultWrap</h1>
      </div>
      
      <div class="flex items-center space-x-2 text-sm">
        <div class="w-2 h-2 bg-green-500 rounded-full"></div>
        <span class="text-gray-600 font-mono">{serverConfig.host}:{serverConfig.port}</span>
      </div>
    </div>
    
    <div class="flex items-center space-x-2">
      <button 
        on:click={handleRefresh} 
        class="btn btn-secondary px-3 py-2"
        disabled={loading}
      >
        <svg class="w-4 h-4 mr-2" class:animate-spin={loading} fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
        </svg>
        Refresh
      </button>
      
      <button on:click={handleDisconnect} class="btn btn-secondary px-3 py-2">
        <svg class="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M17 16l4-4m0 0l-4-4m4 4H7m6 4v1a3 3 0 01-3 3H6a3 3 0 01-3-3V7a3 3 0 013-3h4a3 3 0 013 3v1" />
        </svg>
        Disconnect
      </button>
    </div>
  </div>

  <!-- Main Content -->
  <div class="flex-1 flex min-h-0">
    <!-- Sidebar -->
    <div class="w-80 bg-white border-r border-gray-200 flex flex-col">
      <div class="p-4 border-b border-gray-200">
        <div class="flex items-center justify-between mb-4">
          <h2 class="text-lg font-medium text-gray-900">Environments</h2>
          <button 
            on:click={() => showNewEnvModal = true}
            class="btn btn-primary px-3 py-2"
          >
            <svg class="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 6v6m0 0v6m0-6h6m-6 0H6" />
            </svg>
            New
          </button>
        </div>
      </div>
      
      <div class="flex-1 overflow-y-auto p-4">
        {#if environments.length === 0}
          <div class="text-center py-12">
            <svg class="w-12 h-12 mx-auto mb-4 text-gray-300" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10" />
            </svg>
            <p class="text-gray-500 mb-2">No environments found</p>
            <p class="text-sm text-gray-400">Create your first environment to get started</p>
          </div>
        {:else}
          <div class="space-y-2">
            {#each environments as env}
              <div class="flex items-center group">
                <button
                  on:click={() => onSelectEnvironment(env)}
                  class="flex-1 text-left p-3 rounded-lg border transition-all duration-200 {env === currentEnvironment ? 'border-blue-500 bg-blue-50 shadow-sm' : 'border-gray-200 bg-white hover:border-gray-300 hover:shadow-sm'}"
                >
                  <div class="flex items-center space-x-3">
                    <div class="w-10 h-10 bg-gradient-to-br from-blue-500 to-blue-600 rounded-lg flex items-center justify-center flex-shrink-0">
                      <svg class="w-5 h-5 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10" />
                      </svg>
                    </div>
                    <div class="min-w-0 flex-1">
                      <p class="font-medium text-gray-900 truncate">{env}</p>
                      <p class="text-sm text-gray-500">Environment</p>
                    </div>
                  </div>
                </button>
                <button
                  class="ml-2 opacity-0 group-hover:opacity-100 transition-opacity btn btn-danger px-2 py-1"
                  title="Delete environment"
                  on:click={() => handleDeleteEnvironment(env)}
                >
                  <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                  </svg>
                </button>
              </div>
            {/each}
          </div>
        {/if}
      </div>
    </div>

    <!-- Content Area -->
    <div class="flex-1 flex flex-col min-h-0">
      {#if !currentEnvironment}
        <div class="flex-1 flex items-center justify-center">
          <div class="text-center">
            <svg class="w-16 h-16 mx-auto mb-4 text-gray-300" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
            </svg>
            <h3 class="text-xl font-medium text-gray-900 mb-2">Select an Environment</h3>
            <p class="text-gray-500 mb-6">Choose an environment from the sidebar to view and manage its variables</p>
            <button 
              on:click={() => showNewEnvModal = true}
              class="btn btn-primary px-4 py-2"
            >
              <svg class="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 6v6m0 0v6m0-6h6m-6 0H6" />
              </svg>
              Create New Environment
            </button>
          </div>
        </div>
      {:else}
        <!-- Environment Details -->
        <div class="flex-1 flex flex-col">
          <!-- Environment Header -->
          <div class="bg-white border-b border-gray-200 px-6 py-4">
            <div class="flex items-center justify-between">
              <div class="flex items-center space-x-3">
                <div class="w-10 h-10 bg-gradient-to-br from-blue-600 to-blue-700 rounded-lg flex items-center justify-center">
                  <svg class="w-5 h-5 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10" />
                  </svg>
                </div>
                <div>
                  <h2 class="text-xl font-semibold text-gray-900">{currentEnvironment}</h2>
                  <p class="text-sm text-gray-500">{Object.keys(environmentVariables).length} variables</p>
                </div>
              </div>
              
              <div class="flex items-center space-x-3">
                {#if saveStatus}
                  <span class="text-sm {saveStatus.includes('failed') ? 'text-red-600' : 'text-green-600'}">{saveStatus}</span>
                {/if}
                <button on:click={handleSave} class="btn btn-primary px-4 py-2">
                  <svg class="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 7H5a2 2 0 00-2 2v9a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-3m-1 4l-3 3m0 0l-3-3m3 3V4" />
                  </svg>
                  Save Changes
                </button>
              </div>
            </div>
          </div>

          <!-- Variables -->
          <div class="flex-1 p-6 overflow-y-auto">
            <div class="flex items-center justify-between mb-6">
              <h3 class="text-lg font-medium text-gray-900">Environment Variables</h3>
              <button on:click={onAddVariable} class="btn btn-secondary px-4 py-2">
                <svg class="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 6v6m0 0v6m0-6h6m-6 0H6" />
                </svg>
                Add Variable
              </button>
            </div>
            
            {#if Object.keys(environmentVariables).length === 0}
              <div class="text-center py-12">
                <svg class="w-12 h-12 mx-auto mb-4 text-gray-300" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
                </svg>
                <p class="text-gray-500 mb-4">No variables in this environment</p>
                <button on:click={onAddVariable} class="btn btn-primary px-4 py-2">
                  <svg class="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 6v6m0 0v6m0-6h6m-6 0H6" />
                  </svg>
                  Add Your First Variable
                </button>
              </div>
            {:else}
              <div class="space-y-4">
                {#each Object.entries(environmentVariables) as [key, value]}
                  <div class="card p-4">
                                         <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
                       <div>
                         <label for="var-key-{key}" class="block text-sm font-medium text-gray-700 mb-2">Variable Name</label>
                         <input
                           id="var-key-{key}"
                           type="text"
                           value={key}
                           on:input={(e) => updateVariableKey(key, e.target.value)}
                           class="input font-mono"
                           placeholder="VARIABLE_NAME"
                         />
                       </div>
                       <div>
                         <label for="var-value-{key}" class="block text-sm font-medium text-gray-700 mb-2">Value</label>
                         <input
                           id="var-value-{key}"
                           type="text"
                           value={value}
                           on:input={(e) => updateVariableValue(key, e.target.value)}
                           class="input font-mono"
                           placeholder="variable_value"
                         />
                       </div>
                     </div>
                    <div class="flex justify-end mt-4">
                      <button 
                        on:click={() => onDeleteVariable(key)}
                        class="btn btn-danger px-3 py-2"
                      >
                        <svg class="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                        </svg>
                        Delete
                      </button>
                    </div>
                  </div>
                {/each}
              </div>
            {/if}
          </div>
        </div>
      {/if}
    </div>
  </div>
</div>

<!-- New Environment Modal -->
{#if showNewEnvModal}
  <div class="modal-overlay" role="dialog" aria-modal="true" on:click={() => showNewEnvModal = false} on:keydown={(e) => e.key === 'Escape' && (showNewEnvModal = false)}>
    <div class="modal-content" role="document" on:click|stopPropagation>
      <h3 class="text-lg font-medium text-gray-900 mb-4">Create New Environment</h3>
      <form on:submit|preventDefault={createEnvironment}>
        <div class="mb-4">
          <label for="new-env-name" class="block text-sm font-medium text-gray-700 mb-2">
            Environment Name
          </label>
          <input
            id="new-env-name"
            type="text"
            bind:value={newEnvName}
            placeholder="e.g., production, staging, development"
            class="input"
            required
          />
        </div>
        
        <div class="flex justify-end space-x-3">
          <button type="button" on:click={() => showNewEnvModal = false} class="btn btn-secondary px-4 py-2">
            Cancel
          </button>
          <button type="submit" class="btn btn-primary px-4 py-2">
            Create Environment
          </button>
        </div>
      </form>
    </div>
  </div>
{/if}

<!-- Delete Environment Confirmation Modal -->
{#if showDeleteConfirm}
  <div class="modal-overlay" role="dialog" aria-modal="true">
    <div class="modal-content" role="document">
      <h3 class="text-lg font-medium text-gray-900 mb-4">Delete Environment</h3>
      <p class="mb-4">Are you sure you want to delete the environment <span class="font-bold">{envToDelete}</span>? This action cannot be undone.</p>
      <div class="flex justify-end space-x-3">
        <button type="button" on:click={cancelDeleteEnvironment} class="btn btn-secondary px-4 py-2">
          Cancel
        </button>
        <button type="button" on:click={confirmDeleteEnvironment} class="btn btn-danger px-4 py-2">
          Delete
        </button>
      </div>
    </div>
  </div>
{/if}

<!-- Delete Status Notification -->
{#if deleteStatus}
  <div class="fixed bottom-4 right-4 bg-white border border-gray-300 rounded-lg shadow-lg px-4 py-2 text-sm {deleteStatus.includes('failed') ? 'text-red-600' : 'text-green-600'}">
    {deleteStatus}
  </div>
{/if} 