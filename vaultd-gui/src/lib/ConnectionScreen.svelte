<script>
  export let serverConfig;
  export let onConnect;

  let host = serverConfig.host;
  let port = serverConfig.port;
  let username = serverConfig.username;
  let password = serverConfig.password;
  let rememberConnection = false;
  
  let isConnecting = false;
  let connectionError = '';

  async function handleSubmit() {
    if (!host || !port) {
      connectionError = 'Please enter a valid server address and port';
      return;
    }

    isConnecting = true;
    connectionError = '';

    try {
      const config = { host, port: parseInt(port), username, password };
      
      // Save connection if remember is checked
      if (rememberConnection) {
        localStorage.setItem('vaultd_connection', JSON.stringify({
          ...config,
          autoConnect: true
        }));
      }

      await onConnect(config);
    } catch (error) {
      connectionError = `Failed to connect: ${error}`;
    } finally {
      isConnecting = false;
    }
  }

  // Load saved connection on mount
  function loadSavedConnection() {
    const saved = localStorage.getItem('vaultd_connection');
    if (saved) {
      try {
        const config = JSON.parse(saved);
        host = config.host || '127.0.0.1';
        port = config.port || 4000;
        username = config.username || '';
        password = config.password || '';
        rememberConnection = true;
        
        // Auto-connect if remember is enabled
        if (config.autoConnect) {
          setTimeout(handleSubmit, 500);
        }
      } catch (e) {
        console.error('Failed to load saved connection:', e);
      }
    }
  }

  // Load saved connection when component mounts
  loadSavedConnection();
</script>

<div class="h-full w-full flex items-center justify-center bg-gray-50 p-6">
  <div class="w-full max-w-md">
    <!-- Header -->
    <div class="text-center mb-8">
      <div class="mx-auto w-16 h-16 bg-gradient-to-br from-blue-600 to-blue-700 rounded-xl flex items-center justify-center mb-4 shadow-lg">
        <svg class="w-8 h-8 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z" />
        </svg>
      </div>
      <h1 class="text-2xl font-bold text-gray-900 mb-2">VaultWrap</h1>
      <p class="text-gray-600">Connect to your environment server</p>
    </div>

    <!-- Connection Form -->
    <div class="card p-6">
      <form on:submit|preventDefault={handleSubmit} class="space-y-4">
        <!-- Server Address -->
        <div>
          <label for="server-host" class="block text-sm font-medium text-gray-700 mb-2">
            Server Address
          </label>
          <input
            id="server-host"
            type="text"
            bind:value={host}
            placeholder="127.0.0.1 or your-server.com"
            class="input"
            required
            disabled={isConnecting}
          />
        </div>

        <!-- Port -->
        <div>
          <label for="server-port" class="block text-sm font-medium text-gray-700 mb-2">
            Port
          </label>
          <input
            id="server-port"
            type="number"
            bind:value={port}
            placeholder="4000"
            class="input"
            required
            disabled={isConnecting}
          />
        </div>

        <!-- Optional Credentials -->
        <div class="grid grid-cols-2 gap-3">
          <div>
            <label for="username" class="block text-sm font-medium text-gray-700 mb-2">
              Username <span class="text-gray-400 text-xs">(optional)</span>
            </label>
            <input
              id="username"
              type="text"
              bind:value={username}
              placeholder="Username"
              class="input"
              disabled={isConnecting}
            />
          </div>
          <div>
            <label for="password" class="block text-sm font-medium text-gray-700 mb-2">
              Password <span class="text-gray-400 text-xs">(optional)</span>
            </label>
            <input
              id="password"
              type="password"
              bind:value={password}
              placeholder="Password"
              class="input"
              disabled={isConnecting}
            />
          </div>
        </div>

        <!-- Remember Connection -->
        <div class="flex items-center">
          <input
            id="remember-connection"
            type="checkbox"
            bind:checked={rememberConnection}
            class="h-4 w-4 text-blue-600 focus:ring-blue-500 border-gray-300 rounded"
            disabled={isConnecting}
          />
          <label for="remember-connection" class="ml-2 text-sm text-gray-600">
            Remember this connection
          </label>
        </div>

        <!-- Error Message -->
        {#if connectionError}
          <div class="bg-red-50 border border-red-200 rounded-md p-3">
            <div class="flex">
              <svg class="w-5 h-5 text-red-400 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.964-.833-2.732 0L3.732 16.5c-.77.833.192 2.5 1.732 2.5z" />
              </svg>
              <p class="text-sm text-red-700">{connectionError}</p>
            </div>
          </div>
        {/if}

        <!-- Connect Button -->
        <button 
          type="submit" 
          class="btn btn-primary w-full"
          disabled={isConnecting}
        >
          {#if isConnecting}
            <svg class="w-4 h-4 mr-2 animate-spin" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
            </svg>
            Connecting...
          {:else}
            <svg class="w-4 h-4 mr-2" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z" />
            </svg>
            Connect to Server
          {/if}
        </button>
      </form>
    </div>
  </div>
</div> 