$content = [System.IO.File]::ReadAllText('c:\Users\point\AppData\Local\Temp\aicoding-text-field\d44569ef-1d18-4416-b554-6ed05f26380a\text-field-content-439761dc-d25e-4697-ae5c-60c5a8acf216.txt')
# Search for panic/crash markers
$patterns = @('CRITICAL PANIC', 'PANIC LOCATION', 'PANIC PAYLOAD', 'panicked at', 'thread.*panicked', 'Error:', 'error[E')
foreach ($p in $patterns) {
    $idx = $content.IndexOf($p)
    if ($idx -ge 0) {
        $start = [Math]::Max(0, $idx - 100)
        $len = [Math]::Min(500, $content.Length - $start)
        Write-Output "=== FOUND '$p' at index $idx ==="
        Write-Output $content.Substring($start, $len)
        Write-Output ""
    }
}
# Also show last 3000 chars (often has the crash summary)
$tail = [Math]::Max(0, $content.Length - 3000)
Write-Output "=== LAST 3000 CHARS ==="
Write-Output $content.Substring($tail)
