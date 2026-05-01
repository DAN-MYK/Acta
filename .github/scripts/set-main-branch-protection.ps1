param(
    [string]$Owner = "DAN-MYK",
    [string]$Repo = "Acta",
    [string]$Branch = "main"
)

$requiredChecks = @(
    "Frontend build and checks",
    "Rust backend legacy safety net",
    "SQLx offline metadata contract",
    "Tauri migration compile gate",
    "Postgres integration and Tauri vertical slice"
)

$body = @{
    required_status_checks = @{
        strict   = $true
        contexts = $requiredChecks
    }
    enforce_admins = $true
    required_pull_request_reviews = @{
        dismiss_stale_reviews           = $true
        require_code_owner_reviews      = $false
        required_approving_review_count = 1
        require_last_push_approval      = $false
    }
    restrictions = $null
    required_linear_history = $false
    allow_force_pushes = $false
    allow_deletions = $false
    block_creations = $false
    required_conversation_resolution = $true
    lock_branch = $false
    allow_fork_syncing = $false
} | ConvertTo-Json -Depth 10

$tempFile = New-TemporaryFile

try {
    Set-Content -LiteralPath $tempFile -Value $body -Encoding utf8

    gh api `
      --method PUT `
      -H "Accept: application/vnd.github+json" `
      "/repos/$Owner/$Repo/branches/$Branch/protection" `
      --input $tempFile
}
finally {
    if (Test-Path -LiteralPath $tempFile) {
        Remove-Item -LiteralPath $tempFile -Force
    }
}
