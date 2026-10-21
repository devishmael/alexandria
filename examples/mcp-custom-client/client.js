/**
 * Alexandria MCP Client Example (JSON-RPC 2.0 over stdio)
 */
const { spawn } = require('child_process');

const alexandria = spawn('alexandria', ['mcp'], {
  stdio: ['pipe', 'pipe', 'inherit'],
});

let msgId = 1;

function sendRpc(method, params = {}) {
  const payload = JSON.stringify({
    jsonrpc: '2.0',
    id: msgId++,
    method,
    params,
  }) + '\n';
  alexandria.stdin.write(payload);
}

alexandria.stdout.on('data', (data) => {
  console.log('[ALEXANDRIA RESPONSE]:\n' + data.toString().trim());
  alexandria.kill();
});

// Initialize handshake
sendRpc('initialize', {
  protocolVersion: '2024-11-05',
  capabilities: {},
  clientInfo: { name: 'alexandria-example-client', version: '1.0.0' },
});
