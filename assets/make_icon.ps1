# Génère le logo d'Éclair : éclair vert sur carré noir arrondi.
# Produit assets\eclair.png (256 px) et assets\eclair.ico (16, 32, 48, 256 px, PNG intégrés).
Add-Type -AssemblyName System.Drawing
$dir = $PSScriptRoot

function Draw-Logo([int]$size) {
    $bmp = New-Object System.Drawing.Bitmap $size, $size
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.SmoothingMode = 'AntiAlias'
    $s = $size / 256.0

    # Carré arrondi sombre avec liseré vert.
    $r = 56 * $s; $m = 8 * $s; $w = $size - 2 * $m
    $path = New-Object System.Drawing.Drawing2D.GraphicsPath
    $path.AddArc($m, $m, $r, $r, 180, 90); $path.AddArc($m + $w - $r, $m, $r, $r, 270, 90)
    $path.AddArc($m + $w - $r, $m + $w - $r, $r, $r, 0, 90); $path.AddArc($m, $m + $w - $r, $r, $r, 90, 90)
    $path.CloseFigure()
    $g.FillPath((New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::FromArgb(255, 14, 14, 14))), $path)
    $g.DrawPath((New-Object System.Drawing.Pen ([System.Drawing.Color]::FromArgb(255, 34, 224, 122)), ([Math]::Max(1, 6 * $s))), $path)

    # Éclair.
    $pts = @(@(150, 30), @(70, 142), @(122, 142), @(100, 226), @(186, 104), @(132, 104), @(150, 30)) |
        ForEach-Object { New-Object System.Drawing.PointF ($_[0] * $s), ($_[1] * $s) }
    $g.FillPolygon((New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::FromArgb(255, 34, 224, 122))), [System.Drawing.PointF[]]$pts)
    $g.Dispose()
    $ms = New-Object System.IO.MemoryStream
    $bmp.Save($ms, [System.Drawing.Imaging.ImageFormat]::Png)
    $bmp.Dispose()
    return , $ms.ToArray()
}

$sizes = 16, 32, 48, 256
$images = $sizes | ForEach-Object { , (Draw-Logo $_) }
[System.IO.File]::WriteAllBytes("$dir\eclair.png", $images[3])

# En-tête ICO + répertoire + images PNG.
$out = New-Object System.IO.MemoryStream
$bw = New-Object System.IO.BinaryWriter $out
$bw.Write([UInt16]0); $bw.Write([UInt16]1); $bw.Write([UInt16]$sizes.Count)
$offset = 6 + 16 * $sizes.Count
for ($i = 0; $i -lt $sizes.Count; $i++) {
    $d = if ($sizes[$i] -ge 256) { 0 } else { $sizes[$i] }
    $bw.Write([byte]$d); $bw.Write([byte]$d); $bw.Write([byte]0); $bw.Write([byte]0)
    $bw.Write([UInt16]1); $bw.Write([UInt16]32)
    $bw.Write([UInt32]$images[$i].Length); $bw.Write([UInt32]$offset)
    $offset += $images[$i].Length
}
foreach ($img in $images) { $bw.Write($img) }
[System.IO.File]::WriteAllBytes("$dir\eclair.ico", $out.ToArray())
"ok"
