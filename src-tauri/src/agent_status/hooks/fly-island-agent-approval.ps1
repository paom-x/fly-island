# Approval hook for agents without HTTP hooks: Codex's PermissionRequest, and Z Code's PreToolUse
# (Z Code ignores the answer to its PermissionRequest, so its tool calls are decided here instead).
# This forwards the hook input to Fly Island's approval endpoint (src-tauri/src/agent_approval) and
# prints its answer, which is already in the format the agent reads. The endpoint's address and
# token are read from agent-approval.json next to this script on every run, so a changed port needs
# no reinstall.
#
# Arguments: the agent's name, then the hook the entry was installed for ("permission-request" or
# "pre-tool-use"). The installed entries pass them; older ones without arguments mean Codex's
# permission request, which is what this script was first installed for.
#
# It prints nothing, and the agent asks in its own window as usual, when the island is not running,
# approving on the island is off, the request was handed back ({}), or anything fails.
param(
    [string]$provider = 'codex',
    [string]$mode = 'permission-request'
)
$ErrorActionPreference = 'Stop'
try {
    $utf8 = New-Object System.Text.UTF8Encoding($false)
    $body = (New-Object System.IO.StreamReader([Console]::OpenStandardInput(), $utf8)).ReadToEnd()

    # Z Code reports the mode it runs in: "build" asks before changing anything, which is the mode
    # the island stands in for. In its other modes Z Code runs the tool without asking, and an
    # island card for every call would only add a stop the user did not ask for.
    if ($provider -eq 'zcode') {
        # Not $input: that name is PowerShell's own pipeline enumerator, and assigning to it in a
        # script file is silently ignored, which made this gate always exit here.
        $payload = $body | ConvertFrom-Json
        if ($payload.permission_mode -ne 'build') { exit 0 }
    }

    $config = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $PSScriptRoot 'agent-approval.json') | ConvertFrom-Json
    $port = [int]$config.port

    # Windows takes about 2 s to give up on a refused local connection; a running island accepts
    # at once, so a short probe keeps the agent from waiting while the island is closed.
    $probe = New-Object System.Net.Sockets.TcpClient
    $connected = $probe.ConnectAsync('127.0.0.1', $port).Wait(300)
    $probe.Close()
    if (-not $connected) { exit 0 }

    # The endpoint reads the whole request at once; no 100-continue round trip.
    [System.Net.ServicePointManager]::Expect100Continue = $false
    $path = "/$provider/$mode"
    $request = [System.Net.HttpWebRequest]::Create("http://127.0.0.1:$port$path")
    $request.Method = 'POST'
    $request.Proxy = $null
    $request.ContentType = 'application/json'
    $request.Headers.Add('Authorization', "Bearer $($config.token)")
    # The endpoint hands a request back after 120 s; the hook itself is allowed 150 s (Z Code gets
    # 155 s), so it always answers before the agent gives up on it.
    $request.Timeout = 140000
    $request.ReadWriteTimeout = 140000
    $bytes = $utf8.GetBytes($body)
    $request.ContentLength = $bytes.Length
    $stream = $request.GetRequestStream()
    $stream.Write($bytes, 0, $bytes.Length)
    $stream.Close()

    $response = $request.GetResponse()
    $answer = (New-Object System.IO.StreamReader($response.GetResponseStream(), $utf8)).ReadToEnd().Trim()
    $response.Close()
    if ($answer -and $answer -ne '{}') {
        $out = [Console]::OpenStandardOutput()
        $data = $utf8.GetBytes($answer)
        $out.Write($data, 0, $data.Length)
        $out.Flush()
    }
} catch {
    # No decision: the agent shows its own prompt.
}
exit 0
