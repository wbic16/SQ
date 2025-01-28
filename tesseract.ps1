param(
  [int] $N = 10
)
function bg() {Start-Process -NoNewWindow @args}

$sc = 1
$sn = 1
$ch = 1
$bk = 1
Stop-Process -Name "sq.exe"
Remove-Item -Recurse -Force ".sq"
cargo build --release
bg .\target\release\sq.exe tesseract.phext >tesseract.stdout 2>tesseract.stderr 6>&1
while ($bk -lt $N) {
  while ($ch -lt $N) {
    while ($sn -lt $N) {
      while ($sc -lt $N) {
        .\target\release\sq.exe update "1.1.1/1.1.$bk/$ch.$sn.$sc" "Book $bk, Chapter $ch, Section $sn, Scroll $sc" >$nul 2>&1 6>&1
        ++$sc
      }
      $sc = 1
      ++$sn
    }
    $sc = 1
    $sn = 1
    ++$ch
  }
  $sc = 1
  $sn = 1
  $ch = 1
  ++$bk
}
.\target\release\sq.exe save tesseract.phext
.\target\release\sq.exe shutdown
Write-Host "Tesseract Setup Complete (N=$N)"