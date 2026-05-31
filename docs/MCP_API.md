# SerialRUN MCP API Reference

SerialRUN includes a built-in MCP (Model Context Protocol) server that allows AI assistants to remotely control serial devices via TCP.

## Connection Info

- **Protocol**: JSON-RPC over TCP
- **Address**: 127.0.0.1:9527 (default)
- **Encoding**: UTF-8
- **Delimiter**: Newline `\n`

## Message Format

### Request

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "tool_name",
    "arguments": { ... }
  }
}
```

### Response

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "content": [
      {
        "type": "text",
        "text": "response text"
      }
    ]
  }
}
```

### Error Response

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "error": {
    "code": -1,
    "message": "error description"
  }
}
```

---

## Tools (15)

### 1. list_ports

List all available serial ports.

**Parameters**: None

**Example Response**:
```json
{
  "content": [{
    "type": "text",
    "text": "[\n  {\n    \"name\": \"COM1\",\n    \"description\": null,\n    \"manufacturer\": null\n  }\n]"
  }]
}
```

### 2. connect

Connect to a serial port. If not connected, automatically creates a port_owner.

**Parameters**:
| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| port | string | Yes | - | Port name (e.g., COM1, /dev/ttyUSB0) |
| baud_rate | integer | No | 115200 | Baud rate |
| data_bits | integer | No | 8 | Data bits: 5, 6, 7, 8 |
| stop_bits | integer | No | 1 | Stop bits: 1, 2 |
| parity | string | No | "None" | Parity: None, Odd, Even |
| flow_control | string | No | "None" | Flow control: None, Software, Hardware |

**Example Request**:
```json
{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"connect","arguments":{"port":"COM3","baud_rate":115200}}}
```

**Example Response**:
```json
{"jsonrpc":"2.0","id":1,"result":{"content":[{"type":"text","text":"Connected to COM3 at 115200 baud"}]}}
```

### 3. disconnect

Disconnect from the current serial port.

**Parameters**: None

### 4. send

Send data to the serial port.

**Parameters**:
| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| data | string | Yes | - | Data to send |
| hex | boolean | No | false | If true, data is interpreted as hex |
| pause_after | boolean | No | false | If true, pauses the read loop after sending so next read() can receive the response |

**Example Request**:
```json
{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"send","arguments":{"data":"AT\r\n","hex":false}}}
```

**HEX Format Example**:
```json
{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"send","arguments":{"data":"48 65 6C 6C 6F","hex":true}}}
```

### 5. read

Read data from the serial port RX buffer. Data is automatically captured by background continuous monitoring.

**Parameters**:
| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| timeout_ms | integer | No | 1000 | Read timeout (ms) |
| max_bytes | integer | No | 1024 | Maximum bytes to read |
| resume | boolean | No | true | If true, resumes the read loop after reading. Set to false when reading after send(pause_after=true). |
| format | string | No | "hex" | Output format: 'hex', 'text', 'raw' (base64) |

**Example Response**:
```json
{"jsonrpc":"2.0","id":1,"result":{"content":[{"type":"text","text":"Read 5 bytes\nHEX: 4F 4B 0D 0A\nText: OK\r\n"}]}}
```

### 6. send_command

Send a command and read response from buffer. Recommended for AT command interaction.

**Parameters**:
| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| command | string | Yes | - | Command to send |
| timeout_ms | integer | No | 1000 | Response timeout (ms) |

**Example Request**:
```json
{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"send_command","arguments":{"command":"AT","timeout_ms":1000}}}
```

### 7. modbus_read

Read Modbus RTU holding registers with optional engineering value conversion.

**Parameters**:
| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| slave_id | integer | No | 1 | Slave ID (1-247) |
| address | integer | Yes | - | Start register address |
| quantity | integer | No | 1 | Number of registers |
| scale | number | No | 1.0 | Scale factor: value = raw * scale + offset |
| offset | number | No | 0.0 | Offset added after scaling |
| unit | string | No | "" | Unit label for engineering values |

**Example Request**:
```json
{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"modbus_read","arguments":{"slave_id":1,"address":0,"quantity":10}}}
```

**Example Response**:
```json
{"jsonrpc":"2.0","id":1,"result":{"content":[{"type":"text","text":"Read 10 registers from slave 1\nHEX: 01 03 14 00 01 00 02...\nValues: [1, 2, 3, ...]"}]}}
```

### 8. modbus_write

Write a Modbus RTU holding register.

**Parameters**:
| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| slave_id | integer | No | 1 | Slave ID (1-247) |
| address | integer | Yes | - | Register address |
| value | integer | Yes | - | Value to write (u16) |

### 9. plc_read

Read all registers from a PLC preset brand.

**Parameters**:
| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| brand | string | No | "Siemens" | PLC brand: Siemens, Mitsubishi, Delta, Omron |
| slave_id | integer | No | 1 | Slave ID (1-247) |

### 10. plc_write

Write to a PLC register by address.

**Parameters**:
| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| brand | string | No | "Siemens" | PLC brand |
| slave_id | integer | No | 1 | Slave ID |
| address | integer | Yes | - | Register address |
| value | number | Yes | - | Value to write |

### 11. status

Get serial port status, connection info, and byte counters.

**Parameters**: None

**Example Response**:
```json
{
  "content": [{
    "type": "text",
    "text": "Connected: COM3 @ 115200 baud\nRX bytes: 1024\nTX bytes: 256\nMCP Server: Running on 127.0.0.1:9527\nActive clients: 1"
  }]
}
```

### 12. get_config

Get all UI settings or a specific setting value.

**Parameters**:
| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| key | string | No | - | Setting key (optional, returns all if omitted) |

### 13. set_config

Update a UI setting (syncs to GUI immediately).

**Parameters**:
| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| key | string | Yes | - | Setting key |
| value | any | Yes | - | New value |

### 14. get_access_log

View MCP access log with client IPs, tool calls, and timestamps.

**Parameters**:
| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| limit | integer | No | 50 | Max entries to return |

**Example Response**:
```json
{
  "content": [{
    "type": "text",
    "text": "Active clients: 1\n\nAccess Log (last 3):\n[\n  {\n    \"time\": \"2026-05-29 10:00:00.123\",\n    \"ip\": \"192.168.1.100\",\n    \"action\": \"CALL\",\n    \"detail\": \"list_ports\"\n  }\n]"
  }]
}
```

### 15. get_device_info

Get current device identification info (port, baud rate, connection status).

**Parameters**: None

**Example Response**:
```json
{
  "content": [{
    "type": "text",
    "text": "Device: SerialRUN\nStatus: Connected\nPort: COM3 @ 115200\nActive clients: 1\nTotal access log entries: 10\nServer: MCP v0.2.0\nProtocol: JSON-RPC over TCP"
  }]
}
```

---

## Error Codes

| Code | Description |
|------|-------------|
| -1 | General error |
| -32601 | Unknown method |
| -32602 | Invalid params |
| -32700 | Parse error |

---

## Usage Flow

### 1. Initialize Connection

```bash
# Connect to MCP server
nc 127.0.0.1 9527

# Send initialize request
{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}
```

### 2. List Tools

```bash
{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}
```

### 3. Connect Serial Port

```bash
{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"connect","arguments":{"port":"COM3","baud_rate":115200}}}
```

### 4. Send Data

```bash
{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"send","arguments":{"data":"Hello\r\n"}}}
```

### 5. Read Response

```bash
{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"read","arguments":{"timeout_ms":1000}}}
```

### 6. Disconnect

```bash
{"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"disconnect","arguments":{}}}
```

---

## Notes

1. **Port Exclusivity**: On Windows, serial ports are exclusive resources — only one connection at a time
2. **Timeout**: Recommended send_command timeout_ms of 500-2000ms
3. **HEX Format**: Hex data is space-separated, e.g., "48 65 6C 6C 6F"
4. **Text Line Endings**: Text sending automatically appends \r\n
5. **Access Log**: All operations automatically log client IPs for traceability
6. **Concurrency**: Multiple clients can connect simultaneously, but serial operations are queued

---

## Python Client Example

```python
import socket
import json
import time

def mcp_call(host, port, method, params=None, req_id=1):
    sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    sock.settimeout(5)
    sock.connect((host, port))
    
    msg = {'jsonrpc': '2.0', 'id': req_id, 'method': method}
    if params:
        msg['params'] = params
    sock.sendall((json.dumps(msg) + '\n').encode())
    
    time.sleep(0.3)
    data = b''
    while b'\n' not in data:
        chunk = sock.recv(4096)
        if not chunk:
            break
        data += chunk
    
    sock.close()
    return json.loads(data.decode().strip()) if data else None

# Usage
result = mcp_call('127.0.0.1', 9527, 'tools/call', {
    'name': 'connect',
    'arguments': {'port': 'COM3', 'baud_rate': 115200}
})
print(result)
```

---

## JavaScript/Node.js Client Example

```javascript
const net = require('net');

function mcpCall(host, port, method, params = null, reqId = 1) {
    return new Promise((resolve, reject) => {
        const client = net.createConnection({ host, port }, () => {
            const msg = { jsonrpc: '2.0', id: reqId, method };
            if (params) msg.params = params;
            client.write(JSON.stringify(msg) + '\n');
        });
        
        let data = '';
        client.on('data', (chunk) => {
            data += chunk.toString();
            if (data.includes('\n')) {
                client.end();
                resolve(JSON.parse(data.trim()));
            }
        });
        
        client.on('error', reject);
        setTimeout(() => { client.end(); reject(new Error('Timeout')); }, 5000);
    });
}

// Usage
mcpCall('127.0.0.1', 9527, 'tools/call', {
    name: 'connect',
    arguments: { port: 'COM3', baud_rate: 115200 }
}).then(console.log);
```
