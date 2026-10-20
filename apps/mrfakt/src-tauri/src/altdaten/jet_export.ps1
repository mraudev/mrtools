# Exportiert alle Tabellen alter Access-Datenbanken (Jet 3/4) als JSON.
# Läuft in der 32-bit-PowerShell, weil nur der 32-bit-Provider Microsoft.Jet.OLEDB.4.0 diese Formate öffnet.
# Jede Zeile der Auftragsdatei: <Quelle>|<Ziel.json>. Die Quelle wird vorher kopiert, damit Jet dort keine
# .ldb-Sperrdatei anlegt. Ergebnis je Datei: {"Tabelle":[{"Spalte":Wert,...},...],...}
param([string]$Jobs)

$ErrorActionPreference = 'Stop'
$tmp = Join-Path $env:TEMP ('mrfakt-jet-' + [guid]::NewGuid())
New-Item -ItemType Directory -Force $tmp | Out-Null

$ctl = [regex]'[\x00-\x1f]'
function Esc([string]$s) {
  if ($s.IndexOfAny([char[]]'"\') -ge 0) { $s = $s.Replace('\', '\\').Replace('"', '\"') }
  if ($ctl.IsMatch($s)) { $s = $ctl.Replace($s, { param($m) '\u{0:x4}' -f [int][char]$m.Value }) }
  '"' + $s + '"'
}

function Val($v) {
  if ($v -is [DBNull] -or $null -eq $v) { return 'null' }
  if ($v -is [bool]) { if ($v) { return 'true' } else { return 'false' } }
  if ($v -is [datetime]) { return Esc ($v.ToString('yyyy-MM-ddTHH:mm:ss')) }
  if ($v -is [byte[]]) { return 'null' }
  if ($v -is [string]) { return Esc $v }
  # Zahlen als Text, damit Dezimalwerte (Währung) nicht durch double laufen.
  return Esc ([System.Convert]::ToString($v, [System.Globalization.CultureInfo]::InvariantCulture))
}

$n = 0
foreach ($line in [IO.File]::ReadAllLines($Jobs, [Text.Encoding]::UTF8)) {
  if (-not $line.Trim()) { continue }
  $parts = $line.Split('|')
  $src = $parts[0]; $out = $parts[1]
  $n++
  $copy = Join-Path $tmp "$n.mdb"
  try {
    Copy-Item -LiteralPath $src -Destination $copy
    $c = New-Object System.Data.OleDb.OleDbConnection("Provider=Microsoft.Jet.OLEDB.4.0;Data Source=$copy;Mode=Read")
    $c.Open()
    $w = New-Object System.IO.StreamWriter($out, $false, (New-Object System.Text.UTF8Encoding($false)))
    $w.Write('{')
    $first = $true
    foreach ($t in ($c.GetSchema('Tables') | Where-Object { $_.TABLE_TYPE -eq 'TABLE' })) {
      $name = $t.TABLE_NAME
      if (-not $first) { $w.Write(',') }
      $first = $false
      $w.Write((Esc $name) + ':[')
      $cmd = $c.CreateCommand()
      $cmd.CommandText = "SELECT * FROM [$name]"
      $rd = $cmd.ExecuteReader()
      $cols = @(for ($i = 0; $i -lt $rd.FieldCount; $i++) { Esc $rd.GetName($i) })
      $firstRow = $true
      while ($rd.Read()) {
        if (-not $firstRow) { $w.Write(',') }
        $firstRow = $false
        $w.Write('{')
        for ($i = 0; $i -lt $rd.FieldCount; $i++) {
          if ($i) { $w.Write(',') }
          $w.Write($cols[$i] + ':' + (Val $rd.GetValue($i)))
        }
        $w.Write('}')
      }
      $rd.Close()
      $w.Write(']')
    }
    $w.Write('}')
    $w.Close()
    $c.Close()
  } catch {
    [IO.File]::WriteAllText($out + '.error', $_.ToString())
  } finally {
    Remove-Item -LiteralPath $copy -ErrorAction SilentlyContinue
  }
}
Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue
